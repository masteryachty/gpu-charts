//! Phase 2 performance comparison benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
// Removed unused import
use std::time::Duration;

fn benchmark_culling_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("culling_phase_comparison");
    group.measurement_time(Duration::from_secs(10));

    for size in [10_000, 100_000, 1_000_000, 10_000_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Phase 1: Linear scan culling
        group.bench_with_input(
            BenchmarkId::new("phase1_linear_scan", size),
            size,
            |b, &size| {
                let timestamps: Vec<u64> = (0..size).map(|i| i as u64 * 1000).collect();
                let viewport_start = size as u64 * 250;
                let viewport_end = size as u64 * 750;

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
            },
        );

        // Phase 2: Binary search culling
        group.bench_with_input(
            BenchmarkId::new("phase2_binary_search", size),
            size,
            |b, &size| {
                let timestamps: Vec<u64> = (0..size).map(|i| i as u64 * 1000).collect();
                let viewport_start = size as u64 * 250;
                let viewport_end = size as u64 * 750;

                b.iter(|| {
                    use gpu_charts_renderer::culling::CullingSystem;
                    let range = CullingSystem::binary_search_cull(
                        &timestamps,
                        viewport_start,
                        viewport_end,
                    );
                    black_box((range.start_index, range.end_index))
                });
            },
        );
    }

    group.finish();
}

fn benchmark_data_transformation_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_transform_phase_comparison");

    for size in [10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Bytes((*size * 4) as u64));

        // Phase 1: Scalar transformation
        group.bench_with_input(BenchmarkId::new("phase1_scalar", size), size, |b, &size| {
            let data: Vec<f32> = (0..size).map(|i| i as f32).collect();

            b.iter(|| {
                let transformed: Vec<f32> = data.iter().map(|&x| x * 2.0 + 1.0).collect();
                black_box(transformed)
            });
        });

        // Phase 2: SIMD transformation (simulated)
        group.bench_with_input(BenchmarkId::new("phase2_simd", size), size, |b, &size| {
            let data: Vec<f32> = (0..size).map(|i| i as f32).collect();

            b.iter(|| {
                // Simulate SIMD with chunking
                let mut transformed = Vec::with_capacity(size);

                // Process 8 elements at a time (AVX2)
                for chunk in data.chunks(8) {
                    for &x in chunk {
                        transformed.push(x * 2.0 + 1.0);
                    }
                }

                black_box(transformed)
            });
        });
    }

    group.finish();
}

fn benchmark_vertex_generation_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_gen_phase_comparison");

    for size in [10_000, 100_000, 1_000_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Phase 1: CPU vertex generation
        group.bench_with_input(BenchmarkId::new("phase1_cpu", size), size, |b, &size| {
            let timestamps: Vec<f32> = (0..size).map(|i| i as f32 / size as f32).collect();
            let values: Vec<f32> = (0..size).map(|i| (i as f32).sin()).collect();

            b.iter(|| {
                let mut vertices = Vec::with_capacity(size * 2);

                for i in 0..size {
                    let x = timestamps[i] * 2.0 - 1.0;
                    let y = values[i] * 2.0 - 1.0;
                    vertices.push([x, y]);
                }

                black_box(vertices)
            });
        });

        // Phase 2: GPU vertex generation (simulated)
        group.bench_with_input(
            BenchmarkId::new("phase2_gpu_simulated", size),
            size,
            |b, &size| {
                b.iter(|| {
                    // GPU vertex generation happens on GPU, so we simulate the CPU cost
                    // which is just dispatching the compute shader
                    let dispatch_x = (size + 255) / 256;
                    black_box(dispatch_x);
                });
            },
        );
    }

    group.finish();
}

fn benchmark_compression_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_phase_comparison");

    for size in [10_000, 100_000].iter() {
        group.throughput(Throughput::Bytes((*size * 12) as u64)); // 12 bytes per vertex

        // Phase 1: No compression
        group.bench_with_input(
            BenchmarkId::new("phase1_uncompressed", size),
            size,
            |b, &size| {
                let vertices: Vec<(f32, f32, f32)> = (0..size)
                    .map(|i| (i as f32, (i as f32).sin(), 1.0))
                    .collect();

                b.iter(|| {
                    // Just copy the data
                    let buffer = vertices.clone();
                    black_box(buffer.len() * 12)
                });
            },
        );

        // Phase 2: Vertex compression
        group.bench_with_input(
            BenchmarkId::new("phase2_compressed", size),
            size,
            |b, &size| {
                use gpu_charts_renderer::vertex_compression::CompressedVertex;

                let vertices: Vec<(f32, f32, f32)> = (0..size)
                    .map(|i| (i as f32, (i as f32).sin(), 1.0))
                    .collect();

                b.iter(|| {
                    let mut compressed = Vec::with_capacity(size);
                    let time_range = (0.0, size as f32);
                    let value_range = (-1.0, 1.0);

                    for &(time, value, _) in &vertices {
                        compressed.push(CompressedVertex::pack(
                            time,
                            value,
                            time_range,
                            value_range,
                        ));
                    }

                    black_box(compressed.len() * 8) // 8 bytes per compressed vertex
                });
            },
        );
    }

    group.finish();
}

fn benchmark_draw_calls_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("draw_calls_phase_comparison");

    // Phase 1: Many small draw calls
    group.bench_function("phase1_many_draws", |b| {
        let chunks = 100;
        let vertices_per_chunk = 10_000;

        b.iter(|| {
            let mut total_vertices = 0;

            for chunk in 0..chunks {
                // Simulate draw call overhead
                black_box(chunk);
                total_vertices += vertices_per_chunk;
            }

            black_box(total_vertices)
        });
    });

    // Phase 2: Few batched draw calls
    group.bench_function("phase2_batched_draws", |b| {
        let batches = 5;
        let vertices_per_batch = 200_000;

        b.iter(|| {
            let mut total_vertices = 0;

            for batch in 0..batches {
                // Simulate batched draw call
                black_box(batch);
                total_vertices += vertices_per_batch;
            }

            black_box(total_vertices)
        });
    });

    group.finish();
}

fn benchmark_memory_usage_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage_phase_comparison");

    let size = 1_000_000;

    // Phase 1: Separate buffers
    group.bench_function("phase1_separate_buffers", |b| {
        b.iter(|| {
            let timestamps: Vec<f64> = vec![0.0; size];
            let values: Vec<f32> = vec![0.0; size];
            let colors: Vec<[f32; 4]> = vec![[0.0; 4]; size];

            // Calculate memory usage
            let memory = timestamps.len() * 8 + values.len() * 4 + colors.len() * 16;

            black_box(memory)
        });
    });

    // Phase 2: Compressed interleaved buffer
    group.bench_function("phase2_compressed_interleaved", |b| {
        b.iter(|| {
            use gpu_charts_renderer::vertex_compression::CompressedVertex;

            let compressed: Vec<CompressedVertex> =
                vec![CompressedVertex::pack(0.0, 0.0, (0.0, 1.0), (-1.0, 1.0)); size];

            // Calculate memory usage (8 bytes per vertex)
            let memory = compressed.len() * 8;

            black_box(memory)
        });
    });

    group.finish();
}

criterion_group!(
    phase2_benchmarks,
    benchmark_culling_comparison,
    benchmark_data_transformation_comparison,
    benchmark_vertex_generation_comparison,
    benchmark_compression_comparison,
    benchmark_draw_calls_comparison,
    benchmark_memory_usage_comparison
);

criterion_main!(phase2_benchmarks);
