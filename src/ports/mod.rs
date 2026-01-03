use crate::domain::callgraph::CallGraph;

pub mod flowchart_exporter;

pub trait CallGraphBuilder {
    fn build_call_graph(&self, sources: &[(String, String, String)]) -> CallGraph;
}

pub trait OutputExporter {
    fn export(&self, cg: &CallGraph, path: &str) -> std::io::Result<()>;
}
