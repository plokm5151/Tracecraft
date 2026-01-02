use cargo_metadata::MetadataCommand;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

pub struct ProjectLoader;

impl ProjectLoader {
    /// Load all source files from a Cargo workspace manifest.
    /// Returns a vector of (crate_name, file_path, file_content).
    pub fn load_workspace(manifest_path: &str) -> Result<Vec<(String, String, String)>> {
        let metadata = MetadataCommand::new()
            .manifest_path(manifest_path)
            .no_deps()
            .exec()
            .context("Failed to execute cargo metadata")?;

        let mut files = Vec::new();

        for package_id in &metadata.workspace_members {
            if let Some(package) = metadata.packages.iter().find(|p| &p.id == package_id) {
                let crate_name = &package.name;
                
                // Find source root (usually src/)
                // We assume standard layout or look at targets.
                // A better way is to look at package.targets -> src_path
                for target in &package.targets {
                    // We typically care about 'lib' and 'bin' targets for source code analysis
                    if !target.kind.iter().any(|k| k == "lib" || k == "bin" || k == "proc-macro") {
                       continue; 
                    }
                    
                    let src_path = &target.src_path;
                    let src_dir = src_path.parent().unwrap_or(src_path);
                    
                    // Now recursively find all .rs files in this src_dir
                    // Note: target.src_path is an Utf8PathBuf from cargo_metadata
                    
                    Self::collect_rs_recursive(src_dir.as_std_path(), crate_name, &mut files)?;
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
