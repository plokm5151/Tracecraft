
pub struct AnalyzeUsecase<'a> {
    pub callgraph_builder: &'a dyn CallGraphBuilder,
    pub exporter: &'a dyn OutputExporter,
}

impl<'a> AnalyzeUsecase<'a> {
    pub fn run(&self, sources: &[(String, String)], export_path: &str) -> std::io::Result<()> {
        let cg = self.callgraph_builder.build_call_graph(sources);
        self.exporter.export(&cg, export_path)
    }
}
