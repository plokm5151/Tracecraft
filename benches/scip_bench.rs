/// Benchmarks for TraceCraft SCIP ingestion pipeline.
/// 
/// Run with: `cargo bench`
/// 
/// Phase 3.4: Comprehensive benchmark suite including:
/// - Full pipeline benchmarks at various scales
/// - Mmap loading vs traditional read comparison
/// - Parallel processing overhead measurement

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::fs::File;
use std::io::{Read, Write};
use tempfile::tempdir;
use protobuf::Message;
use memmap2::Mmap;

// ═══════════════════════════════════════════════════════════════════════════
// Synthetic Data Generators
// ═══════════════════════════════════════════════════════════════════════════

/// Create a synthetic SCIP index with configurable size.
fn create_synthetic_scip_index(
    num_documents: usize, 
    defs_per_doc: usize, 
    refs_per_doc: usize
) -> scip::types::Index {
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
            let def_idx = ref_idx % defs_per_doc;
            let mut occ = scip::types::Occurrence::new();
            let target_doc = (doc_idx + 1) % num_documents;
            occ.symbol = format!("pkg::file_{}::func_0", target_doc);
            let start_line = (def_idx * 20 + 5) as i32;
            occ.range = vec![start_line, 5, 15];
            occ.symbol_roles = 0; // Reference
            doc.occurrences.push(occ);
        }

        index.documents.push(doc);
    }

    index
}

/// Write SCIP index to a temporary file and return handle + path.
fn write_scip_to_temp(index: &scip::types::Index) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bench.scip");
    let bytes = index.write_to_bytes().unwrap();
    let mut file = File::create(&path).unwrap();
    file.write_all(&bytes).unwrap();
    (dir, path)
}

// ═══════════════════════════════════════════════════════════════════════════
// Full Pipeline Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

fn bench_scip_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("scip_ingest/full_pipeline");

    // Test with different document counts
    for num_docs in [10, 50, 100, 250, 500].iter() {
        let defs = 10;
        let refs = 20;
        
        let index = create_synthetic_scip_index(*num_docs, defs, refs);
        let (_dir, path) = write_scip_to_temp(&index);
        
        // Calculate total elements for throughput
        let total_defs = num_docs * defs;
        group.throughput(Throughput::Elements(total_defs as u64));

        group.bench_with_input(
            BenchmarkId::new("docs", num_docs),
            &path,
            |b, path| {
                b.iter(|| {
                    mr_hedgehog::domain::scip_ingest::ScipIngestor::ingest_and_build_graph(
                        black_box(path)
                    ).unwrap()
                })
            },
        );
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Mmap vs Traditional Read Comparison
// ═══════════════════════════════════════════════════════════════════════════

fn bench_mmap_vs_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("scip_load/mmap_vs_read");

    // Create a reasonably sized index for comparison
    let index = create_synthetic_scip_index(200, 30, 60);
    let (_dir, path) = write_scip_to_temp(&index);
    
    let file_size = std::fs::metadata(&path).unwrap().len();
    group.throughput(Throughput::Bytes(file_size));

    // Benchmark traditional file read
    group.bench_function("traditional_read", |b| {
        b.iter(|| {
            let mut file = File::open(&path).unwrap();
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).unwrap();
            scip::types::Index::parse_from_bytes(black_box(&buffer)).unwrap()
        })
    });

    // Benchmark memory-mapped read
    group.bench_function("mmap_read", |b| {
        b.iter(|| {
            let file = File::open(&path).unwrap();
            let mmap = unsafe { Mmap::map(&file) }.unwrap();
            scip::types::Index::parse_from_bytes(black_box(&mmap)).unwrap()
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Scaling Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("scip_ingest/scaling");
    group.sample_size(30); // Fewer samples for large tests

    // Scale by definition count per document
    for defs_per_doc in [5, 20, 50, 100].iter() {
        let num_docs = 50;
        let refs = *defs_per_doc * 2;
        
        let index = create_synthetic_scip_index(num_docs, *defs_per_doc, refs);
        let (_dir, path) = write_scip_to_temp(&index);

        group.bench_with_input(
            BenchmarkId::new("defs_per_doc", defs_per_doc),
            &path,
            |b, path| {
                b.iter(|| {
                    mr_hedgehog::domain::scip_ingest::ScipIngestor::ingest_and_build_graph(
                        black_box(path)
                    ).unwrap()
                })
            },
        );
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Reference Resolution Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

fn bench_reference_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("scip_ingest/references");
    group.sample_size(30);

    // Scale by reference count
    for refs_per_doc in [10, 50, 100, 200].iter() {
        let num_docs = 50;
        let defs = 20;
        
        let index = create_synthetic_scip_index(num_docs, defs, *refs_per_doc);
        let (_dir, path) = write_scip_to_temp(&index);

        group.bench_with_input(
            BenchmarkId::new("refs_per_doc", refs_per_doc),
            &path,
            |b, path| {
                b.iter(|| {
                    mr_hedgehog::domain::scip_ingest::ScipIngestor::ingest_and_build_graph(
                        black_box(path)
                    ).unwrap()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches, 
    bench_scip_full_pipeline, 
    bench_mmap_vs_read,
    bench_scaling,
    bench_reference_resolution
);
criterion_main!(benches);
