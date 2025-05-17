// Infrastructure implementations for TraceCraft.

use crate::domain::ast::{AstNode, AstNodeKind};
use crate::domain::callgraph::*;
use crate::ports::{AstParser, CallGraphBuilder, OutputExporter};
use syn::{File, Item};

pub struct SynAstParser;
impl AstParser for SynAstParser {
    fn parse(&self, src: &str) -> AstNode {
        let ast_file: File = match syn::parse_file(src) {
            Ok(file) => file,
            Err(_) => return AstNode {
                kind: AstNodeKind::Module,
                name: None,
                children: vec![],
            },
        };

        let mut children = Vec::new();
        for item in ast_file.items {
            if let Item::Fn(ref func) = item {
                let name = func.sig.ident.to_string();
                children.push(AstNode {
                    kind: AstNodeKind::Function,
                    name: Some(name),
                    children: vec![],
                });
            }
        }
        AstNode {
            kind: AstNodeKind::Module,
            name: None,
            children,
        }
    }
}

pub struct SimpleCallGraphBuilder;
impl CallGraphBuilder for SimpleCallGraphBuilder {
    fn build_call_graph(&self, root: &AstNode) -> CallGraph {
        let mut nodes = Vec::new();
        for child in root.children.iter() {
            if let AstNodeKind::Function = child.kind {
                let id = child.name.clone().unwrap_or("unknown".to_string());
                nodes.push(CallGraphNode {
                    id,
                    callees: vec![],
                });
            }
        }
        CallGraph { nodes }
    }
}

pub struct DotExporter;
impl OutputExporter for DotExporter {
    fn export(&self, data: &str, path: &str) -> std::io::Result<()> {
        // data: 是 call graph 的 debug print，這裡我們要手動轉成 DOT 格式
        // 所以這裡實際應該要接收 CallGraph，但目前介面傳進來的是字串
        // 暫時直接產生一個簡單的 DOT file
        let lines = [
            "digraph G {",
            // 可進一步遍歷 call graph node/callees
            "    main;",
            "}",
        ];
        let dot_content = lines.join("\n");
        std::fs::write(path, dot_content)
    }
}
