// AST data structures for TraceCraft.
// These types represent parsed Rust code in a form suitable for static analysis.

/// A node in the abstract syntax tree.
#[derive(Debug)]
pub struct AstNode {
    pub kind: AstNodeKind,
    pub name: Option<String>,
    pub children: Vec<AstNode>,
}

/// Supported AST node types (expand as needed).
#[derive(Debug)]
pub enum AstNodeKind {
    Function,
    Module,
    Struct,
    Enum,
    Trait,
    Impl,
    Macro,
    Statement,
    Expression,
    // ... add more as needed
}
