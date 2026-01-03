//! Flowchart DOT Exporter
//!
//! Exports FlowGraph as Graphviz DOT with flowchart styling.

use crate::domain::flowgraph::{FlowGraph, FlowNodeType};
use std::io::Result;

pub struct FlowchartExporter;

impl FlowchartExporter {
    /// Export a FlowGraph to DOT format with flowchart styling.
    pub fn export(flow: &FlowGraph, path: &str) -> Result<()> {
        let content = Self::to_dot(flow);
        std::fs::write(path, content)
    }

    /// Convert FlowGraph to DOT string.
    pub fn to_dot(flow: &FlowGraph) -> String {
        let mut lines = Vec::new();

        // Graph configuration for flowchart layout
        lines.push("digraph FlowChart {".to_string());
        lines.push("    rankdir=TB;".to_string()); // Top to bottom
        lines.push("    splines=ortho;".to_string()); // Orthogonal edges
        lines.push("    nodesep=0.8;".to_string());
        lines.push("    ranksep=1.0;".to_string());
        lines.push("    node [fontname=\"Helvetica\", fontsize=12];".to_string());
        lines.push("    edge [fontname=\"Helvetica\", fontsize=10];".to_string());
        lines.push("".to_string());

        // Node definitions with styling
        for node in &flow.nodes {
            let (shape, color, style) = Self::node_style(&node.node_type);
            let label = Self::escape_label(&node.label);
            lines.push(format!(
                "    \"{}\" [label=\"{}\", shape={}, style=\"{}\", fillcolor=\"{}\", color=\"{}\"];",
                node.id, label, shape, style, color, Self::border_color(&node.node_type)
            ));
        }

        lines.push("".to_string());

        // Edge definitions with sequence numbers
        for edge in &flow.edges {
            let label = edge
                .label
                .as_ref()
                .map(|l| format!(" [{}]", l))
                .unwrap_or_default();
            lines.push(format!(
                "    \"{}\" -> \"{}\" [label=\"{}{}\"];",
                edge.from, edge.to, edge.sequence, label
            ));
        }

        // Group nodes by depth for layered layout
        let layers = flow.nodes_by_depth();
        for (depth, layer) in layers.iter().enumerate() {
            if !layer.is_empty() {
                let node_ids: Vec<String> = layer.iter().map(|n| format!("\"{}\"", n.id)).collect();
                lines.push(format!("    {{ rank=same; {} }}", node_ids.join("; ")));
            }
        }

        lines.push("}".to_string());

        lines.join("\n")
    }

    fn node_style(node_type: &FlowNodeType) -> (&'static str, &'static str, &'static str) {
        match node_type {
            FlowNodeType::Entry => ("box", "#a6e3a1", "filled,rounded"), // Green
            FlowNodeType::Call => ("box", "#89b4fa", "filled"),          // Blue
            FlowNodeType::Branch => ("diamond", "#f9e2af", "filled"),    // Yellow
            FlowNodeType::Loop => ("hexagon", "#cba6f7", "filled"),      // Purple
            FlowNodeType::Return => ("box", "#f38ba8", "filled,rounded"),// Red
            FlowNodeType::External => ("box", "#6c7086", "filled,dashed"),// Gray
        }
    }

    fn border_color(node_type: &FlowNodeType) -> &'static str {
        match node_type {
            FlowNodeType::Entry => "#40a02b",
            FlowNodeType::Call => "#1e66f5",
            FlowNodeType::Branch => "#df8e1d",
            FlowNodeType::Loop => "#8839ef",
            FlowNodeType::Return => "#d20f39",
            FlowNodeType::External => "#5c5f77",
        }
    }

    fn escape_label(label: &str) -> String {
        label
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::flowgraph::{FlowEdge, FlowNode};

    #[test]
    fn test_to_dot() {
        let flow = FlowGraph {
            entry_points: vec![],
            nodes: vec![
                FlowNode {
                    id: "main".to_string(),
                    label: "main".to_string(),
                    node_type: FlowNodeType::Entry,
                    file_path: None,
                    line: None,
                    depth: 0,
                },
                FlowNode {
                    id: "foo".to_string(),
                    label: "foo".to_string(),
                    node_type: FlowNodeType::Call,
                    file_path: None,
                    line: None,
                    depth: 1,
                },
            ],
            edges: vec![FlowEdge {
                from: "main".to_string(),
                to: "foo".to_string(),
                sequence: 1,
                label: None,
            }],
        };

        let dot = FlowchartExporter::to_dot(&flow);
        assert!(dot.contains("digraph FlowChart"));
        assert!(dot.contains("rankdir=TB"));
        assert!(dot.contains("\"main\""));
        assert!(dot.contains("\"foo\""));
        assert!(dot.contains("->"));
    }
}
