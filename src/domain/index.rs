use dashmap::DashMap;
use rayon::prelude::*;
use syn::{Item, Type, Visibility};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub is_public: bool,
    pub receiver: Option<String>, // "&self", "self", or None for static
    pub location: String,         // file:line
    pub crate_name: String,
}

/// Thread-safe symbol index using DashMap for concurrent access.
/// Enables parallel parsing and indexing of source files.
pub struct SymbolIndex {
    // Key: crate::func
    pub global_functions: DashMap<String, FunctionSignature>,
    
    // Key: (TypeName, MethodName)
    pub type_methods: DashMap<(String, String), FunctionSignature>,

    // Acceleration map: MethodName -> Vec<(TypeName, MethodName)>
    pub method_lookup: DashMap<String, Vec<(String, String)>>,
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self {
            global_functions: DashMap::new(),
            type_methods: DashMap::new(),
            method_lookup: DashMap::new(),
        }
    }
}

impl SymbolIndex {
    /// Build the symbol index from source files in parallel.
    pub fn build(sources: &[(String, String, String)]) -> Self {
        let index = SymbolIndex::default();

        // Parallel iteration over all source files
        sources.par_iter().for_each(|(crate_name, file_path, code)| {
            match syn::parse_file(code) {
                Ok(ast) => {
                    index.index_items(crate_name, file_path, &ast.items);
                }
                Err(e) => {
                    eprintln!("WARN: Failed to parse {}: {}", file_path, e);
                }
            }
        });

        index
    }

    /// Find all methods with a given name (for conservative resolution).
    /// Returns cloned signatures to avoid holding DashMap locks.
    pub fn find_methods_by_name(&self, method_name: &str) -> Vec<FunctionSignature> {
        if let Some(candidates) = self.method_lookup.get(method_name) {
            candidates
                .iter()
                .filter_map(|key| self.type_methods.get(key).map(|r| r.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Index all items in a list (recursive for nested modules).
    fn index_items(&self, crate_name: &str, file_path: &str, items: &[Item]) {
        for item in items {
            match item {
                Item::Fn(func) => {
                    let name = func.sig.ident.to_string();
                    let is_public = matches!(func.vis, Visibility::Public(_));
                    let span = func.sig.ident.span();
                    let line = span.start().line;
                    
                    let qualified_name = format!("{}::{}", crate_name, name);

                    let sig = FunctionSignature {
                        name: name.clone(),
                        is_public,
                        receiver: None,
                        location: format!("{}:{}", file_path, line),
                        crate_name: crate_name.to_string(),
                    };
                    self.global_functions.insert(qualified_name, sig);
                }
                Item::Impl(imp) => {
                    if let Type::Path(tp) = &*imp.self_ty {
                        if let Some(segment) = tp.path.segments.last() {
                            let type_name = segment.ident.to_string();
                            
                            for impl_item in &imp.items {
                                if let syn::ImplItem::Fn(method) = impl_item {
                                    let method_name = method.sig.ident.to_string();
                                    let is_public = matches!(method.vis, Visibility::Public(_));
                                    let span = method.sig.ident.span();
                                    let line = span.start().line;

                                    let receiver = method.sig.inputs.first().and_then(|arg| {
                                        match arg {
                                            syn::FnArg::Receiver(r) => {
                                                if r.reference.is_some() { Some("&self".to_string()) }
                                                else { Some("self".to_string()) }
                                            },
                                            _ => None,
                                        }
                                    });

                                    let sig = FunctionSignature {
                                        name: method_name.clone(),
                                        is_public,
                                        receiver,
                                        location: format!("{}:{}", file_path, line),
                                        crate_name: crate_name.to_string(),
                                    };

                                    let key = (type_name.clone(), method_name.clone());
                                    self.type_methods.insert(key.clone(), sig);
                                    
                                    // Thread-safe append to method_lookup
                                    self.method_lookup
                                        .entry(method_name.clone())
                                        .or_default()
                                        .push(key);
                                }
                            }
                        }
                    }
                }
                Item::Mod(module) => {
                    // Recurse into inline modules
                    if let Some((_, content)) = &module.content {
                        self.index_items(crate_name, file_path, content);
                    }
                }
                _ => {}
            }
        }
    }
}
