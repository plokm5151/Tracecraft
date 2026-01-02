use std::collections::HashMap;
use syn::{Item, Type, Visibility};

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub is_public: bool,
    pub receiver: Option<String>, // "&self", "self", or None for static
    pub location: String,         // crate::file:line
    pub crate_name: String,
}

#[derive(Debug, Default)]
pub struct SymbolIndex {
    // Key: crate::mod::func (Note: currently we only track crate::func because we flatten modules)
    pub global_functions: HashMap<String, FunctionSignature>,
    
    // Key: (Type, Method)
    pub type_methods: HashMap<(String, String), FunctionSignature>,
}

impl SymbolIndex {
    pub fn build(sources: &[(String, String, String)]) -> Self {
        let mut index = SymbolIndex::default();

        for (crate_name, file_path, code) in sources {
            // For robustness, parse errors in individual files shouldn't panic the whole process
            if let Ok(ast) = syn::parse_file(code) {
                index.index_file(crate_name, file_path, &ast);
            } else {
                eprintln!("WARN: Failed to parse {}", file_path);
            }
        }

        index
    }

    fn index_file(&mut self, crate_name: &str, file_path: &str, ast: &syn::File) {
        for item in &ast.items {
            match item {
                Item::Fn(func) => {
                    let name = func.sig.ident.to_string();
                    let is_public = matches!(func.vis, Visibility::Public(_));
                    let span = func.sig.ident.span();
                    let line = span.start().line;
                    
                    // TODO: Handle nested modules properly. 
                    // For now, consistent with legacy behavior, we flatten file paths but create unique IDs via crates.
                    let qualified_name = format!("{}::{}", crate_name, name); // Simple crate::func

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
                    // Try to resolve the Type name
                    if let Type::Path(tp) = &*imp.self_ty {
                        // Simple extraction of the last segment (e.g., "MyType" from "crate::MyType")
                        if let Some(segment) = tp.path.segments.last() {
                            let type_name = segment.ident.to_string();
                            
                            for item in &imp.items {
                                if let syn::ImplItem::Fn(method) = item {
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

                                    self.type_methods.insert((type_name.clone(), method_name), sig);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
