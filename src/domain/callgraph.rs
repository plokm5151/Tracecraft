// Call graph structures for TraceCraft.
// Represents function/module call relationships.

/// A node in the call graph.
#[derive(Debug)]
pub struct CallGraphNode {
    pub id: String, // function/module/unique identifier
    pub callees: Vec<String>, // list of IDs this node calls
    // ... add more metadata as needed
}

/// The call graph itself.
#[derive(Debug)]
pub struct CallGraph {
    pub nodes: Vec<CallGraphNode>,
}

/// Call graph for a single file.
pub struct FileCallGraph {
    pub filename: String,
    pub callgraph: CallGraph,
}
