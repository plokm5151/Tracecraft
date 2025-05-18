use crate::ports::{CallGraphBuilder, OutputExporter};
use crate::domain::callgraph::{CallGraph, CallGraphNode};
use syn::{File, Item, Expr, Stmt};
use std::collections::HashMap;
use std::fs;

fn visit_use(
    tree: &syn::UseTree,
    prefix: Vec<String>,
    map: &mut HashMap<String, Vec<String>>,
) {
    match tree {
        syn::UseTree::Name(n) => {
            let mut full = prefix.clone();
            full.push(n.ident.to_string());
            map.insert(n.ident.to_string(), full);
        }
        syn::UseTree::Rename(r) => {
            let mut full = prefix.clone();
            full.push(r.ident.to_string());
            map.insert(r.rename.to_string(), full);
        }
        syn::UseTree::Path(p) => {
            let mut pre = prefix.clone();
            pre.push(p.ident.to_string());
            visit_use(&p.tree, pre, map);
        }
        syn::UseTree::Group(g) => {
            for t in &g.items {
                visit_use(t, prefix.clone(), map);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

pub struct DotExporter;
impl OutputExporter for DotExporter {
    fn export(&self, cg: &CallGraph, path: &str) -> std::io::Result<()> {
        let mut lines = vec!["digraph G {".to_string()];
        for node in &cg.nodes {
            lines.push(format!("    \"{}\";", node.id));
            for callee in &node.callees {
                lines.push(format!("    \"{}\" -> \"{}\";", node.id, callee));
            }
        }
        lines.push("}".to_string());
        fs::write(path, lines.join("\n"))
    }
}

pub struct SimpleCallGraphBuilder;
impl CallGraphBuilder for SimpleCallGraphBuilder {
    fn build_call_graph(&self, files: &[(String, String, String)]) -> CallGraph {
        let mut alias_map: HashMap<String, Vec<String>> = HashMap::new();
        let mut trait_impls: HashMap<String, Vec<String>> = HashMap::new(); // trait_name -> [type names]
        for (_crate_name, _path, code) in files {
            let ast_file: File = match syn::parse_file(code) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for item in &ast_file.items {
                if let Item::Use(ref u) = item {
                    visit_use(&u.tree, vec![], &mut alias_map);
                }
                if let Item::Impl(ref imp) = item {
                    if let Some((_, path, _)) = &imp.trait_ {
                        // 這是一個 trait impl
                        let trait_name = path.segments.last().unwrap().ident.to_string();
                        let type_name = match &*imp.self_ty {
                            syn::Type::Path(type_path) => {
                                type_path.path.segments.last().unwrap().ident.to_string()
                            }
                            _ => "Self".to_string(),
                        };
                        trait_impls.entry(trait_name).or_default().push(type_name);
                    }
                }
            }
        }
        println!("DEBUG: alias_map = {:?}", alias_map);
        println!("DEBUG: trait_impls = {:?}", trait_impls);

        let mut func_defs = vec![];
        let mut trait_methods = HashMap::new(); // trait_name -> [method names]
        for (crate_name, path, code) in files {
            let ast_file: File = match syn::parse_file(code) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for item in &ast_file.items {
                if let Item::Fn(ref func) = item {
                    let name = func.sig.ident.to_string();
                    let mut callees = vec![];
                    visit_stmts(&func.block.stmts, &mut callees, &trait_impls, &mut trait_methods);
                    println!(
                        "DEBUG: insert fn={} crate={} path={} callees={:?}",
                        name, crate_name, path, callees
                    );
                    func_defs.push((name, crate_name.clone(), path.clone(), callees));
                }
                if let Item::Impl(ref imp) = item {
                    let type_name = match &*imp.self_ty {
                        syn::Type::Path(type_path) => {
                            type_path.path.segments.last().unwrap().ident.to_string()
                        }
                        _ => "Self".to_string(),
                    };
                    for imp_item in &imp.items {
                        if let syn::ImplItem::Fn(ref method) = imp_item {
                            let method_name = method.sig.ident.to_string();
                            let name = format!("{}::{}", type_name, method_name);
                            let mut callees = vec![];
                            visit_stmts(&method.block.stmts, &mut callees, &trait_impls, &mut trait_methods);
                            println!(
                                "DEBUG: insert method name={} crate={} path={} callees={:?}",
                                name, crate_name, path, callees
                            );
                            func_defs.push((name, crate_name.clone(), path.clone(), callees));
                        }
                    }
                }
                if let Item::Trait(ref tr) = item {
                    let trait_name = tr.ident.to_string();
                    let methods: Vec<_> = tr.items.iter().filter_map(|i| {
                        if let syn::TraitItem::Fn(f) = i {
                            Some(f.sig.ident.to_string())
                        } else { None }
                    }).collect();
                    trait_methods.insert(trait_name, methods);
                }
            }
        }
        let mut id_map: HashMap<String, (String, String, String)> = HashMap::new();
        for (name, crate_name, path, _) in &func_defs {
            let id = format!("{}@{}", name, crate_name);
            id_map.insert(id, (name.clone(), crate_name.clone(), path.clone()));
        }
        println!(
            "DEBUG: final id_map keys = {:?}",
            id_map.keys().collect::<Vec<_>>()
        );
        let mut nodes = vec![];
        for (name, crate_name, _path, callees) in &func_defs {
            let id = format!("{}@{}", name, crate_name);
            let callee_ids = callees
                .iter()
                .flat_map(|callee_name| {
                    // 若是 "Trait::method"（trait method call），展開所有 impl
                    if let Some((trait_name, method)) = callee_name.split_once("::") {
                        if let Some(types) = trait_impls.get(trait_name) {
                            // 展開所有 Type::method
                            types.iter().map(|ty| format!("{}::{}@{}", ty, method, crate_name)).collect::<Vec<_>>()
                        } else {
                            vec![format!("{}@{}", callee_name, crate_name)]
                        }
                    } else {
                        vec![format!("{}@{}", callee_name, crate_name)]
                    }
                })
                .filter(|search_id| {
                    if id_map.contains_key(search_id) {
                        true
                    } else {
                        println!(
                            "DEBUG: resolve-miss search_id={}  all_keys={:?}",
                            search_id,
                            id_map.keys().collect::<Vec<_>>()
                        );
                        false
                    }
                })
                .collect();
            nodes.push(CallGraphNode {
                id,
                callees: callee_ids,
            });
        }
        CallGraph { nodes }
    }
}

fn visit_stmts(
    stmts: &Vec<Stmt>,
    callees: &mut Vec<String>,
    trait_impls: &HashMap<String, Vec<String>>,
    trait_methods: &mut HashMap<String, Vec<String>>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr, _) => visit_expr(expr, callees, trait_impls, trait_methods),
            _ => {}
        }
    }
}
fn visit_expr(
    expr: &Expr,
    callees: &mut Vec<String>,
    trait_impls: &HashMap<String, Vec<String>>,
    trait_methods: &mut HashMap<String, Vec<String>>,
) {
    match expr {
        Expr::Call(expr_call) => {
            if let Expr::Path(ref expr_path) = *expr_call.func {
                let segments: Vec<_> =
                    expr_path.path.segments.iter().map(|s| s.ident.to_string()).collect();
                if !segments.is_empty() {
                    println!("DEBUG: Detected call: {}", segments.join("::"));
                    callees.push(segments.join("::"));
                }
            }
            for arg in &expr_call.args {
                visit_expr(arg, callees, trait_impls, trait_methods);
            }
        }
        Expr::MethodCall(expr_method) => {
            let method_name = expr_method.method.to_string();
            let receiver_type = match &*expr_method.receiver {
                Expr::Path(expr_path) => expr_path.path.segments.last().map(|s| {
                    let id = s.ident.to_string();
                    if id.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                        let mut chars = id.chars();
                        if let Some(f) = chars.next() {
                            f.to_uppercase().collect::<String>() + chars.as_str()
                        } else {
                            id
                        }
                    } else {
                        id
                    }
                }),
                _ => None,
            };

            // 如果 method 名字在 trait_methods 裡出現，代表這可能是 trait 呼叫
            let mut is_trait_method = false;
            for (trait_name, methods) in trait_methods.iter() {
                if methods.contains(&method_name) {
                    is_trait_method = true;
                    let callee_id = format!("{}::{}", trait_name, method_name);
                    println!("DEBUG: Detected trait method call: {}", callee_id);
                    callees.push(callee_id);
                }
            }
            if !is_trait_method {
                let callee_id = if let Some(ty) = receiver_type {
                    format!("{}::{}", ty, method_name)
                } else {
                    method_name.clone()
                };
                println!("DEBUG: Detected method call: {}", callee_id);
                callees.push(callee_id);
            }
            for arg in &expr_method.args {
                visit_expr(arg, callees, trait_impls, trait_methods);
            }
            visit_expr(&expr_method.receiver, callees, trait_impls, trait_methods);
        }
        Expr::Block(expr_block) => visit_stmts(&expr_block.block.stmts, callees, trait_impls, trait_methods),
        _ => {}
    }
}
