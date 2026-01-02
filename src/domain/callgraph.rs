// Call graph structures for Mr. Hedgehog.
// Represents function/module call relationships.

/// A node in the call graph.
#[derive(Debug)]
pub struct CallGraphNode {
    pub id: String, // function/module/unique identifier
    pub callees: Vec<String>, // list of IDs this node calls
    pub label: Option<String>, // label for DOT (file:line etc)
}

/// The call graph itself.
#[derive(Debug)]
pub struct CallGraph {
    pub nodes: Vec<CallGraphNode>,
}

impl CallGraph {
    pub fn new(nodes: Vec<CallGraphNode>) -> Self {
        Self { nodes }
    }

    pub fn add_edge(&mut self, caller_id: &str, callee_id: &str) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == caller_id) {
            node.callees.push(callee_id.to_string());
        }
    }
}

/// Call graph for a single file.
pub struct FileCallGraph {
    pub filename: String,
    pub callgraph: CallGraph,
}
