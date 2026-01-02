/// SCIP Index Ingestor.
/// Parses SCIP indices and builds a precise CallGraph using semantic information.

use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};

use crate::domain::callgraph::{CallGraph, CallGraphNode};

/// Represents a range in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceRange {
    start_line: i32,
    start_col: i32,
    end_line: i32,
    end_col: i32,
}

impl SourceRange {
    fn contains(&self, other: &SourceRange) -> bool {
        // Check if `other` is fully contained within `self`
        if self.start_line > other.start_line || self.end_line < other.end_line {
            return false;
        }
        if self.start_line == other.start_line && self.start_col > other.start_col {
            return false;
        }
        if self.end_line == other.end_line && self.end_col < other.end_col {
            return false;
        }
        true
    }
}

/// A definition occurrence extracted from SCIP.
#[derive(Debug, Clone)]
struct DefinitionInfo {
    symbol: String,
    range: SourceRange,
}

/// SCIP Ingestor for building CallGraphs from SCIP indices.
pub struct ScipIngestor;

impl ScipIngestor {
    /// Ingest a SCIP index file and build a CallGraph.
    pub fn ingest_and_build_graph(scip_path: &Path) -> Result<CallGraph> {
        use scip::types::{Index, Occurrence};

        println!("[SCIP Ingest] Loading index from: {}", scip_path.display());
        
        let index = scip::read_index_from_file(scip_path)
            .context("Failed to read SCIP index file")?;

        // Pass 1: Collect all Definitions per file
        // Map: file_path -> Vec<DefinitionInfo>
        let mut definitions_by_file: HashMap<String, Vec<DefinitionInfo>> = HashMap::new();
        // Map: symbol -> node_id (for CallGraph)
        let mut symbol_to_node: HashMap<String, usize> = HashMap::new();
        let mut nodes: Vec<CallGraphNode> = Vec::new();

        for document in &index.documents {
            let file_path = document.relative_path.clone();
            let mut file_defs: Vec<DefinitionInfo> = Vec::new();

            for occurrence in &document.occurrences {
                // Check if this is a Definition (bit 0 of symbol_roles)
                let is_definition = occurrence.symbol_roles & 1 != 0;
                
                if is_definition && !occurrence.symbol.is_empty() {
                    let range = parse_scip_range(&occurrence.range);
                    
                    // Create a node for this definition if we haven't seen it
                    if !symbol_to_node.contains_key(&occurrence.symbol) {
                        let node_id = nodes.len();
                        symbol_to_node.insert(occurrence.symbol.clone(), node_id);
                        
                        // Extract a human-readable label from the symbol
                        let label = extract_label_from_symbol(&occurrence.symbol);
                        
                        nodes.push(CallGraphNode {
                            id: occurrence.symbol.clone(),
                            callees: Vec::new(),
                            label: Some(label),
                        });
                    }

                    file_defs.push(DefinitionInfo {
                        symbol: occurrence.symbol.clone(),
                        range,
                    });
                }
            }

            // Sort definitions by range size (largest first) for containment lookup
            file_defs.sort_by(|a, b| {
                let a_size = (a.range.end_line - a.range.start_line) * 1000 
                           + (a.range.end_col - a.range.start_col);
                let b_size = (b.range.end_line - b.range.start_line) * 1000 
                           + (b.range.end_col - b.range.start_col);
                b_size.cmp(&a_size) // Largest first
            });

            definitions_by_file.insert(file_path, file_defs);
        }

        println!("[SCIP Ingest] Found {} definitions", nodes.len());

        // Pass 2: Find References and create edges
        let mut edge_count = 0;
        
        for document in &index.documents {
            let file_path = &document.relative_path;
            let file_defs = definitions_by_file.get(file_path).cloned().unwrap_or_default();

            for occurrence in &document.occurrences {
                // Check if this is a Reference (not a definition)
                let is_definition = occurrence.symbol_roles & 1 != 0;
                
                if !is_definition && !occurrence.symbol.is_empty() {
                    let ref_range = parse_scip_range(&occurrence.range);
                    let callee_symbol = &occurrence.symbol;

                    // Find the enclosing definition (the caller)
                    for def in &file_defs {
                        if def.range.contains(&ref_range) {
                            // This definition contains the reference
                            let caller_symbol = &def.symbol;
                            
                            // Add edge: caller -> callee
                            if let Some(&caller_idx) = symbol_to_node.get(caller_symbol) {
                                // Avoid self-references and duplicates
                                if caller_symbol != callee_symbol {
                                    if !nodes[caller_idx].callees.contains(callee_symbol) {
                                        nodes[caller_idx].callees.push(callee_symbol.clone());
                                        edge_count += 1;
                                    }
                                }
                            }
                            break; // Found the innermost enclosing definition
                        }
                    }
                }
            }
        }

        println!("[SCIP Ingest] Created {} edges", edge_count);

        Ok(CallGraph { nodes })
    }
}

/// Parse SCIP range format: [start_line, start_col, end_line, end_col] or [start_line, start_col, end_col]
fn parse_scip_range(range: &[i32]) -> SourceRange {
    match range.len() {
        3 => SourceRange {
            start_line: range[0],
            start_col: range[1],
            end_line: range[0], // Same line
            end_col: range[2],
        },
        4 => SourceRange {
            start_line: range[0],
            start_col: range[1],
            end_line: range[2],
            end_col: range[3],
        },
        _ => SourceRange {
            start_line: 0, start_col: 0, end_line: 0, end_col: 0,
        },
    }
}

/// Extract a human-readable label from a SCIP symbol string.
/// SCIP symbols look like: `rust-analyzer cargo crate_name 0.1.0 module/struct#method().`
fn extract_label_from_symbol(symbol: &str) -> String {
    // Take the last meaningful segment
    let parts: Vec<&str> = symbol.split(' ').collect();
    if let Some(last) = parts.last() {
        // Remove trailing punctuation like `().` or `#`
        let cleaned = last.trim_end_matches(|c| c == '(' || c == ')' || c == '.' || c == '#');
        // Replace path separators
        cleaned.replace('/', "::").to_string()
    } else {
        symbol.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_range_contains() {
        let outer = SourceRange {
            start_line: 10, start_col: 0,
            end_line: 20, end_col: 0,
        };
        let inner = SourceRange {
            start_line: 15, start_col: 5,
            end_line: 15, end_col: 10,
        };
        assert!(outer.contains(&inner));
        assert!(!inner.contains(&outer));
    }

    #[test]
    fn test_parse_scip_range() {
        let r3 = parse_scip_range(&[10, 5, 15]);
        assert_eq!(r3.start_line, 10);
        assert_eq!(r3.end_line, 10);
        
        let r4 = parse_scip_range(&[10, 5, 20, 10]);
        assert_eq!(r4.start_line, 10);
        assert_eq!(r4.end_line, 20);
    }

    #[test]
    fn test_extract_label() {
        let symbol = "rust-analyzer cargo my_crate 0.1.0 src/lib.rs/MyStruct#my_method().";
        let label = extract_label_from_symbol(symbol);
        assert!(label.contains("my_method"));
    }
}
