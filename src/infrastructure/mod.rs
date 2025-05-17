// Infrastructure implementations for TraceCraft.

use crate::domain::ast::*;
use crate::domain::callgraph::*;
use crate::ports::{AstParser, CallGraphBuilder, OutputExporter};

pub struct SynAstParser;
impl AstParser for SynAstParser {
    fn parse(&self, src: &str) -> AstNode {
        // TODO: Implement using syn crate
        AstNode {
            kind: AstNodeKind::Module,
            children: vec![],
        }
    }
}

pub struct SimpleCallGraphBuilder;
impl CallGraphBuilder for SimpleCallGraphBuilder {
    fn build_call_graph(&self, root: &AstNode) -> CallGraph {
        // TODO: Implement call graph logic
        CallGraph { nodes: vec![] }
    }
}

pub struct DotExporter;
impl OutputExporter for DotExporter {
    fn export(&self, data: &str, path: &str) -> std::io::Result<()> {
        // TODO: Implement DOT file exporter
        std::fs::write(path, data)
    }
}
