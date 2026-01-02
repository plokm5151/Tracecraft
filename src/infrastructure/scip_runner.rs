/// SCIP Index Generator Runner.
/// 
/// Multi-language support for generating SCIP indices:
/// - Rust: Uses `rust-analyzer scip`
/// - Python: Uses `scip-python`
/// 
/// Phase 3.2: Caching integration for incremental regeneration.
/// Phase 3 v2: Multi-language support (Rust + Python).

use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result, bail};
use super::scip_cache::ScipCache;
use crate::domain::language::Language;

// ═══════════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════════

/// Generate a SCIP index for the given workspace (defaults to Rust).
pub fn generate_scip_index(workspace_root: &Path) -> Result<PathBuf> {
    generate_scip_index_for_language(workspace_root, Language::Rust, &[])
}

/// Generate a SCIP index for the given language with source file tracking.
pub fn generate_scip_index_for_language(
    workspace_root: &Path,
    language: Language,
    source_files: &[String],
) -> Result<PathBuf> {
    let cache = ScipCache::new(workspace_root);

    // Check if cache is valid
    if let Some(cached_path) = cache.get_valid_cache() {
        return Ok(cached_path);
    }

    // Cache miss - need to regenerate
    generate_fresh_index(workspace_root, language, &cache, source_files)
}

/// Force regeneration of the SCIP index, ignoring cache.
pub fn generate_fresh_index(
    workspace_root: &Path,
    language: Language,
    cache: &ScipCache,
    source_files: &[String],
) -> Result<PathBuf> {
    // Check if the language-specific indexer is available
    check_indexer_available(language)?;

    // Generate the index
    let output_file = cache.index_path().to_path_buf();
    
    println!("[SCIP] Generating {} index for: {}", language, workspace_root.display());
    
    let status = run_indexer_command(workspace_root, language, &output_file)?;

    if !status.success() {
        bail!("{} SCIP indexer failed with exit code: {:?}", 
              language.scip_command(), status.code());
    }

    if !output_file.exists() {
        bail!("Expected index.scip was not created at: {}", output_file.display());
    }

    // Update cache metadata
    if !source_files.is_empty() {
        if let Err(e) = cache.update_metadata(source_files) {
            eprintln!("[SCIP Cache] Warning: Failed to update metadata: {}", e);
        }
    }

    println!("[SCIP] Generated index: {}", output_file.display());
    Ok(output_file)
}

// ═══════════════════════════════════════════════════════════════════════════
// Internal Implementation
// ═══════════════════════════════════════════════════════════════════════════

/// Check if the language-specific SCIP indexer is available.
fn check_indexer_available(language: Language) -> Result<()> {
    let command = language.scip_command();
    let version_arg = match language {
        Language::Rust => "--version",
        Language::Python => "--version",
    };
    
    let check = Command::new(command)
        .arg(version_arg)
        .output();
    
    match check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("[SCIP] Using {}: {}", command, version.trim());
            Ok(())
        }
        Ok(output) => {
            bail!("{} found but returned error: {:?}", command, output.status.code());
        }
        Err(_) => {
            bail!(
                "{} not found in PATH. {}", 
                command, 
                language.install_instructions()
            );
        }
    }
}

/// Run the language-specific indexer command.
fn run_indexer_command(
    workspace_root: &Path,
    language: Language,
    output_file: &Path,
) -> Result<std::process::ExitStatus> {
    let output_filename = output_file.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid output path"))?;
    
    match language {
        Language::Rust => {
            Command::new("rust-analyzer")
                .arg("scip")
                .arg(".")
                .arg("--output")
                .arg(output_filename)
                .current_dir(workspace_root)
                .status()
                .context("Failed to execute rust-analyzer scip")
        }
        Language::Python => {
            Command::new("scip-python")
                .arg("index")
                .arg(".")
                .arg("--output")
                .arg(output_filename)
                .current_dir(workspace_root)
                .status()
                .context("Failed to execute scip-python index")
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Testable Command Builder (for unit tests)
// ═══════════════════════════════════════════════════════════════════════════

/// Describes the command that would be run for a given language.
/// This is primarily for testing without actually executing commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScipCommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

/// Build the command specification for a given language (testable function).
pub fn build_command_spec(language: Language) -> ScipCommandSpec {
    match language {
        Language::Rust => ScipCommandSpec {
            program: "rust-analyzer".to_string(),
            args: vec!["scip".to_string(), ".".to_string(), "--output".to_string(), "index.scip".to_string()],
        },
        Language::Python => ScipCommandSpec {
            program: "scip-python".to_string(),
            args: vec!["index".to_string(), ".".to_string(), "--output".to_string(), "index.scip".to_string()],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_command_spec_rust() {
        let spec = build_command_spec(Language::Rust);
        assert_eq!(spec.program, "rust-analyzer");
        assert!(spec.args.contains(&"scip".to_string()));
        assert!(spec.args.contains(&"--output".to_string()));
    }

    #[test]
    fn test_build_command_spec_python() {
        let spec = build_command_spec(Language::Python);
        assert_eq!(spec.program, "scip-python");
        assert!(spec.args.contains(&"index".to_string()));
        assert!(spec.args.contains(&"--output".to_string()));
    }

    #[test]
    fn test_command_differences() {
        let rust_spec = build_command_spec(Language::Rust);
        let python_spec = build_command_spec(Language::Python);
        
        assert_ne!(rust_spec.program, python_spec.program);
        assert_ne!(rust_spec.args[0], python_spec.args[0]); // "scip" vs "index"
    }

    #[test]
    #[ignore] // Requires rust-analyzer to be installed
    fn test_generate_scip_index() {
        use std::env;
        let workspace = env::current_dir().unwrap();
        let result = generate_scip_index(&workspace);
        if result.is_ok() {
            let path = result.unwrap();
            assert!(path.exists());
        }
    }
}
