use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

pub struct ProjectLoader;

impl ProjectLoader {
    /// Load all source files from a Cargo workspace manifest.
    /// Returns a vector of (crate_name, file_path, file_content).
    pub fn load_workspace(manifest_path: &str, expand_macros: bool) -> Result<Vec<(String, String, String)>> {
        let metadata = MetadataCommand::new()
            .manifest_path(manifest_path)
            .no_deps()
            .exec()
            .context("Failed to execute cargo metadata")?;

        let mut files = Vec::new();

        for package in metadata.workspace_packages() {
            let crate_name = &package.name;
            
            // Skip if no targets or irrelevant (though workspace_packages usually are relevant)
             for target in &package.targets {
                if target.kind.iter().any(|k| k == "lib" || k == "bin" || k == "proc-macro") {
                    // Logic Branch: Expand Macros vs Raw Files
                    if expand_macros {
                         // SINGLE file per target/crate? 
                         // cargo expand works per crate (or specific target). 
                         // Simple usage: cargo expand --manifest-path package/Cargo.toml
                         // But we are at workspace level. We might need package manifest.
                         let package_manifest = &package.manifest_path;
                         
                         // Note: running cargo expand for EACH package.
                         // Optimization: cargo expand runs the whole crate.
                         // We probably only want to run it once per package.
                         
                         // We are inside a loop over TARGETS. We should loop over PACKAGES.
                         // The outer loop IS package.
                         // Just break after doing it once per package? Or careful with multiple targets?
                         // cargo expand usually expands the library by default or bin if specified.
                         // For simplicity, let's try to expand the package.
                         
                         // Check if we already processed this package (simple dedup if targets iterate same package multiple times - wait, workspace_packages() returns unique packages)
                         // But we are iterating targets inside.
                         
                         // Let's perform expansion on the *first* interesting target and skip others for this package?
                         // Or better: move expansion logic outside target loop.
                    } else {
                        let src_path = &target.src_path;
                        let src_dir = src_path.parent().unwrap_or(src_path);
                        Self::collect_rs_recursive(src_dir.as_std_path(), crate_name, &mut files)?;
                    }
                }
            }
            
            // Handling Expansion Outside Target Loop to avoid duplicates
            if expand_macros {
                // We attempt to expand the whole package
                match crate::infrastructure::expander::expand_crate(package.manifest_path.as_str()) {
                    Ok(expanded_code) => {
                         // We treat the expanded result as a single "virtual" file for this crate.
                         files.push((crate_name.clone(), format!("<expanded:{}>", crate_name), expanded_code));
                    },
                    Err(e) => {
                        eprintln!("WARN: Failed to expand crate {}: {}", crate_name, e);
                        // Fallback? Or just warn? Warn is safer.
                    }
                }
            }
        }
        
        // Dedup files if multiple targets point to same files (unlikely main.rs/lib.rs overlap, but robust)
        files.sort_by(|a, b| a.1.cmp(&b.1));
        files.dedup_by(|a, b| a.1 == b.1);

        Ok(files)
    }

    fn collect_rs_recursive(
        dir: &Path, 
        crate_name: &str, 
        out: &mut Vec<(String, String, String)>
    ) -> Result<()> {
        if dir.ends_with("target") || dir.ends_with(".git") {
            return Ok(());
        }
        if !dir.exists() {
             return Ok(());
        }

        if dir.is_file() {
            // It might be a single file target (like main.rs)
             if let Some(ext) = dir.extension() {
                if ext == "rs" {
                     let content = fs::read_to_string(dir)
                        .with_context(|| format!("Failed to read file {}", dir.display()))?;
                    out.push((crate_name.to_string(), dir.display().to_string(), content));
                }
             }
             return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                Self::collect_rs_recursive(&path, crate_name, out)?;
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    let content = fs::read_to_string(&path)
                        .with_context(|| format!("Failed to read file {}", path.display()))?;
                    out.push((crate_name.to_string(), path.display().to_string(), content));
                }
            }
        }
        Ok(())
    }
}
