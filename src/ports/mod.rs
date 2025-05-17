// Interface definitions (traits) for TraceCraft core logic.

pub trait AstParser {
    /// Parse source code and return an AST root node.
    fn parse(&self, src: &str) -> crate::domain::ast::AstNode;
}

pub trait CallGraphBuilder {
    /// Build a call graph from the root AST node.
    fn build_call_graph(&self, root: &crate::domain::ast::AstNode) -> crate::domain::callgraph::CallGraph;
}

pub trait OutputExporter {
    /// Export the call graph or AST in a specified format.
    fn export(&self, data: &str, path: &str) -> std::io::Result<()>;
}
