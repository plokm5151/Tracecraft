use clap::Parser;
use std::collections::HashMap;

use mr_hedgehog::infrastructure::{SimpleCallGraphBuilder, DotExporter};
use mr_hedgehog::infrastructure::project_loader::ProjectLoader;
use mr_hedgehog::infrastructure::source_manager::SourceManager;
use mr_hedgehog::infrastructure::concurrency;
use mr_hedgehog::domain::trace::TraceGenerator;
use mr_hedgehog::domain::language::Language;
use mr_hedgehog::ports::{CallGraphBuilder, OutputExporter};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// single .rs file(s)
    #[arg(short, long)]
    input: Vec<String>,

    /// folder(s) (recursively collect *.rs)
    #[arg(short='d', long)]
    folder: Vec<String>,

    /// Cargo workspace Cargo.toml
    #[arg(long)]
    workspace: Option<String>,

    /// output path
    #[arg(short, long)]
    output: String,

    /// output format (ignored for now)
    #[arg(short, long, default_value="dot")]
    format: String,

    /// 反向查詢（查詢所有能呼叫到此 function 的所有路徑，例 Type::func@crate）
    #[arg(long)]
    reverse: Option<String>,

    /// 展開 main 到所有葉節點的完整呼叫路徑
    #[arg(long)]
    expand_paths: bool,

    /// 分支 event 摘要模式（if/match 分支遇到相同 event 只記一次，不重複展開）
    #[arg(long)]
    branch_summary: bool,

    /// Enable debug output
    #[arg(long, short='D')]
    debug: bool,

    /// Expand macros using `cargo expand` before analysis
    #[arg(long)]
    expand_macros: bool,

    /// Storage backend: "mem" (default, in-memory) or "disk" (sled DB)
    #[arg(long, default_value = "mem")]
    store: String,

    /// Analysis engine: "syn" (default, AST-based) or "scip" (rust-analyzer semantic)
    #[arg(long, default_value = "syn")]
    engine: String,

    /// Programming language: "rust" (default) or "python"
    #[arg(long, default_value = "rust")]
    lang: String,
}

fn main() {
    // Initialize adaptive thread pool (reserves 50% CPU for UI/LSP)
    if let Err(e) = concurrency::init_thread_pool() {
        eprintln!("Warning: Failed to initialize thread pool: {}. Using defaults.", e);
    }

    let cli=Cli::parse();

    if cli.debug {
        println!("[DEBUG] Config: {:?}", cli);
    }

    // Branch based on engine selection
    let (callgraph, files) = match cli.engine.as_str() {
        "scip" => {
            // SCIP Engine: Use language-specific indexer for precise semantic analysis
            let language = Language::from_str(&cli.lang).unwrap_or(Language::Rust);
            println!("[Engine] Using SCIP ({} semantic analysis)", language);
            
            let workspace_path = cli.workspace.as_ref()
                .map(|ws| std::path::Path::new(ws).parent().unwrap_or(std::path::Path::new(".")))
                .unwrap_or(std::path::Path::new("."));
            
            // Generate SCIP index for the specified language
            let scip_path = match mr_hedgehog::infrastructure::scip_runner::generate_scip_index_for_language(
                workspace_path, 
                language,
                &[]
            ) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Error generating SCIP index: {}", e);
                    if language == Language::Rust {
                        eprintln!("Falling back to syn engine...");
                        return run_syn_engine(&cli);
                    } else {
                        eprintln!("No fallback available for {} (syn only supports Rust)", language);
                        std::process::exit(1);
                    }
                }
            };
            
            // Ingest SCIP and build graph
            match mr_hedgehog::domain::scip_ingest::ScipIngestor::ingest_and_build_graph(&scip_path) {
                Ok(cg) => {
                    // For SCIP engine, we still might want file contents for rich traces
                    let loaded_files = if let Some(ws) = &cli.workspace {
                        ProjectLoader::load_workspace(ws, cli.expand_macros).unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    (cg, loaded_files)
                }
                Err(e) => {
                    eprintln!("Error ingesting SCIP index: {}", e);
                    eprintln!("Falling back to syn engine...");
                    return run_syn_engine(&cli);
                }
            }
        }
        _ => {
            // Syn Engine: Traditional AST-based analysis
            println!("[Engine] Using syn (AST-based analysis)");
            run_syn_engine_internal(&cli)
        }
    };

    run_post_processing(&cli, &callgraph, &files);
}

