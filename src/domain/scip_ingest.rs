/// SCIP Index Ingestor.
/// Parses SCIP indices and builds a precise CallGraph using semantic information.
/// 
/// Phase 3.1: Parallel processing with rayon and DashMap for high performance.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use anyhow::{Context, Result};
use dashmap::DashMap;
use rayon::prelude::*;

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
    /// 
    /// Uses parallel processing for both definition collection (Pass 1)
    /// and reference resolution (Pass 2).
    /// 
    /// Phase 3.3: Uses memory-mapped file I/O to avoid large allocations.
    pub fn ingest_and_build_graph(scip_path: &Path) -> Result<CallGraph> {
        use std::fs::File;
        use memmap2::Mmap;
        use protobuf::Message;

        println!("[SCIP Ingest] Loading index from: {}", scip_path.display());
        
        // Memory-map the SCIP index file for efficient access
        let file = File::open(scip_path)
            .context("Failed to open SCIP index file")?;
        
        // SAFETY: We assume the file won't be modified while we're reading it.
        // The mmap provides a zero-copy view into the file.
        let mmap = unsafe { Mmap::map(&file) }
            .context("Failed to memory-map SCIP index file")?;
        
        let index = scip::types::Index::parse_from_bytes(&mmap)
            .context("Failed to parse SCIP index protobuf")?;

        // ═══════════════════════════════════════════════════════════════════
        // Pass 1: Parallel Definition Collection
        // ═══════════════════════════════════════════════════════════════════
        
        // Thread-safe maps for parallel access
        let definitions_by_file: DashMap<String, Vec<DefinitionInfo>> = DashMap::new();
        let symbol_to_node: DashMap<String, usize> = DashMap::new();
        let node_counter = AtomicUsize::new(0);
        
        // Collect nodes in parallel (we'll sort them later)
        let node_data: DashMap<usize, CallGraphNode> = DashMap::new();

        index.documents.par_iter().for_each(|document| {
            let file_path = document.relative_path.clone();
            let mut file_defs: Vec<DefinitionInfo> = Vec::new();

            for occurrence in &document.occurrences {
                // Check if this is a Definition (bit 0 of symbol_roles)
                let is_definition = occurrence.symbol_roles & 1 != 0;
                
                if is_definition && !occurrence.symbol.is_empty() {
                    let range = parse_scip_range(&occurrence.range);
                    
                    // Atomically get or create node ID for this symbol
                    let node_id = *symbol_to_node
                        .entry(occurrence.symbol.clone())
                        .or_insert_with(|| {
                            let id = node_counter.fetch_add(1, Ordering::SeqCst);
                            let label = extract_label_from_symbol(&occurrence.symbol);
                            node_data.insert(id, CallGraphNode {
                                id: occurrence.symbol.clone(),
                                callees: Vec::new(),
                                label: Some(label),
                            });
                            id
                        });

                    // We don't use node_id here directly, just ensure it's registered
                    let _ = node_id;

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
        });

        let def_count = node_counter.load(Ordering::SeqCst);
        println!("[SCIP Ingest] Found {} definitions (parallel)", def_count);

        // ═══════════════════════════════════════════════════════════════════
        // Pass 2: Parallel Reference Resolution
        // ═══════════════════════════════════════════════════════════════════
        
        let edge_counter = AtomicUsize::new(0);

        index.documents.par_iter().for_each(|document| {
            let file_path = &document.relative_path;
            
            // Get definitions for this file (if any)
            let file_defs = definitions_by_file
                .get(file_path)
                .map(|r| r.clone())
                .unwrap_or_default();

            for occurrence in &document.occurrences {
                // Check if this is a Reference (not a definition)
                let is_definition = occurrence.symbol_roles & 1 != 0;
                
                if !is_definition && !occurrence.symbol.is_empty() {
                    let ref_range = parse_scip_range(&occurrence.range);
                    let callee_symbol = &occurrence.symbol;

                    // Find the enclosing definition (the caller)
                    for def in &file_defs {
                        if def.range.contains(&ref_range) {
                            let caller_symbol = &def.symbol;
                            
                            // Add edge: caller -> callee
                            if let Some(caller_idx) = symbol_to_node.get(caller_symbol) {
                                // Avoid self-references
                                if caller_symbol != callee_symbol {
                                    // Thread-safe edge insertion
                                    if let Some(mut node) = node_data.get_mut(&*caller_idx) {
                                        if !node.callees.contains(callee_symbol) {
                                            node.callees.push(callee_symbol.clone());
                                            edge_counter.fetch_add(1, Ordering::Relaxed);
                                        }
                                    }
                                }
                            }
                            break; // Found the innermost enclosing definition
                        }
                    }
                }
            }
        });

        let edge_count = edge_counter.load(Ordering::Relaxed);
        println!("[SCIP Ingest] Created {} edges (parallel)", edge_count);

        // ═══════════════════════════════════════════════════════════════════
        // Finalize: Convert DashMap to sorted Vec
        // ═══════════════════════════════════════════════════════════════════
        
        let mut nodes: Vec<CallGraphNode> = node_data
            .into_iter()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(_, node)| node)
            .collect();
        
        // Sort by ID for deterministic output
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

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
