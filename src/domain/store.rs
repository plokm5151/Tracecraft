use crate::domain::index::FunctionSignature;
use dashmap::DashMap;
use sled::Db;
use std::sync::Arc;

/// Trait for symbol storage backends.
/// Implementations must be thread-safe (Send + Sync).
pub trait SymbolStore: Send + Sync {
    fn insert_function(&self, key: String, sig: FunctionSignature);
    fn insert_method(&self, type_name: String, method_name: String, sig: FunctionSignature);
    fn get_function(&self, key: &str) -> Option<FunctionSignature>;
    fn get_method(&self, type_name: &str, method_name: &str) -> Option<FunctionSignature>;
    fn find_methods_by_name(&self, method_name: &str) -> Vec<FunctionSignature>;
    fn register_method_lookup(&self, method_name: String, type_name: String);
}

// ============================================================================
// MemorySymbolStore - Fast in-memory storage using DashMap
// ============================================================================

pub struct MemorySymbolStore {
    pub global_functions: DashMap<String, FunctionSignature>,
    pub type_methods: DashMap<(String, String), FunctionSignature>,
    pub method_lookup: DashMap<String, Vec<String>>, // method_name -> Vec<type_name>
}

impl Default for MemorySymbolStore {
    fn default() -> Self {
        Self {
            global_functions: DashMap::new(),
            type_methods: DashMap::new(),
            method_lookup: DashMap::new(),
        }
    }
}

impl SymbolStore for MemorySymbolStore {
    fn insert_function(&self, key: String, sig: FunctionSignature) {
        self.global_functions.insert(key, sig);
    }

    fn insert_method(&self, type_name: String, method_name: String, sig: FunctionSignature) {
        self.type_methods.insert((type_name, method_name), sig);
    }

    fn get_function(&self, key: &str) -> Option<FunctionSignature> {
        self.global_functions.get(key).map(|r| r.clone())
    }

    fn get_method(&self, type_name: &str, method_name: &str) -> Option<FunctionSignature> {
        self.type_methods.get(&(type_name.to_string(), method_name.to_string())).map(|r| r.clone())
    }

    fn find_methods_by_name(&self, method_name: &str) -> Vec<FunctionSignature> {
        if let Some(type_names) = self.method_lookup.get(method_name) {
            type_names
                .iter()
                .filter_map(|tn| self.get_method(tn, method_name))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn register_method_lookup(&self, method_name: String, type_name: String) {
        self.method_lookup.entry(method_name).or_default().push(type_name);
    }
}

// ============================================================================
// DiskSymbolStore - Scalable disk-based storage using sled
// ============================================================================

pub struct DiskSymbolStore {
    db: Db,
    // Trees for different data types
    functions_tree: sled::Tree,
    methods_tree: sled::Tree,
    lookup_tree: sled::Tree,
}

impl DiskSymbolStore {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let db = sled::open(path)?;
        let functions_tree = db.open_tree("functions")?;
        let methods_tree = db.open_tree("methods")?;
        let lookup_tree = db.open_tree("method_lookup")?;
        
        Ok(Self {
            db,
            functions_tree,
            methods_tree,
            lookup_tree,
        })
    }

    fn method_key(type_name: &str, method_name: &str) -> String {
        format!("{}::{}", type_name, method_name)
    }
}

impl SymbolStore for DiskSymbolStore {
    fn insert_function(&self, key: String, sig: FunctionSignature) {
        if let Ok(bytes) = bincode::serialize(&sig) {
            let _ = self.functions_tree.insert(key.as_bytes(), bytes);
        }
    }

    fn insert_method(&self, type_name: String, method_name: String, sig: FunctionSignature) {
        let key = Self::method_key(&type_name, &method_name);
        if let Ok(bytes) = bincode::serialize(&sig) {
            let _ = self.methods_tree.insert(key.as_bytes(), bytes);
        }
    }

    fn get_function(&self, key: &str) -> Option<FunctionSignature> {
        self.functions_tree
            .get(key.as_bytes())
            .ok()
            .flatten()
            .and_then(|bytes| bincode::deserialize(&bytes).ok())
    }

    fn get_method(&self, type_name: &str, method_name: &str) -> Option<FunctionSignature> {
        let key = Self::method_key(type_name, method_name);
        self.methods_tree
            .get(key.as_bytes())
            .ok()
            .flatten()
            .and_then(|bytes| bincode::deserialize(&bytes).ok())
    }

    fn find_methods_by_name(&self, method_name: &str) -> Vec<FunctionSignature> {
        self.lookup_tree
            .get(method_name.as_bytes())
            .ok()
            .flatten()
            .and_then(|bytes| bincode::deserialize::<Vec<String>>(&bytes).ok())
            .map(|type_names| {
                type_names
                    .iter()
                    .filter_map(|tn| self.get_method(tn, method_name))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn register_method_lookup(&self, method_name: String, type_name: String) {
        // Read-modify-write pattern for the lookup list
        let mut type_names: Vec<String> = self.lookup_tree
            .get(method_name.as_bytes())
            .ok()
            .flatten()
            .and_then(|bytes| bincode::deserialize(&bytes).ok())
            .unwrap_or_default();
        
        if !type_names.contains(&type_name) {
            type_names.push(type_name);
            if let Ok(bytes) = bincode::serialize(&type_names) {
                let _ = self.lookup_tree.insert(method_name.as_bytes(), bytes);
            }
        }
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_sig(name: &str) -> FunctionSignature {
        FunctionSignature {
            name: name.to_string(),
            is_public: true,
            receiver: Some("&self".to_string()),
            location: "test.rs:1".to_string(),
            crate_name: "test_crate".to_string(),
        }
    }

    #[test]
    fn test_memory_store_functions() {
        let store = MemorySymbolStore::default();
        store.insert_function("test::foo".to_string(), sample_sig("foo"));
        
        let retrieved = store.get_function("test::foo");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "foo");
        
        assert!(store.get_function("nonexistent").is_none());
    }

    #[test]
    fn test_memory_store_methods() {
        let store = MemorySymbolStore::default();
        store.insert_method("MyType".to_string(), "bar".to_string(), sample_sig("bar"));
        store.register_method_lookup("bar".to_string(), "MyType".to_string());
        
        let retrieved = store.get_method("MyType", "bar");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "bar");
        
        let by_name = store.find_methods_by_name("bar");
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].name, "bar");
    }

    #[test]
    fn test_disk_store_functions() {
        let dir = tempdir().unwrap();
        let store = DiskSymbolStore::new(dir.path().to_str().unwrap()).unwrap();
        
        store.insert_function("test::baz".to_string(), sample_sig("baz"));
        
        let retrieved = store.get_function("test::baz");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "baz");
    }

    #[test]
    fn test_disk_store_methods() {
        let dir = tempdir().unwrap();
        let store = DiskSymbolStore::new(dir.path().to_str().unwrap()).unwrap();
        
        store.insert_method("DiskType".to_string(), "method".to_string(), sample_sig("method"));
        store.register_method_lookup("method".to_string(), "DiskType".to_string());
        
        let retrieved = store.get_method("DiskType", "method");
        assert!(retrieved.is_some());
        
        let by_name = store.find_methods_by_name("method");
        assert_eq!(by_name.len(), 1);
    }
}
