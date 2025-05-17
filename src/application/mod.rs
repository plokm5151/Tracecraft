// Application layer: main usecase orchestration for static analysis.

use crate::ports::{AstParser, CallGraphBuilder, OutputExporter};

/// The main usecase: analyze source code and export results.
pub struct AnalyzeUsecase<'a> {
    pub parser: &'a dyn AstParser,
    pub callgraph_builder: &'a dyn CallGraphBuilder,
    pub exporter: &'a dyn OutputExporter,
}

impl<'a> AnalyzeUsecase<'a> {
    pub fn run(&self, src: &str, export_path: &str) -> std::io::Result<()> {
        let ast = self.parser.parse(src);
        let cg = self.callgraph_builder.build_call_graph(&ast);
        // For now, just serialize the callgraph with debug output.
        let output = format!("{:#?}", cg);
        self.exporter.export(&output, export_path)
    }
}