/// Run the syn-based analysis engine (internal, returns CallGraph and Files)
fn run_syn_engine_internal(cli: &Cli) -> (mr_hedgehog::domain::callgraph::CallGraph, Vec<(String, String, String)>) {
    let mut files = Vec::<(String,String,String)>::new();

    // workspace (primary method)
    if let Some(ws) = &cli.workspace {
        match ProjectLoader::load_workspace(ws, cli.expand_macros) {
            Ok(loaded_files) => {
                println!("Loaded {} files from workspace", loaded_files.len());
                files.extend(loaded_files);
            },
            Err(e) => panic!("Failed to load workspace: {:?}", e),
        }
    } else {
        if !cli.input.is_empty() || !cli.folder.is_empty() {
             panic!("Legacy input/folder mode is momentarily disabled during refactor. Please use --workspace.");
        }
    }

    if files.is_empty() { panic!("No input provided"); }

    // Initialize storage backend
    let store: std::sync::Arc<dyn mr_hedgehog::domain::store::SymbolStore> = match cli.store.as_str() {
        "disk" => {
            let db_path = "mr_hedgehog_db";
            std::sync::Arc::new(mr_hedgehog::domain::store::DiskSymbolStore::new(db_path).expect("Failed to open disk store"))
        }
        _ => std::sync::Arc::new(mr_hedgehog::domain::store::MemorySymbolStore::default()),
    };

    println!("Using storage backend: {}", cli.store);

    let cg_builder = SimpleCallGraphBuilder::new_with_store(store);
    (cg_builder.build_call_graph(&files), files)
}

/// Run syn engine (wrapper for fallback)
fn run_syn_engine(cli: &Cli) {
    let (callgraph, files) = run_syn_engine_internal(cli);
    run_post_processing(cli, &callgraph, &files);
}

/// Common post-processing: reverse queries, trace expansion, DOT export
fn run_post_processing(cli: &Cli, callgraph: &mr_hedgehog::domain::callgraph::CallGraph, files: &[(String, String, String)]) {

    // for quick lookup
    let mut map=HashMap::new(); 
    for n in &callgraph.nodes {
        map.insert(n.id.clone(), n);
    }
    
    let entry=callgraph.nodes.iter()
        .find(|n| n.id.starts_with("main@") || n.id.contains("::main"))
        .map(|n| n.id.clone())
        .unwrap_or_else(|| {
            eprintln!("WARN: no main() found in call graph");
            "".into()
        });

    // ── reverse call查詢 ──────────────────────
    if let Some(ref target_id) = cli.reverse {
        println!("=== Reverse call tracing: {} ===", target_id);
        // 1. 構建 caller_map: callee_id → Vec<caller_id>
        let mut caller_map: HashMap<String, Vec<String>> = HashMap::new();
        for node in &callgraph.nodes {
            for callee in &node.callees {
                caller_map.entry(callee.clone()).or_default().push(node.id.clone());
            }
        }

        // 2. BFS/DFS 搜尋所有從 main@... 到 target_id 的完整呼叫路徑
        let mut all_paths: Vec<Vec<String>> = vec![];
        let mut stack = vec![(vec![entry.clone()], entry.clone())]; // (目前路徑, 當前節點)

        while let Some((path, node_id)) = stack.pop() {
            if node_id == *target_id {
                all_paths.push(path.clone());
                continue;
            }
            // 找 callee
            if let Some(n) = map.get(&node_id) {
                for callee in &n.callees {
                    if !path.contains(callee) { // 防止循環
                        let mut new_path = path.clone();
                        new_path.push(callee.clone());
                        stack.push((new_path, callee.clone()));
                    }
                }
            }
        }
        if all_paths.is_empty() {
            println!("找不到任何路徑從 main 到 {}", target_id);
        } else {
            for (i, path) in all_paths.iter().enumerate() {
                println!("路徑 {}:", i+1);
                for seg in path {
                    println!("  {}", seg);
                }
            }
        }
        return;
    }

    // ── 3. trace from main ──────────────────
    if cli.debug {
        println!("\n==== [DEBUG nodes] ====");
        for n in &callgraph.nodes{println!("{} -> {:?}",n.id,n.callees);}
        println!("========================");
    }

    if !entry.is_empty() && cli.expand_paths {
        // Init SourceManager
        let source_manager = SourceManager::new(&files);

        println!("\n=== Rich Trace Paths from {} ===", entry);
        let trace_gen = TraceGenerator::new(&callgraph, &source_manager);
        let paths = trace_gen.generate_paths(&entry);

        if paths.is_empty() {
             println!("No paths found.");
        }

        for (i, path) in paths.iter().enumerate() {
            println!("Path {}:", i + 1);
            for (step_idx, step) in path.steps.iter().enumerate() {
                let location = step.location.as_deref().unwrap_or("?");
                let note = step.note.as_deref().unwrap_or("");
                let note_str = if !note.is_empty() { format!(" {}", note) } else { "".to_string() };
                
                // Indentation based on depth (step.depth or just loop index? 
                // trace.rs sets depth. Let's use it.)
                let indent = "  ".repeat(step.depth);
                
                println!("{}[{}] {}{} ({})", indent, step_idx, step.id, note_str, location);
                
                if let Some(code) = &step.snippet {
                    println!("{}    Code: {}", indent, code);
                }
            }
            println!();
        }
    }

    // ── 4. export dot ────────────────────────
    let exporter=DotExporter{};
    exporter.export(&callgraph,&cli.output).unwrap();
    println!("Graph saved to {}",cli.output);
}

