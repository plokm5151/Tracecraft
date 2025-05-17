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
        // 每個 function 建立一個 CallGraphNode（用 function 名稱做 id）
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
        std::fs::write(path, data)
    }
}
