use crate::ports::{CallGraphBuilder, OutputExporter};
use crate::domain::callgraph::{CallGraph, CallGraphNode};
use syn::{File, Item, Expr, Stmt, spanned::Spanned};
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
            let label = node.label.clone().unwrap_or_else(|| node.id.clone());
            lines.push(format!("    \"{}\" [label=\"{}\"];", node.id, label.replace("\"", "\\\"")));
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
        for (_crate_name, _path, code) in files {
            let ast_file: File = match syn::parse_file(code) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for item in &ast_file.items {
                if let Item::Use(ref u) = item {
                    visit_use(&u.tree, vec![], &mut alias_map);
                }
            }
        }
        println!("DEBUG: alias_map = {:?}", alias_map);

        let mut func_defs = vec![];
        for (crate_name, path, code) in files {
            let ast_file: File = match syn::parse_file(code) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let file = path.clone();
            for item in &ast_file.items {
                if let Item::Fn(ref func) = item {
                    let name = func.sig.ident.to_string();
                    let mut callees = vec![];
                    visit_stmts(&func.block.stmts, &mut callees);
                    let line = func.sig.ident.span().start().line;
                    let label = Some(format!("{}:{}", file, line));
                    println!(
                        "DEBUG: insert fn={} crate={} path={} line={} callees={:?}",
                        name, crate_name, path, line, callees
                    );
                    func_defs.push((name, crate_name.clone(), path.clone(), callees, label));
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
                            visit_stmts(&method.block.stmts, &mut callees);
                            let line = method.sig.ident.span().start().line;
                            let label = Some(format!("{}:{}", file, line));
                            println!(
                                "DEBUG: insert method name={} crate={} path={} line={} callees={:?}",
                                name, crate_name, path, line, callees
                            );
                            func_defs.push((name, crate_name.clone(), path.clone(), callees, label));
                        }
                    }
                }
            }
        }
        let mut id_map: HashMap<String, (String, String, String)> = HashMap::new();
        for (name, crate_name, path, _, _) in &func_defs {
            let id = format!("{}@{}", name, crate_name);
            id_map.insert(id, (name.clone(), crate_name.clone(), path.clone()));
        }
        println!(
            "DEBUG: final id_map keys = {:?}",
            id_map.keys().collect::<Vec<_>>()
        );
        let mut nodes = vec![];
        for (name, crate_name, _path, callees, label) in &func_defs {
            let id = format!("{}@{}", name, crate_name);
            let callee_ids = callees
                .iter()
                .filter_map(|callee_name| {
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
                    let search_id = format!("{}@{}", real_callee, crate_name);
                    if id_map.contains_key(&search_id) {
                        Some(search_id)
                    } else {
                        println!(
                            "DEBUG: resolve-miss search_id={}  all_keys={:?}",
                            search_id,
                            id_map.keys().collect::<Vec<_>>()
                        );
                        None
                    }
                })
                .collect();
            nodes.push(CallGraphNode {
                id,
                callees: callee_ids,
                label: label.clone(),
            });
        }
        CallGraph { nodes }
    }
}

fn visit_stmts(stmts: &Vec<Stmt>, callees: &mut Vec<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr, _) => visit_expr(expr, callees),
            _ => {}
        }
    }
}
fn visit_expr(expr: &Expr, callees: &mut Vec<String>) {
    match expr {
        Expr::Call(expr_call) => {
            if let Expr::Path(ref expr_path) = *expr_call.func {
                let segments: Vec<_> =
                    expr_path.path.segments.iter().map(|s| s.ident.to_string()).collect();
                if !segments.is_empty() {
                    callees.push(segments.join("::"));
                }
            }
            for arg in &expr_call.args {
                visit_expr(arg, callees);
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
            let callee_id = if let Some(ty) = receiver_type {
                format!("{}::{}", ty, method_name)
            } else {
                method_name.clone()
            };
            callees.push(callee_id);
            for arg in &expr_method.args {
                visit_expr(arg, callees);
            }
            visit_expr(&expr_method.receiver, callees);
        }
        Expr::Block(expr_block) => visit_stmts(&expr_block.block.stmts, callees),
        _ => {}
    }
}
