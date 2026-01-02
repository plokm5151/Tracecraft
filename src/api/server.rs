use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use crate::infrastructure::scip_runner;
use crate::domain::language::Language;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct CommandReq {
    command: String,
    params: Option<serde_json::Value>,
}

pub fn start_server(port: u16) -> Result<()> {
    let address = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&address)
        .with_context(|| format!("Failed to bind to {}", address))?;

    println!("[Mr. Hedgehog] API Server listening on {}", address);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    if let Err(e) = handle_connection(stream) {
                        eprintln!("[API] Connection error: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("[API] Accept error: {}", e),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    // Clone stream for reading/writing
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            break; // Connection closed
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = match process_command(trimmed) {
            Ok(data) => json!({
                "status": "success",
                "data": data
            }),
            Err(e) => json!({
                "status": "error",
                "message": e.to_string()
            }),
        };

        let response_str = serde_json::to_string(&response)?;
        stream.write_all(response_str.as_bytes())?;
        stream.write_all(b"\n")?;
        
        // Handle SHUTDOWN specially to break loop? 
        // Or client closes connection.
        // Actually command logic might want to terminate the process.
        if let Ok(req) = serde_json::from_str::<CommandReq>(trimmed) {
             if req.command == "SHUTDOWN" {
                 println!("[API] Shutdown requested.");
                 std::process::exit(0);
             }
        }
    }
    Ok(())
}

fn process_command(json_str: &str) -> Result<serde_json::Value> {
    let req: CommandReq = serde_json::from_str(json_str)
        .context("Invalid JSON format")?;

    match req.command.as_str() {
        "PING" => Ok(json!("PONG")),
        "ANALYZE" => handle_analyze(req.params),
        "SHUTDOWN" => Ok(json!("Shutting down...")),
        _ => anyhow::bail!("Unknown command: {}", req.command),
    }
}

fn handle_analyze(params: Option<serde_json::Value>) -> Result<serde_json::Value> {
    let params = params.ok_or_else(|| anyhow::anyhow!("Missing params for ANALYZE"))?;
    
    let path_str = params.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing 'path' param"))?;

    let engine_str = params.get("engine")
        .and_then(|v| v.as_str())
        .unwrap_or("scip");

    // We only support SCIP for the persistent backend for now as it's cleaner
    if engine_str != "scip" {
        anyhow::bail!("Only 'scip' engine is supported in daemon mode");
    }

    let workspace_path = PathBuf::from(path_str);
    if !workspace_path.exists() {
         anyhow::bail!("Workspace path not found: {}", path_str);
    }
    
    println!("[API] Analyzing: {}", path_str);
    
    // 1. Generate SCIP index
    // Assume Rust for now, or infer from params
    let lang_str = params.get("lang").and_then(|v| v.as_str()).unwrap_or("rust");
    let lang = match lang_str {
        "python" => Language::Python,
        _ => Language::Rust,
    };
    
    let index_path = scip_runner::generate_scip_index_for_language(
        &workspace_path, 
        lang,
        &[] // No specific file filtering yet
    )?;

    // 2. Ingest
    let callgraph = crate::domain::scip_ingest::ScipIngestor::ingest_and_build_graph(&index_path)
        .context("Failed to ingest SCIP index")?;

    // 3. Convert to DTO
    let graph_dto = crate::api::dto::GraphDto::from(callgraph);
    
    Ok(serde_json::to_value(graph_dto)?)
}
