/// Benchmarks for TraceCraft SCIP ingestion pipeline.
/// 
/// Run with: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::Path;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use protobuf::Message;

/// Create a synthetic SCIP index with configurable size.
fn create_synthetic_scip_index(num_documents: usize, defs_per_doc: usize, refs_per_doc: usize) -> scip::types::Index {
    let mut index = scip::types::Index::new();

    for doc_idx in 0..num_documents {
        let mut doc = scip::types::Document::new();
        doc.relative_path = format!("src/file_{}.rs", doc_idx);

        // Add definitions
        for def_idx in 0..defs_per_doc {
            let mut occ = scip::types::Occurrence::new();
            occ.symbol = format!("pkg::file_{}::func_{}", doc_idx, def_idx);
            let start_line = (def_idx * 20) as i32;
            occ.range = vec![start_line, 0, start_line + 15, 0];
            occ.symbol_roles = 1; // Definition bit
            doc.occurrences.push(occ);
        }

        // Add references
        for ref_idx in 0..refs_per_doc {
            let def_idx = ref_idx % defs_per_doc; // Reference inside a definition
            let mut occ = scip::types::Occurrence::new();
            // Reference to a function in a different document
            let target_doc = (doc_idx + 1) % num_documents;
            occ.symbol = format!("pkg::file_{}::func_0", target_doc);
            let start_line = (def_idx * 20 + 5) as i32;
            occ.range = vec![start_line, 5, 15]; // Inside the def_idx definition
            occ.symbol_roles = 0; // Reference (not a definition)
            doc.occurrences.push(occ);
        }

        index.documents.push(doc);
    }

    index
}

/// Write SCIP index to a temporary file.
fn write_scip_to_temp(index: &scip::types::Index) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bench.scip");
    let bytes = index.write_to_bytes().unwrap();
    let mut file = File::create(&path).unwrap();
    file.write_all(&bytes).unwrap();
    (dir, path)
}

fn bench_scip_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("scip_ingest");

    // Test with different sizes
    for num_docs in [10, 50, 100, 500].iter() {
        let defs = 10;
        let refs = 20;
        
        let index = create_synthetic_scip_index(*num_docs, defs, refs);
        let (_dir, path) = write_scip_to_temp(&index);

        group.bench_with_input(
            BenchmarkId::new("full_pipeline", format!("{}_docs", num_docs)),
            &path,
            |b, path| {
                b.iter(|| {
                    tracecraft::domain::scip_ingest::ScipIngestor::ingest_and_build_graph(
                        black_box(path)
                    ).unwrap()
                })
            },
        );
    }

    group.finish();
}

fn bench_scip_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("scip_load");

    // Medium-sized index
    let index = create_synthetic_scip_index(100, 20, 40);
    let (_dir, path) = write_scip_to_temp(&index);

    group.bench_function("protobuf_parse", |b| {
        b.iter(|| {
            use std::io::Read;
            let mut file = File::open(&path).unwrap();
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).unwrap();
            scip::types::Index::parse_from_bytes(black_box(&buffer)).unwrap()
        })
    });

    group.finish();
}

criterion_group!(benches, bench_scip_ingestion, bench_scip_loading);
criterion_main!(benches);
