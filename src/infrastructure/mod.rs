use syn::{Item, Stmt, Expr, ImplItem, Type};
use crate::domain::callgraph::{CallGraph, CallGraphNode};

pub struct SimpleCallGraphBuilder;

impl crate::ports::CallGraphBuilder for SimpleCallGraphBuilder {
    fn build_call_graph(&self, files: &[(String, String, String)]) -> CallGraph {
        let mut impls = Vec::new();
        let mut func_defs = Vec::new();

        for (crate_name, file, code) in files {
            let ast_file = syn::parse_file(code).expect("Parse error");
            // 收集 impl
            for item in &ast_file.items {
                if let Item::Impl(imp) = item {
                    if let Type::Path(tp) = &*imp.self_ty {
                        let type_name = tp.path.segments.last().unwrap().ident.to_string();
                        for ii in &imp.items {
                            if let ImplItem::Fn(f) = ii {
                                let method_name = f.sig.ident.to_string();
                                impls.push((type_name.clone(), method_name));
                            }
                        }
                    }
                }
            }
            // 收集 fn
            for item in &ast_file.items {
                if let Item::Fn(func) = item {
                    let name = func.sig.ident.to_string();
                    let mut callees = vec![];
                    visit_stmts(&func.block.stmts, &mut callees, &impls, crate_name);
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
                    id: format!("{}@{}", name, crate_name),
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
    impls: &Vec<(String, String)>,
    crate_name: &str,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Expr(expr, _) => visit_expr(expr, callees, impls, crate_name),
            _ => {}
        }
    }
}

fn visit_expr(
    expr: &Expr,
    callees: &mut Vec<String>,
    impls: &Vec<(String, String)>,
    crate_name: &str,
) {
    match expr {
        Expr::Call(expr_call) => {
            if let Expr::Path(ref expr_path) = *expr_call.func {
                let segments: Vec<_> = expr_path.path.segments.iter().map(|s| s.ident.to_string()).collect();
                if !segments.is_empty() {
                    callees.push(format!("{}@{}", segments.join("::"), crate_name));
                }
            }
            for arg in &expr_call.args {
                visit_expr(arg, callees, impls, crate_name);
            }
        }
        Expr::MethodCall(expr_method) => {
            let method_name = expr_method.method.to_string();
            // 嘗試靜態取得 receiver 型別
            let receiver_type = match &*expr_method.receiver {
                Expr::Path(expr_path) => expr_path.path.segments.last().map(|s| s.ident.to_string()),
                _ => None,
            };
            if let Some(rt) = &receiver_type {
                let mut found = false;
                for (type_name, method) in impls {
                    if type_name == rt && method == &method_name {
                        let callee_id = format!("{}::{}@{}", type_name, method_name, crate_name);
                        callees.push(callee_id);
                        found = true;
                        break;
                    }
                }
                if !found {
                    callees.push(format!("{}::{}@{}", rt, method_name, crate_name));
                }
            } else {
                callees.push(format!("{}@{}", method_name, crate_name));
            }
            for arg in &expr_method.args {
                visit_expr(arg, callees, impls, crate_name);
            }
            visit_expr(&expr_method.receiver, callees, impls, crate_name);
        }
        Expr::Block(expr_block) => visit_stmts(&expr_block.block.stmts, callees, impls, crate_name),
        Expr::If(expr_if) => {
            callees.push("if(...)".to_string());
            visit_expr(&expr_if.cond, callees, impls, crate_name);
            visit_block(&expr_if.then_branch, callees, impls, crate_name);
            if let Some((_, else_branch)) = &expr_if.else_branch {
                match &**else_branch {
                    Expr::Block(block) => visit_block(&block.block, callees, impls, crate_name),
                    Expr::If(else_if) => {
                        callees.push("else if(...)".to_string());
                        visit_expr(&else_if.cond, callees, impls, crate_name);
                        visit_block(&else_if.then_branch, callees, impls, crate_name);
                    }
                    other => visit_expr(other, callees, impls, crate_name),
                }
            }
        }
        Expr::Match(expr_match) => {
            callees.push("match(...)".to_string());
            visit_expr(&expr_match.expr, callees, impls, crate_name);
            for (i, arm) in expr_match.arms.iter().enumerate() {
                let label = format!("match_arm_{}", i);
                callees.push(label.clone());
                visit_expr(&arm.body, callees, impls, crate_name);
            }
        }
        _ => {}
    }
}

fn visit_block(
    block: &syn::Block,
    callees: &mut Vec<String>,
    impls: &Vec<(String, String)>,
    crate_name: &str,
) {
    visit_stmts(&block.stmts, callees, impls, crate_name);
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
