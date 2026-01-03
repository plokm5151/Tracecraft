//! FlowGraph Data Structure
//!
//! Represents sequential execution flow from entry points.

use crate::domain::entry_point::{EntryPoint, EntryPointKind};
use crate::domain::callgraph::CallGraph;
use std::collections::{HashMap, HashSet};

/// Represents a sequential execution flow graph.
#[derive(Debug, Clone)]
pub struct FlowGraph {
    /// Detected entry points
    pub entry_points: Vec<EntryPoint>,
    /// All nodes in the graph
    pub nodes: Vec<FlowNode>,
    /// Edges representing execution flow
    pub edges: Vec<FlowEdge>,
}

/// A node in the flow graph
#[derive(Debug, Clone)]
pub struct FlowNode {
    /// Unique identifier
    pub id: String,
    /// Display label
    pub label: String,
    /// Node type for visual styling
    pub node_type: FlowNodeType,
    /// Source file path
    pub file_path: Option<String>,
    /// Line number
    pub line: Option<usize>,
    /// Execution depth from entry point
    pub depth: usize,
}

/// Classification of node types for visual styling
#[derive(Debug, Clone, PartialEq)]
pub enum FlowNodeType {
    /// Entry point (green, rounded rectangle)
    Entry,
    /// Regular function call (blue, rectangle)
    Call,
    /// Branch point (yellow, diamond)
    Branch,
    /// Loop construct (purple, hexagon)
    Loop,
    /// Return/Exit point (red, rounded rectangle)
    Return,
    /// External/Library call (gray, dashed)
    External,
}

/// An edge in the flow graph
#[derive(Debug, Clone)]
pub struct FlowEdge {
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Execution sequence number
    pub sequence: usize,
    /// Edge label (e.g., "then", "else", "loop")
    pub label: Option<String>,
}

impl FlowGraph {
    /// Create a FlowGraph from a CallGraph starting from detected entry points.
    pub fn from_callgraph(
        callgraph: &CallGraph,
        entry_points: Vec<EntryPoint>,
        max_depth: usize,
    ) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut visited = HashSet::new();
        let mut sequence = 0;

        // Build adjacency map from callgraph
        let adj_map: HashMap<String, Vec<String>> = callgraph
            .nodes
            .iter()
            .map(|n| (n.id.clone(), n.callees.clone()))
            .collect();

        // Process each entry point
        for entry in &entry_points {
            let node_type = match entry.kind {
                EntryPointKind::Main | EntryPointKind::AsyncMain | EntryPointKind::PythonMain => {
                    FlowNodeType::Entry
                }
                EntryPointKind::FlaskRoute | EntryPointKind::FastAPIRoute | EntryPointKind::DjangoView => {
                    FlowNodeType::Entry
                }
                _ => FlowNodeType::Call,
            };

            let entry_node = FlowNode {
                id: entry.id.clone(),
                label: entry.name.clone(),
                node_type,
                file_path: Some(entry.file_path.clone()),
                line: entry.line,
                depth: 0,
            };
            nodes.push(entry_node);
            visited.insert(entry.id.clone());

            // DFS from this entry point
            Self::expand_node(
                &entry.id,
                0,
                max_depth,
                &adj_map,
                &mut nodes,
                &mut edges,
                &mut visited,
                &mut sequence,
            );
        }

        FlowGraph {
            entry_points,
            nodes,
            edges,
        }
    }

    fn expand_node(
        node_id: &str,
        depth: usize,
        max_depth: usize,
        adj_map: &HashMap<String, Vec<String>>,
        nodes: &mut Vec<FlowNode>,
        edges: &mut Vec<FlowEdge>,
        visited: &mut HashSet<String>,
        sequence: &mut usize,
    ) {
        if depth >= max_depth {
            return;
        }

        if let Some(callees) = adj_map.get(node_id) {
            for callee in callees {
                *sequence += 1;

                // Add edge
                edges.push(FlowEdge {
                    from: node_id.to_string(),
                    to: callee.clone(),
                    sequence: *sequence,
                    label: None,
                });

                // Add node if not visited
                if !visited.contains(callee) {
                    visited.insert(callee.clone());

                    let node_type = Self::infer_node_type(callee);
                    let label = callee
                        .split("::")
                        .last()
                        .unwrap_or(callee)
                        .split('@')
                        .next()
                        .unwrap_or(callee)
                        .to_string();

                    nodes.push(FlowNode {
                        id: callee.clone(),
                        label,
                        node_type,
                        file_path: None,
                        line: None,
                        depth: depth + 1,
                    });

                    // Recurse
                    Self::expand_node(
                        callee,
                        depth + 1,
                        max_depth,
                        adj_map,
                        nodes,
                        edges,
                        visited,
                        sequence,
                    );
                }
            }
        }
    }

    fn infer_node_type(node_id: &str) -> FlowNodeType {
        let lower = node_id.to_lowercase();
        if lower.contains("if(") || lower.contains("match(") {
            FlowNodeType::Branch
        } else if lower.contains("loop") || lower.contains("while") || lower.contains("for") {
            FlowNodeType::Loop
        } else if lower.contains("return") || lower.contains("exit") {
            FlowNodeType::Return
        } else if lower.contains("std::") || lower.contains("::new") {
            FlowNodeType::External
        } else {
            FlowNodeType::Call
        }
    }

    /// Get nodes grouped by depth for layered rendering
    pub fn nodes_by_depth(&self) -> Vec<Vec<&FlowNode>> {
        let max_depth = self.nodes.iter().map(|n| n.depth).max().unwrap_or(0);
        let mut layers = vec![Vec::new(); max_depth + 1];
        for node in &self.nodes {
            layers[node.depth].push(node);
        }
        layers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::callgraph::CallGraphNode;

    #[test]
    fn test_flowgraph_from_callgraph() {
        let callgraph = CallGraph {
            nodes: vec![
                CallGraphNode {
                    id: "main".to_string(),
                    callees: vec!["foo".to_string(), "bar".to_string()],
                    label: Some("main".to_string()),
                },
                CallGraphNode {
                    id: "foo".to_string(),
                    callees: vec!["baz".to_string()],
                    label: Some("foo".to_string()),
                },
                CallGraphNode {
                    id: "bar".to_string(),
                    callees: vec![],
                    label: Some("bar".to_string()),
                },
                CallGraphNode {
                    id: "baz".to_string(),
                    callees: vec![],
                    label: Some("baz".to_string()),
                },
            ],
        };

        let entries = vec![EntryPoint {
            id: "main".to_string(),
            name: "main".to_string(),
            kind: EntryPointKind::Main,
            file_path: "src/main.rs".to_string(),
            line: Some(1),
        }];

        let flow = FlowGraph::from_callgraph(&callgraph, entries, 5);
        assert_eq!(flow.nodes.len(), 4);
        assert_eq!(flow.edges.len(), 3);
    }
}
