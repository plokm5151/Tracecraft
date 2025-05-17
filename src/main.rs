// Command-line entry point for TraceCraft.

use clap::Parser;
use tracecraft::application::AnalyzeUsecase;
use tracecraft::infrastructure::{SynAstParser, SimpleCallGraphBuilder, DotExporter};
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, required = false)]
    input: Vec<String>,

    #[arg(short = 'd', long, required = false)]
    folder: Vec<String>,

    #[arg(long, required = false)]
    workspace: Option<String>,

    #[arg(short, long)]
    output: String,

    #[arg(short, long, default_value = "dot")]
    format: String,
}

fn read_all_rs_files(dir: &str) -> Vec<(String, String)> {
    let mut files = vec![];
    fn visit_dir(dir: &Path, files: &mut Vec<(String, String)>) {
        if dir.ends_with("target") || dir.ends_with(".git") { return; }
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit_dir(&path, files);
                } else if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        if let Ok(src) = fs::read_to_string(&path) {
                            files.push((path.to_string_lossy().to_string(), src));
                        }
                    }
                }
            }
        }
    }
    visit_dir(Path::new(dir), &mut files);
    files
}

fn main() {
    let cli = Cli::parse();

    // 收集所有來源
    let mut sources: Vec<(String, String)> = vec![];
    for input_file in &cli.input {
        if let Ok(code) = fs::read_to_string(input_file) {
            sources.push((input_file.clone(), code));
        }
    }
    for folder in &cli.folder {
        sources.extend(read_all_rs_files(folder));
    }
    // workspace (略，同現有設計)

    if sources.is_empty() {
        panic!("Please provide at least one --input <file> or --folder <dir> or --workspace <Cargo.toml>");
    }

    // 逐檔案建立 call graph
    let usecase = AnalyzeUsecase {
        parser: &SynAstParser,
        callgraph_builder: &SimpleCallGraphBuilder,
        exporter: &DotExporter,
    };

    let mut file_callgraphs = vec![];
    for (filename, src) in &sources {
        let cg = usecase.callgraph_builder.build_call_graph(src);
        file_callgraphs.push((filename.clone(), cg));
    }

    // 合併所有 call graph 節點與邊
    let mut all_nodes = vec![];
    let mut all_edges = vec![];
    for (filename, cg) in &file_callgraphs {
        for node in &cg.nodes {
            all_nodes.push(format!("{} [{}]", node.id, filename));
            for callee in &node.callees {
                all_edges.push(format!("{} -> {} [{}]", node.id, callee, filename));
            }
        }
    }

    // 輸出 DOT
    let mut dot_lines = vec!["digraph G {".to_string()];
    for n in &all_nodes { dot_lines.push(format!("    {};", n)); }
    for e in &all_edges { dot_lines.push(format!("    {};", e)); }
    dot_lines.push("}".to_string());
    fs::write(&cli.output, dot_lines.join("\n")).unwrap();

    println!(
        "Analysis completed! Output written to {} (format: {})",
        cli.output, cli.format
    );
}
