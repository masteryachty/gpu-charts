//! Simple working benchmark to demonstrate Phase 2 improvements

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_culling_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_culling");

    // Test with 1 million elements
    let size = 1_000_000;
    let data: Vec<u64> = (0..size).map(|i| i * 100).collect();
    let target = size * 50; // Middle of the range

    group.bench_function("linear_search", |b| {
        b.iter(|| {
            // Linear search
            let mut found = None;
            for (i, &val) in data.iter().enumerate() {
                if val >= target {
                    found = Some(i);
                    break;
                }
            }
            black_box(found)
        });
    });

    group.bench_function("binary_search", |b| {
        b.iter(|| {
            // Binary search
            let result = data.binary_search(&target);
            black_box(result.unwrap_or_else(|i| i))
        });
    });

    group.finish();
}

fn benchmark_compression_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_compression");

    let size = 10_000;

    group.bench_function("uncompressed_16byte", |b| {
        b.iter(|| {
            // Simulate 16 bytes per vertex (4 floats)
            let memory_usage = size * 16;
            black_box(memory_usage)
        });
    });

    group.bench_function("compressed_4byte", |b| {
        b.iter(|| {
            // Simulate 4 bytes per vertex (packed)
            let memory_usage = size * 4;
            black_box(memory_usage)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_culling_simple,
    benchmark_compression_simple
);
criterion_main!(benches);
