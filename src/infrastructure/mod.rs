use crate::ports::{CallGraphBuilder, OutputExporter};
use crate::domain::callgraph::{CallGraph, CallGraphNode};
use syn::{File, Item, Expr, Stmt};
use std::collections::HashMap;
use std::fs;

pub struct DotExporter;
impl OutputExporter for DotExporter {
    fn export(&self, cg: &CallGraph, path: &str) -> std::io::Result<()> {
        let mut lines = vec!["digraph G {".to_string()];
        for node in &cg.nodes {
            // 只標註來源 filename 作為 label（因 syn/proc_macro2 v2 沒行號 API）
            let label = if let Some(ref meta) = node.label {
                format!("{}\\n{}", node.id, meta)
            } else {
                node.id.clone()
            };
            lines.push(format!("    \"{}\" [label=\"{}\"];", node.id, label));
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
        let mut trait_impls: HashMap<String, Vec<String>> = HashMap::new();
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

        let mut func_defs: Vec<NodeInfo> = vec![];
        let mut trait_methods = HashMap::new();
        for (crate_name, path, code) in files {
            let ast_file: File = match syn::parse_file(code) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for item in &ast_file.items {
                let file = path.clone();
                // fn
                if let Item::Fn(ref func) = item {
                    let name = func.sig.ident.to_string();
                    let mut callees = vec![];
                    visit_stmts(&func.block.stmts, &mut callees, &trait_impls, &mut trait_methods, &alias_map);
                    // 沒有行號 API, 只顯示 filename
                    let label = format!("{}", file);
                    println!("DEBUG: insert fn={} crate={} path={} callees={:?}", name, crate_name, path, callees);
                    func_defs.push(NodeInfo {
                        id: format!("{}@{}", name, crate_name),
                        label,
                        file: file.clone(),
                        callees,
                    });
                }
                // impl method
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
                            let mut callees = vec![];
                            visit_stmts(&method.block.stmts, &mut callees, &trait_impls, &mut trait_methods, &alias_map);
                            let label = format!("{}", file);
                            println!("DEBUG: insert method name={} crate={} path={} callees={:?}", method_name, crate_name, path, callees);
                            func_defs.push(NodeInfo {
                                id: format!("{}::{}@{}", type_name, method_name, crate_name),
                                label,
                                file: file.clone(),
                                callees,
                            });
                        }
                    }
                }
                // trait
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
        let mut id_map: HashMap<String, (String, String)> = HashMap::new();
        for n in &func_defs {
            id_map.insert(n.id.clone(), (n.id.clone(), n.file.clone()));
        }
        println!(
            "DEBUG: final id_map keys = {:?}",
            id_map.keys().collect::<Vec<_>>()
        );
        let mut nodes = vec![];
        for n in &func_defs {
            let callee_ids = n.callees.iter().flat_map(|callee_name| {
                if let Some((trait_name, method)) = callee_name.split_once("::") {
                    if let Some(types) = trait_impls.get(trait_name) {
                        types.iter().map(|ty| format!("{}::{}@main", ty, method)).collect::<Vec<_>>()
                    } else {
                        vec![format!("{}@main", callee_name)]
                    }
                } else {
                    // 跨 crate alias 展開
                    let mut real_callee = callee_name.clone();
                    let alias_key = callee_name.split("::").next().unwrap_or("");
                    if let Some(full_path) = alias_map.get(alias_key) {
                        let rest: Vec<&str> = callee_name.split("::").skip(1).collect();
                        let mut full = full_path.clone();
                        for r in rest {
                            full.push(r.to_string());
                        }
                        real_callee = full.join("::");
                    }
                    vec![format!("{}@main", real_callee)]
                }
            })
            .filter(|search_id| {
                if id_map.contains_key(search_id) {
                    true
                } else {
                    println!("DEBUG: resolve-miss search_id={}  all_keys={:?}", search_id, id_map.keys().collect::<Vec<_>>());
                    false
                }
            }).collect();
            nodes.push(CallGraphNode {
                id: n.id.clone(),
                callees: callee_ids,
                label: Some(n.label.clone()),
            });
        }
        CallGraph { nodes }
    }
}

// 完整 NodeInfo 結構，包含 label
struct NodeInfo {
    id: String,
    label: String,
    file: String,
    callees: Vec<String>,
}

// alias 展開工具
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

fn visit_stmts(
    stmts: &Vec<Stmt>,
    callees: &mut Vec<String>,
    trait_impls: &HashMap<String, Vec<String>>,
    trait_methods: &mut HashMap<String, Vec<String>>,
    alias_map: &HashMap<String, Vec<String>>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr, _) => visit_expr(expr, callees, trait_impls, trait_methods, alias_map),
            _ => {}
        }
    }
}
fn visit_expr(
    expr: &Expr,
    callees: &mut Vec<String>,
    trait_impls: &HashMap<String, Vec<String>>,
    trait_methods: &mut HashMap<String, Vec<String>>,
    alias_map: &HashMap<String, Vec<String>>,
) {
    match expr {
        Expr::Call(expr_call) => {
            if let Expr::Path(ref expr_path) = *expr_call.func {
                let segments: Vec<_> = expr_path.path.segments.iter().map(|s| s.ident.to_string()).collect();
                if !segments.is_empty() {
                    println!("DEBUG: Detected call: {}", segments.join("::"));
                    callees.push(segments.join("::"));
                }
            }
            for arg in &expr_call.args {
                visit_expr(arg, callees, trait_impls, trait_methods, alias_map);
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
                visit_expr(arg, callees, trait_impls, trait_methods, alias_map);
            }
            visit_expr(&expr_method.receiver, callees, trait_impls, trait_methods, alias_map);
        }
        Expr::Block(expr_block) => visit_stmts(&expr_block.block.stmts, callees, trait_impls, trait_methods, alias_map),
        _ => {}
    }
}
