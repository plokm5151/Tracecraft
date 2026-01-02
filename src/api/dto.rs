use serde::{Serialize, Deserialize};
use crate::domain::callgraph::CallGraph;

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphDto {
    pub nodes: Vec<NodeDto>,
    pub edges: Vec<EdgeDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeDto {
    pub id: String,
    pub label: String,
    pub package: String,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EdgeDto {
    pub from: String,
    pub to: String,
    pub type_: String,
}

impl From<CallGraph> for GraphDto {
    fn from(cg: CallGraph) -> Self {
        let nodes = cg.nodes.iter().map(|n| {
            // Simple heuristics to extract package/language from ID or label if available
            // For now, mapping simplified fields.
            NodeDto {
                id: n.id.clone(),
                label: n.label.clone().unwrap_or_else(|| n.id.clone()),
                package: "unknown".to_string(), // TODO: Extract from ID
                language: "rust".to_string(),
            }
        }).collect();

        let mut edges = Vec::new();
        for node in &cg.nodes {
            for callee in &node.callees {
                edges.push(EdgeDto {
                    from: node.id.clone(),
                    to: callee.clone(),
                    type_: "call".to_string(),
                });
            }
        }

        GraphDto { nodes, edges }
    }
}
