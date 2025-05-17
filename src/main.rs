// Command-line entry point for TraceCraft.

use clap::Parser;
use tracecraft::application::AnalyzeUsecase;
use tracecraft::infrastructure::{SynAstParser, SimpleCallGraphBuilder, DotExporter};
use std::fs;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Input source file path (can specify multiple)
    #[arg(short, long, required = false)]
    input: Vec<String>,

    /// Input source folder(s)
    #[arg(short = 'd', long, required = false)]
    folder: Vec<String>,

    /// Workspace Cargo.toml
    #[arg(long, required = false)]
    workspace: Option<String>,

    /// Output file path
    #[arg(short, long)]
    output: String,

    /// Output format (dot, json, text)
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

// 讀取 Cargo.toml，收集所有 member crate 的 src/*.rs
fn collect_rs_from_workspace(cargo_toml: &str) -> Vec<(String, String)> {
    let toml_content = fs::read_to_string(cargo_toml).expect("Cannot read workspace Cargo.toml");
    let parsed: toml::Value = toml::from_str(&toml_content).expect("Invalid toml");
    let root = Path::new(cargo_toml).parent().unwrap();
    let members = parsed["workspace"]["members"].as_array().unwrap();
    let mut files = vec![];
    for m in members {
        let member_dir = root.join(m.as_str().unwrap());
        let src_dir = member_dir.join("src");
        if src_dir.exists() {
            files.extend(read_all_rs_files(&src_dir.to_string_lossy()));
        }
    }
    files
}

fn main() {
    let cli = Cli::parse();

    let mut all_sources: Vec<(String, String)> = vec![];

    // 1. input files
    for input_file in &cli.input {
        if let Ok(code) = fs::read_to_string(input_file) {
            all_sources.push((input_file.clone(), code));
        } else {
            eprintln!("[WARN] Cannot read input file: {}", input_file);
        }
    }

    // 2. folders
    for folder in &cli.folder {
        all_sources.extend(read_all_rs_files(folder));
    }

    // 3. workspace
    if let Some(cargo_toml) = &cli.workspace {
        let ws_sources = collect_rs_from_workspace(cargo_toml);
        println!("[DEBUG] workspace collected {} .rs files", ws_sources.len());
        for (i, (filename, code)) in ws_sources.iter().enumerate() {
            println!("[DEBUG] File {}: {} ({} bytes)", i, filename, code.len());
        }
        all_sources.extend(ws_sources);
    }

    if all_sources.is_empty() {
        panic!("Please provide at least one --input <file> or --folder <dir> or --workspace <Cargo.toml>");
    }

    // 全部連成一個字串，交給 usecase（之後可優化為多 crate 分開處理）
    let src_code: String = all_sources.iter().map(|(_, code)| code.as_str()).collect::<String>();

    let usecase = AnalyzeUsecase {
        parser: &SynAstParser,
        callgraph_builder: &SimpleCallGraphBuilder,
        exporter: &DotExporter,
    };

    let result = usecase.run(&src_code, &cli.output);

    match result {
        Ok(_) => println!(
            "Analysis completed! Output written to {} (format: {})",
            cli.output, cli.format
        ),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}
