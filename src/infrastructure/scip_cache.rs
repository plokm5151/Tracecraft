/// SCIP Cache Module
/// 
/// Provides incremental indexing by caching SCIP indices and validating
/// them against source file modifications.
/// 
/// Cache structure:
/// - `index.scip` - The SCIP protobuf index
/// - `index.scip.meta` - JSON metadata for cache validation

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Cache metadata stored alongside the SCIP index.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScipCacheMetadata {
    /// Version of the cache format (for future compatibility)
    pub version: u32,
    /// Timestamp of when the cache was created
    pub created_at: u64,
    /// Map of source file path -> modification timestamp (unix seconds)
    pub source_files: HashMap<String, u64>,
    /// Hash of Cargo.lock (if present)
    pub cargo_lock_hash: Option<String>,
}

impl ScipCacheMetadata {
    pub const CURRENT_VERSION: u32 = 1;
}

/// SCIP Cache Manager
pub struct ScipCache {
    workspace_root: PathBuf,
    index_path: PathBuf,
    meta_path: PathBuf,
}

impl ScipCache {
    /// Create a new cache manager for the given workspace.
    pub fn new(workspace_root: &Path) -> Self {
        let index_path = workspace_root.join("index.scip");
        let meta_path = workspace_root.join("index.scip.meta");
        
        Self {
            workspace_root: workspace_root.to_path_buf(),
            index_path,
            meta_path,
        }
    }

    /// Check if a valid cache exists and is up-to-date.
    /// Returns the path to the cached index if valid.
    pub fn get_valid_cache(&self) -> Option<PathBuf> {
        // Check if both index and metadata exist
        if !self.index_path.exists() || !self.meta_path.exists() {
            println!("[SCIP Cache] No cache found");
            return None;
        }

        // Load and validate metadata
        let meta = match self.load_metadata() {
            Ok(m) => m,
            Err(e) => {
                println!("[SCIP Cache] Failed to load metadata: {}", e);
                return None;
            }
        };

        // Check version
        if meta.version != ScipCacheMetadata::CURRENT_VERSION {
            println!("[SCIP Cache] Cache version mismatch");
            return None;
        }

        // Validate source files haven't changed
        if !self.validate_source_files(&meta) {
            println!("[SCIP Cache] Source files have changed");
            return None;
        }

        // Validate Cargo.lock hasn't changed
        if !self.validate_cargo_lock(&meta) {
            println!("[SCIP Cache] Cargo.lock has changed");
            return None;
        }

        println!("[SCIP Cache] Cache is valid, skipping regeneration");
        Some(self.index_path.clone())
    }

    /// Update the cache metadata after generating a new index.
    pub fn update_metadata(&self, source_files: &[String]) -> Result<()> {
        let mut file_times = HashMap::new();
        
        for file_path in source_files {
            if let Ok(mtime) = Self::get_file_mtime(file_path) {
                file_times.insert(file_path.clone(), mtime);
            }
        }

        let cargo_lock_hash = self.compute_cargo_lock_hash();

        let meta = ScipCacheMetadata {
            version: ScipCacheMetadata::CURRENT_VERSION,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_files: file_times,
            cargo_lock_hash,
        };

        let json = serde_json::to_string_pretty(&meta)
            .context("Failed to serialize cache metadata")?;
        
        let mut file = File::create(&self.meta_path)
            .context("Failed to create cache metadata file")?;
        file.write_all(json.as_bytes())
            .context("Failed to write cache metadata")?;

        println!("[SCIP Cache] Metadata updated with {} source files", meta.source_files.len());
        Ok(())
    }

    /// Clear the cache.
    pub fn invalidate(&self) -> Result<()> {
        if self.index_path.exists() {
            fs::remove_file(&self.index_path)?;
        }
        if self.meta_path.exists() {
            fs::remove_file(&self.meta_path)?;
        }
        Ok(())
    }

    /// Get the expected index path.
    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    // ─────────────────────────────────────────────────────────────────────
    // Private helpers
    // ─────────────────────────────────────────────────────────────────────

