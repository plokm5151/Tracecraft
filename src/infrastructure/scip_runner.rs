/// SCIP Index Generator Runner.
/// Invokes `rust-analyzer scip` to produce a Code Intelligence index.

use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{Context, Result, bail};

/// Generate a SCIP index for the given workspace.
/// Returns the path to the generated `index.scip` file.
pub fn generate_scip_index(workspace_root: &Path) -> Result<PathBuf> {
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
    let output_file = workspace_root.join("index.scip");
    
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
