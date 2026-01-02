use syn::{Item, Stmt, Expr};
use crate::domain::callgraph::{CallGraph, CallGraphNode};
use crate::domain::index::SymbolIndex;

pub mod project_loader;
pub mod source_manager;
pub mod expander;

pub struct SimpleCallGraphBuilder;

impl SimpleCallGraphBuilder {
    pub fn build_from_asts(&self, index: &SymbolIndex, asts: &[(String, String, syn::File)]) -> CallGraph {
        let mut func_defs = Vec::new();

        // Step 1: Collect all potential nodes (functions and methods)
        for (crate_name, file, ast) in asts {
            for item in &ast.items {
                 if let Item::Fn(func) = item {
                     // Register node
                     let name = func.sig.ident.to_string();
                     // Use unique ID from index logic: crate::func
                     let id = format!("{}::{}", crate_name, name);
                     let label = Some(format!("{}::{}", crate_name, name)); // Label is Option<String>
                     
                     func_defs.push(CallGraphNode {
                         id: id.clone(),
                         callees: Vec::new(), // Initialize with empty callees
                         label,
                     });
                 }
                 // Handle Impl blocks
                 if let Item::Impl(imp) = item {
                     // Try to resolve Type
                     if let syn::Type::Path(tp) = &*imp.self_ty {
                         if let Some(segment) = tp.path.segments.last() {
                             let type_name = segment.ident.to_string();
                             for item in &imp.items {
                                 if let syn::ImplItem::Fn(method) = item {
                                     let method_name = method.sig.ident.to_string();
                                     let id = format!("{}::{}@{}", type_name, method_name, crate_name);
                                     let label = Some(format!("{}::{}", type_name, method_name)); // Label is Option<String>
                                     
                                     func_defs.push(CallGraphNode {
                                         id, 
                                         callees: Vec::new(), // Initialize with empty callees
                                         label,
                                     });
                                 }
                             }
                         }
                     }
                 }
            }
        }

        // Create a CallGraph from the collected nodes.
        // The CallGraph constructor will likely convert this Vec<CallGraphNode> into a HashMap for efficient lookup.
        let mut graph = CallGraph::new_from_nodes(func_defs);

        // Step 2: Analyze bodies to add edges
        for (crate_name, _, ast) in asts {
             self.visit_ast_items(&ast.items, &mut graph, index, crate_name);
        }

        graph
    }

    fn visit_ast_items(&self, items: &[Item], graph: &mut CallGraph, index: &SymbolIndex, crate_name: &str) {
        for item in items {
            match item {
                Item::Fn(func) => {
                     let caller_id = format!("{}::{}", crate_name, func.sig.ident);
                     let mut callees = Vec::new();
                     for stmt in &func.block.stmts {
                         visit_stmt(stmt, &mut callees, index, crate_name); // Changed to visit_stmt
                     }
                     for callee in callees {
                         graph.add_edge(&caller_id, &callee);
                     }
                }
                Item::Impl(imp) => {
                     if let syn::Type::Path(tp) = &*imp.self_ty {
                         if let Some(segment) = tp.path.segments.last() {
                             let type_name = segment.ident.to_string();
                             for item in &imp.items {
                                 if let syn::ImplItem::Fn(method) = item {
                                     let method_name = method.sig.ident.to_string();
                                     let caller_id = format!("{}::{}@{}", type_name, method_name, crate_name);
                                     let mut callees = Vec::new();
                                     for stmt in &method.block.stmts {
                                         visit_stmt(stmt, &mut callees, index, crate_name); // Changed to visit_stmt
                                     }
                                     for callee in callees {
                                         graph.add_edge(&caller_id, &callee);
                                     }
                                 }
                             }
                         }
                     }
                }
                Item::Mod(module) => {
                    if let Some((_, content)) = &module.content {
                         self.visit_ast_items(content, graph, index, crate_name);
                    }
                }
                _ => {}
            }
        }
    }
}

impl crate::ports::CallGraphBuilder for SimpleCallGraphBuilder {
    fn build_call_graph(&self, files: &[(String, String, String)]) -> CallGraph {
        // Step 1: Build the global symbol index
        // We now get index + cached ASTs + errors
        let (index, asts, errors) = SymbolIndex::build(files);
        
        if !errors.is_empty() {
             eprintln!(" WARN: Encountered {} parse errors:", errors.len());
             for e in &errors {
                 eprintln!("  - {}: {}", e.file, e.error);
             }
        }
        


        // Use the new build_from_asts method
        self.build_from_asts(&index, &asts)
    }
}

// 遍歷語法樹、分析函式呼叫
fn visit_stmt( // Renamed from visit_stmts to visit_stmt
    stmt: &Stmt, // Changed from &Vec<Stmt> to &Stmt
    callees: &mut Vec<String>,
    index: &SymbolIndex,
    crate_name: &str,
) {
    match stmt {
        Stmt::Expr(expr, _) => visit_expr(expr, callees, index, crate_name),
        _ => {}
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
                let key = (rt.clone(), method_name.clone());
                if let Some(sig_ref) = index.type_methods.get(&key) {
                     // Found it! Use canonical ID. Dereference the Ref guard.
                     let callee_id = format!("{}::{}@{}", rt, method_name, sig_ref.crate_name);
                     callees.push(callee_id);
                     resolved = true;
                }
            }
            
            // Strategy 2: Conservative Lookup (Name-based resolution)
            if !resolved {
                let candidates = index.find_methods_by_name(&method_name);
                if !candidates.is_empty() {
                    // Link to ALL matching methods (conservative approach)
                    for sig in candidates {
                        let callee_id = format!("{}::{}@{}", sig.name, method_name, sig.crate_name);
                        callees.push(callee_id);
                    }
                    resolved = true;
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