    fn load_metadata(&self) -> Result<ScipCacheMetadata> {
        let mut file = File::open(&self.meta_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let meta: ScipCacheMetadata = serde_json::from_str(&contents)?;
        Ok(meta)
    }

    fn validate_source_files(&self, meta: &ScipCacheMetadata) -> bool {
        for (path, cached_mtime) in &meta.source_files {
            match Self::get_file_mtime(path) {
                Ok(current_mtime) => {
                    if current_mtime != *cached_mtime {
                        return false;
                    }
                }
                Err(_) => {
                    // File no longer exists or can't be read
                    return false;
                }
            }
        }
        true
    }

    fn validate_cargo_lock(&self, meta: &ScipCacheMetadata) -> bool {
        let current_hash = self.compute_cargo_lock_hash();
        meta.cargo_lock_hash == current_hash
    }

    fn compute_cargo_lock_hash(&self) -> Option<String> {
        let lock_path = self.workspace_root.join("Cargo.lock");
        if !lock_path.exists() {
            return None;
        }

        match fs::read(&lock_path) {
            Ok(contents) => {
                // Simple hash using first/last bytes + length (fast approximation)
                let len = contents.len();
                let first = contents.first().copied().unwrap_or(0);
                let last = contents.last().copied().unwrap_or(0);
                Some(format!("{:02x}{:02x}{:08x}", first, last, len))
            }
            Err(_) => None,
        }
    }

    fn get_file_mtime(path: &str) -> Result<u64> {
        let metadata = fs::metadata(path)?;
        let mtime = metadata.modified()?;
        let duration = mtime.duration_since(SystemTime::UNIX_EPOCH)?;
        Ok(duration.as_secs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cache_miss_when_no_files() {
        let dir = tempdir().unwrap();
        let cache = ScipCache::new(dir.path());
        
        assert!(cache.get_valid_cache().is_none());
    }

    #[test]
    fn test_cache_hit_after_metadata_update() {
        let dir = tempdir().unwrap();
        let cache = ScipCache::new(dir.path());
        
        // Create a fake source file
        let src_file = dir.path().join("test.rs");
        fs::write(&src_file, "fn main() {}").unwrap();
        
        // Create a fake SCIP index
        fs::write(cache.index_path(), b"fake scip data").unwrap();
        
        // Update metadata
        let source_files = vec![src_file.to_string_lossy().to_string()];
        cache.update_metadata(&source_files).unwrap();
        
        // Cache should be valid
        assert!(cache.get_valid_cache().is_some());
    }

    #[test]
    fn test_cache_invalidation_on_source_change() {
        let dir = tempdir().unwrap();
        let cache = ScipCache::new(dir.path());
        
        // Create source file and fake index
        let src_file = dir.path().join("test.rs");
        fs::write(&src_file, "fn main() {}").unwrap();
        fs::write(cache.index_path(), b"fake scip data").unwrap();
        
        // Create metadata with an OLD timestamp (simulating stale cache)
        let source_files = vec![src_file.to_string_lossy().to_string()];
        
        // Manually create stale metadata with mtime = 0 (very old)
        let stale_meta = ScipCacheMetadata {
            version: ScipCacheMetadata::CURRENT_VERSION,
            created_at: 0,
            source_files: {
                let mut m = HashMap::new();
                m.insert(source_files[0].clone(), 0u64); // Fake old timestamp
                m
            },
            cargo_lock_hash: None,
        };
        
        let json = serde_json::to_string_pretty(&stale_meta).unwrap();
        fs::write(&cache.meta_path, json).unwrap();
        
        // Cache should be invalid because file mtime != 0
        assert!(cache.get_valid_cache().is_none());
    }

    #[test]
    fn test_explicit_invalidation() {
        let dir = tempdir().unwrap();
        let cache = ScipCache::new(dir.path());
        
        // Create files
        fs::write(cache.index_path(), b"data").unwrap();
        fs::write(&cache.meta_path, b"{}").unwrap();
        
        // Invalidate
        cache.invalidate().unwrap();
        
        // Files should be gone
        assert!(!cache.index_path.exists());
        assert!(!cache.meta_path.exists());
    }
}
