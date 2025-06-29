# TraceCraft

A Rust-based static analysis tool focused on multi-crate workspaces, enabling comprehensive call graph, AST, and dependency tree analysis for large-scale projects, optimized for local and privacy-sensitive environments.

Call graph nodes are labeled using the `function@crate` format so functions from
different crates remain distinct when analyzing workspaces.
