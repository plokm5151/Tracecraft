/// SCIP Index Generator Runner.
/// Invokes `rust-analyzer scip` to produce a Code Intelligence index.
/// 
/// Phase 3.2: Integrated with caching for incremental regeneration.

use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result, bail};
use super::scip_cache::ScipCache;

/// Generate a SCIP index for the given workspace.
/// 
/// If a valid cache exists (source files unchanged), returns the cached index.
/// Otherwise, invokes `rust-analyzer scip` to generate a fresh index.
/// 
/// Returns the path to the `index.scip` file.
pub fn generate_scip_index(workspace_root: &Path) -> Result<PathBuf> {
    generate_scip_index_with_sources(workspace_root, &[])
}

/// Generate a SCIP index with explicit source file tracking for cache.
pub fn generate_scip_index_with_sources(
    workspace_root: &Path,
    source_files: &[String],
) -> Result<PathBuf> {
    let cache = ScipCache::new(workspace_root);

    // Check if cache is valid
    if let Some(cached_path) = cache.get_valid_cache() {
        return Ok(cached_path);
    }

    // Cache miss - need to regenerate
    generate_fresh_index(workspace_root, &cache, source_files)
}

/// Force regeneration of the SCIP index, ignoring cache.
pub fn generate_fresh_index(
    workspace_root: &Path,
    cache: &ScipCache,
    source_files: &[String],
) -> Result<PathBuf> {
    // Check if rust-analyzer is available
    let ra_check = Command::new("rust-analyzer")
        .arg("--version")
        .output();
    
    if let Ok(output) = ra_check {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("[SCIP] Using rust-analyzer: {}", version.trim());
        } else {
             bail!("rust-analyzer found but returned error: {:?}", output.status.code());
        }
    } else {
        bail!(
            "rust-analyzer not found in PATH. Please install it: \
             https://rust-analyzer.github.io/manual.html#installation"
        );
    }

    // Run rust-analyzer scip command
    let output_file = cache.index_path().to_path_buf();
    
    println!("[SCIP] Generating index for: {}", workspace_root.display());
    
    let status = Command::new("rust-analyzer")
        .arg("scip")
        .arg(".")
        .arg("--output")
        .arg(output_file.file_name().unwrap())
        .current_dir(workspace_root)
        .status()
        .context("Failed to execute rust-analyzer scip")?;

    if !status.success() {
        bail!("rust-analyzer scip failed with exit code: {:?}", status.code());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    #[ignore] // Requires rust-analyzer to be installed
    fn test_generate_scip_index() {
        // This test requires a valid Cargo workspace
        let workspace = env::current_dir().unwrap();
        let result = generate_scip_index(&workspace);
        // If rust-analyzer is not installed, this will fail - that's expected
        if result.is_ok() {
            let path = result.unwrap();
            assert!(path.exists());
        }
    }
}
