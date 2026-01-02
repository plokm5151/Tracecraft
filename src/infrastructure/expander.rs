use anyhow::{Context, Result};
use std::process::Command;

pub fn expand_crate(manifest_path: &str) -> Result<String> {
    // Check if cargo-expand is installed (optional check, but good for UX)
    // Actually, simply trying to run it and handling the error is fairly robust.
    
    // Command: cargo expand --manifest-path <manifest_path>
    // Note: 'expand' is a subcommand.
    let output = Command::new("cargo")
        .arg("expand")
        .arg("--manifest-path")
        .arg(manifest_path)
        .output()
        .context("Failed to execute 'cargo expand'. Is cargo-expand installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cargo expand failed: {}", stderr);
    }

    let content = String::from_utf8(output.stdout)
        .context("cargo expand output was not valid UTF-8")?;

    Ok(content)
}
