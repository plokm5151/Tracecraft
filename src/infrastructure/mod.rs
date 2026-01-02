use syn::{Item, Stmt, Expr, ImplItem, Type};
use crate::domain::callgraph::{CallGraph, CallGraphNode};
use crate::domain::index::SymbolIndex;

pub mod project_loader;

pub struct SimpleCallGraphBuilder;

impl crate::ports::CallGraphBuilder for SimpleCallGraphBuilder {
    fn build_call_graph(&self, files: &[(String, String, String)]) -> CallGraph {
        // Step 1: Build the global symbol index
        let index = SymbolIndex::build(files);
        
        let mut func_defs = Vec::new();

        // Step 2: Traverse files to build the graph
        for (crate_name, file, code) in files {
             // We can re-parse here or cache the ASTs. For simplicity/memory trade-off, we re-parse.
             // Given build() consumed the files slice to read them, we assume it's cheap enough for now 
             // (or we could have let build take ASTs, but that complicates the API).
             // Actually index.rs parses `syn::parse_file` inside. We duplicate parsing here. 
             // Optimization for later: parse once.
             
            let ast_file = syn::parse_file(code).expect("Parse error");

            for item in &ast_file.items {
                if let Item::Fn(func) = item {
                    let name = func.sig.ident.to_string();
                    let mut callees = vec![];
                    
                    // Pass index instead of impls list
                    visit_stmts(&func.block.stmts, &mut callees, &index, crate_name);
                    
                    let line = func.sig.ident.span().start().line;
                    let label = Some(format!("{}:{}", file, line));
                    func_defs.push((name, crate_name.clone(), "".to_string(), callees, label));
                }
            }
        }

        // 封裝成 CallGraph
        let nodes = func_defs.into_iter()
            .map(|(name, crate_name, _path, callees, label)| {
                CallGraphNode {
                    id: format!("{}@{}", name, crate_name), // Canonical ID for entry points
                    callees,
                    label,
                }
            })
            .collect();
        CallGraph { nodes }
    }
}

// 遍歷語法樹、分析函式呼叫
fn visit_stmts(
    stmts: &Vec<Stmt>,
    callees: &mut Vec<String>,
    index: &SymbolIndex,
    crate_name: &str,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr, _) => visit_expr(expr, callees, index, crate_name),
            _ => {}
        }
    }
}

fn visit_expr(
    expr: &Expr,
    callees: &mut Vec<String>,
    index: &SymbolIndex,
    crate_name: &str,
) {
    match expr {
        Expr::Call(expr_call) => {
            if let Expr::Path(ref expr_path) = *expr_call.func {
                let segments: Vec<_> = expr_path.path.segments.iter().map(|s| s.ident.to_string()).collect();
                if !segments.is_empty() {
                    // Try to resolve global function: crate::mod::func
                    // Currently we don't have full path resolution (imports), 
                    // so we do a best-effort guess or strictly rely on our simplified index keys (crate::func) 
                    // OR just default "name@crate".
                    
                    // If it looks like "func", we assume local or same-crate.
                    // If "mod::func", we check if we can resolve it.
                    // For Stage 2, let's keep the existing logic:
                    // format!("{}@{}", segments.join("::"), crate_name)
                    callees.push(format!("{}@{}", segments.join("::"), crate_name));
                }
            }
            for arg in &expr_call.args {
                visit_expr(arg, callees, index, crate_name);
            }
        }
        Expr::MethodCall(expr_method) => {
            let method_name = expr_method.method.to_string();
            // 嘗試靜態取得 receiver 型別 (Best effort inference)
            let receiver_type = match &*expr_method.receiver {
                Expr::Path(expr_path) => expr_path.path.segments.last().map(|s| s.ident.to_string()),
                _ => None,
            };
            
            let mut resolved = false;

            // Strategy 1: Exact match via inferred type
            if let Some(rt) = &receiver_type {
                if let Some(sig) = index.type_methods.get(&(rt.clone(), method_name.clone())) {
                     // Found it! Use canonical ID.
                     let callee_id = format!("{}::{}@{}", rt, method_name, sig.crate_name);
                     callees.push(callee_id);
                     resolved = true;
                }
            }
            
            // Strategy 2: Conservative Lookup (Name-based resolution)
            if !resolved {
                let candidates = index.find_methods_by_name(&method_name);
                if !candidates.is_empty() {
                    // We found one or more methods with this name. Link to ALL of them.
                    // This is conservative: "It could be any of these".
                    
                    // Actually, let's just use the public method_lookup map directly since fields are public in struct.
                    if let Some(keys) = index.method_lookup.get(&method_name) {
                        for (type_name, _) in keys {
                             if let Some(sig) = index.type_methods.get(&(type_name.clone(), method_name.clone())) {
                                 let callee_id = format!("{}::{}@{}", type_name, method_name, sig.crate_name);
                                 callees.push(callee_id);
                             }
                        }
                        resolved = true;
                    }
                }
            }

            // Strategy 3: Fallback (Unknown local call)
            if !resolved {
                if let Some(rt) = receiver_type {
                    callees.push(format!("{}::{}@{}", rt, method_name, crate_name));
                } else {
                    callees.push(format!("{}@{}", method_name, crate_name));
                }
            }
            
            for arg in &expr_method.args {
                visit_expr(arg, callees, index, crate_name);
            }
            visit_expr(&expr_method.receiver, callees, index, crate_name);
        }
        Expr::Block(expr_block) => visit_stmts(&expr_block.block.stmts, callees, index, crate_name),
        Expr::If(expr_if) => {
            callees.push("if(...)".to_string());
            visit_expr(&expr_if.cond, callees, index, crate_name);
            visit_block(&expr_if.then_branch, callees, index, crate_name);
            if let Some((_, else_branch)) = &expr_if.else_branch {
                match &**else_branch {
                    Expr::Block(block) => visit_block(&block.block, callees, index, crate_name),
                    Expr::If(else_if) => {
                        callees.push("else if(...)".to_string());
                        visit_expr(&else_if.cond, callees, index, crate_name);
                        visit_block(&else_if.then_branch, callees, index, crate_name);
                    }
                    other => visit_expr(other, callees, index, crate_name),
                }
            }
        }
        Expr::Match(expr_match) => {
            callees.push("match(...)".to_string());
            visit_expr(&expr_match.expr, callees, index, crate_name);
            for (i, arm) in expr_match.arms.iter().enumerate() {
                let label = format!("match_arm_{}", i);
                callees.push(label.clone());
                visit_expr(&arm.body, callees, index, crate_name);
            }
        }
        _ => {}
    }
}

fn visit_block(
    block: &syn::Block,
    callees: &mut Vec<String>,
    index: &SymbolIndex,
    crate_name: &str,
) {
    visit_stmts(&block.stmts, callees, index, crate_name);
}

pub struct DotExporter;

impl crate::ports::OutputExporter for DotExporter {
    fn export(&self, cg: &CallGraph, path: &str) -> std::io::Result<()> {
        let mut out = vec![];
        out.push("digraph G {".to_string());
        for n in &cg.nodes {
            let lbl = n.label.clone().unwrap_or_else(|| n.id.clone());
            out.push(format!("    \"{}\" [label=\"{}\"];", n.id, lbl.replace('\"', "\\\"")));
            for c in &n.callees {
                out.push(format!("    \"{}\" -> \"{}\";", n.id, c));
            }
        }
        out.push("}".to_string());
        std::fs::write(path, out.join("\n"))
    }
}
