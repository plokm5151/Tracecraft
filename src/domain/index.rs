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

use std::sync::Arc;

/// Thread-safe symbol index that delegates storage to a SymbolStore backend.
/// Enables parallel parsing and indexing with either memory or disk persistence.
pub struct SymbolIndex {
    pub store: Arc<dyn SymbolStore>,
}

impl SymbolIndex {
    pub fn new(store: Arc<dyn SymbolStore>) -> Self {
        Self { store }
    }

    #[derive(Debug, Clone)]
    pub struct AnalysisError {
        pub file: String,
        pub error: String,
    }

    /// Build the symbol index from source files in parallel and return any errors.
    pub fn build(sources: &[(String, String, String)], store: Arc<dyn SymbolStore>) -> (Self, Vec<AnalysisError>) {
        let index = SymbolIndex::new(store);

        // Parallel parsing and indexing
        let errors: Vec<AnalysisError> = sources.par_iter()
            .filter_map(|(crate_name, file_path, code)| {
                match syn::parse_file(code) {
                    Ok(ast) => {
                        index.index_items(crate_name, file_path, &ast.items);
                        None
                    }
                    Err(e) => {
                        Some(AnalysisError {
                            file: file_path.clone(),
                            error: e.to_string(),
                        })
                    }
                }
            })
            .collect();

        (index, errors)
    }

    /// Find all methods with a given name (for conservative resolution).
    pub fn find_methods_by_name(&self, method_name: &str) -> Vec<FunctionSignature> {
        self.store.find_methods_by_name(method_name)
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
                    self.store.insert_function(qualified_name, sig);
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

                                    self.store.insert_method(type_name.clone(), method_name.clone(), sig);
                                    self.store.register_method_lookup(method_name, type_name.clone());
                                }
                            }
                        }
                    }
                }
                Item::Mod(module) => {
                    if let Some((_, content)) = &module.content {
                        self.index_items(crate_name, file_path, content);
                    }
                }
                _ => {}
            }
        }
    }
}
