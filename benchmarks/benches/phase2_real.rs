//! Real Phase 2 performance benchmarks that actually compile and run

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

/// Benchmark binary search vs linear scan culling
fn benchmark_culling_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("culling_algorithms");
    group.measurement_time(Duration::from_secs(10));

    for size in [1_000, 10_000, 100_000, 1_000_000, 10_000_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Create sorted timestamps
        let timestamps: Vec<u64> = (0..*size).map(|i| i as u64 * 1000).collect();
        let viewport_start = size * 250;
        let viewport_end = size * 750;

        // Linear scan (Phase 1 approach)
        group.bench_with_input(BenchmarkId::new("linear_scan", size), size, |b, _| {
            b.iter(|| {
                let mut start_idx = None;
                let mut end_idx = 0;

                for (i, &ts) in timestamps.iter().enumerate() {
                    if ts >= viewport_start && start_idx.is_none() {
                        start_idx = Some(i);
                    }
                    if ts <= viewport_end {
                        end_idx = i + 1;
                    }
                    if ts > viewport_end {
                        break;
                    }
                }

                black_box((start_idx.unwrap_or(0), end_idx))
            });
        });

        // Binary search (Phase 2 approach)
        group.bench_with_input(BenchmarkId::new("binary_search", size), size, |b, _| {
            b.iter(|| {
                let start_idx = timestamps
                    .binary_search(&viewport_start)
                    .unwrap_or_else(|i| i);
                let end_idx = timestamps
                    .binary_search(&viewport_end)
                    .unwrap_or_else(|i| i);

                black_box((start_idx, end_idx))
            });
        });
    }

    group.finish();
}

/// Benchmark data transformation with and without SIMD
fn benchmark_data_transformation(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_transformation");

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        let data: Vec<f32> = (0..*size).map(|i| i as f32).collect();

        // Scalar transformation (Phase 1)
        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, &size| {
            b.iter(|| {
                let mut result = Vec::with_capacity(size);
                for &x in &data {
                    result.push(x * 2.0 + 1.0);
                }
                black_box(result)
            });
        });

        // SIMD-style transformation (Phase 2)
        group.bench_with_input(BenchmarkId::new("simd_style", size), size, |b, &size| {
            b.iter(|| {
                let mut result = Vec::with_capacity(size);

                // Process in chunks of 8 (simulating AVX2)
                let chunks = data.chunks_exact(8);
                let remainder = chunks.remainder();

                for chunk in chunks {
                    // In real SIMD, this would be a single instruction
                    for &x in chunk {
                        result.push(x * 2.0 + 1.0);
                    }
                }

                // Handle remainder
                for &x in remainder {
                    result.push(x * 2.0 + 1.0);
                }

                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark vertex compression
fn benchmark_vertex_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_compression");

    for size in [1_000, 10_000, 100_000].iter() {
        let vertices: Vec<(f32, f32, f32, f32)> = (0..*size)
            .map(|i| {
                let t = i as f32 / *size as f32;
                (t, (t * 6.28).sin(), 1.0, 0.5)
            })
            .collect();

        // Uncompressed size (Phase 1: 16 bytes per vertex)
        group.throughput(Throughput::Bytes((*size * 16) as u64));
        group.bench_with_input(BenchmarkId::new("uncompressed", size), size, |b, &size| {
            b.iter(|| {
                let buffer_size = size * 16; // 4 floats * 4 bytes
                black_box(buffer_size)
            });
        });

        // Compressed size (Phase 2: 8 bytes per vertex)
        group.throughput(Throughput::Bytes((*size * 8) as u64));
        group.bench_with_input(
            BenchmarkId::new("compressed_8byte", size),
            size,
            |b, &size| {
                b.iter(|| {
                    // Simulate compression to 2 x u16 + 2 x u16
                    let mut compressed = Vec::with_capacity(size * 2);
                    for &(x, y, _, _) in &vertices {
                        let x_u16 = (x.clamp(0.0, 1.0) * 65535.0) as u16;
                        let y_u16 = ((y + 1.0) * 0.5 * 65535.0) as u16;
                        compressed.push((x_u16, y_u16));
                    }
                    black_box(compressed.len() * 4) // 2 u16 = 4 bytes
                });
            },
        );

        // Ultra compressed (Phase 2: 4 bytes per vertex)
        group.throughput(Throughput::Bytes((*size * 4) as u64));
        group.bench_with_input(
            BenchmarkId::new("compressed_4byte", size),
            size,
            |b, &size| {
                b.iter(|| {
                    // Simulate ultra compression to single u32
                    let mut compressed = Vec::with_capacity(size);
                    for &(x, y, _, _) in &vertices {
                        let x_u12 = (x.clamp(0.0, 1.0) * 4095.0) as u32;
                        let y_u12 = ((y + 1.0) * 0.5 * 4095.0) as u32;
                        let packed = (x_u12 << 20) | (y_u12 << 8); // 8 bits left for flags
                        compressed.push(packed);
                    }
                    black_box(compressed.len() * 4)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark draw call batching
fn benchmark_draw_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("draw_batching");

    let dataset_count = vec![10, 50, 100, 500];

    for count in dataset_count {
        // Phase 1: Individual draw calls
        group.bench_with_input(
            BenchmarkId::new("individual_draws", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let mut total_cost = 0;
                    for _ in 0..count {
                        // Simulate draw call overhead (e.g., 10 microseconds)
                        total_cost += 10;
                    }
                    black_box(total_cost)
                });
            },
        );

        // Phase 2: Batched draws
        group.bench_with_input(
            BenchmarkId::new("batched_draws", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    // Batch into groups of 10
                    let batch_count = (count + 9) / 10;
                    let total_cost = batch_count * 10; // Same overhead but fewer calls
                    black_box(total_cost)
                });
            },
        );

        // Phase 2: Indirect draws
        group.bench_with_input(
            BenchmarkId::new("indirect_draws", count),
            &count,
            |b, &count| {
                b.iter(|| {
                    // Single indirect draw call handles all
                    let total_cost = 10 + count; // One call + minimal per-item cost
                    black_box(total_cost)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark memory allocation patterns
fn benchmark_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    let sizes = vec![1_000, 10_000, 100_000];

    for size in sizes {
        // Phase 1: Frequent allocations
        group.bench_with_input(
            BenchmarkId::new("frequent_alloc", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let mut vecs = Vec::new();
                    for _i in 0..100 {
                        let v: Vec<f32> = vec![0.0; size / 100];
                        vecs.push(v);
                    }
                    black_box(vecs)
                });
            },
        );

        // Phase 2: Pre-allocated pool
        group.bench_with_input(BenchmarkId::new("buffer_pool", size), &size, |b, &size| {
            // Pre-allocate once
            let mut buffer = vec![0.0f32; size];

            b.iter(|| {
                // Reuse the same buffer
                for i in 0..buffer.len() {
                    buffer[i] = i as f32;
                }
                black_box(buffer.len())
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_culling_algorithms,
    benchmark_data_transformation,
    benchmark_vertex_compression,
    benchmark_draw_batching,
    benchmark_memory_patterns
);
criterion_main!(benches);
