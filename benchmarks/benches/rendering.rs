//! Rendering performance benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gpu_charts_benchmarks::*;
use std::sync::Arc;

async fn setup_gpu() -> BenchmarkGpu {
    BenchmarkGpu::new().await
}

fn benchmark_vertex_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_generation");
    let rt = tokio::runtime::Runtime::new().unwrap();

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.bench_with_input(BenchmarkId::new("line_vertices", size), size, |b, &size| {
            let mut gen = data_generator::DataGenerator::new(42);
            let data = gen.generate_line_data(size);

            b.iter(|| {
                let mut vertices = Vec::with_capacity(size * 2);

                for [x, y] in &data {
                    // Transform to screen coordinates
                    let screen_x = (x * 2.0) - 1.0;
                    let screen_y = (y * 2.0) - 1.0;
                    vertices.push([screen_x, screen_y]);
                }

                black_box(vertices)
            });
        });

        group.bench_with_input(
            BenchmarkId::new("candlestick_vertices", size),
            size,
            |b, &size| {
                let mut gen = data_generator::DataGenerator::new(42);
                let data = gen.generate_ohlc_data(size);

                b.iter(|| {
                    let mut vertices = Vec::with_capacity(size * 6 * 2); // 6 vertices per candle

                    for [time, open, high, low, close] in &data {
                        let x = (time * 2.0) - 1.0;
                        let width = 0.001; // Candle width

                        // Generate 6 vertices for rectangle (2 triangles)
                        vertices.push([x - width, *open]);
                        vertices.push([x + width, *open]);
                        vertices.push([x + width, *close]);
                        vertices.push([x - width, *close]);

                        // High/low wicks
                        vertices.push([x, *low]);
                        vertices.push([x, *high]);
                    }

                    black_box(vertices)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_culling(c: &mut Criterion) {
    let mut group = c.benchmark_group("viewport_culling");

    for size in [10_000, 100_000, 1_000_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("frustum_culling", size),
            size,
            |b, &size| {
                let mut gen = data_generator::DataGenerator::new(42);
                let data = gen.generate_line_data(size);
                let viewport = (0.25, 0.75); // View middle 50%

                b.iter(|| {
                    let visible: Vec<_> = data
                        .iter()
                        .filter(|&&[x, _]| x >= viewport.0 && x <= viewport.1)
                        .cloned()
                        .collect();

                    black_box(visible)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("binary_search_culling", size),
            size,
            |b, &size| {
                let mut gen = data_generator::DataGenerator::new(42);
                let mut data = gen.generate_line_data(size);
                data.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
                let viewport = (0.25, 0.75);

                b.iter(|| {
                    // Binary search for start
                    let start_idx = data
                        .binary_search_by(|point| point[0].partial_cmp(&viewport.0).unwrap())
                        .unwrap_or_else(|x| x);

                    // Binary search for end
                    let end_idx = data
                        .binary_search_by(|point| point[0].partial_cmp(&viewport.1).unwrap())
                        .unwrap_or_else(|x| x);

                    black_box(&data[start_idx..end_idx])
                });
            },
        );
    }

    group.finish();
}

fn benchmark_lod_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("lod_selection");

    group.bench_function("lod_decision", |b| {
        let zoom_levels = vec![0.01, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0];
        let point_counts = vec![1_000, 10_000, 100_000, 1_000_000];

        b.iter(|| {
            for &zoom in &zoom_levels {
                for &count in &point_counts {
                    let lod = match (zoom, count) {
                        (z, n) if z < 0.1 && n > 1_000_000 => "Aggressive",
                        (z, n) if z < 0.5 && n > 100_000 => "Moderate",
                        _ => "Full",
                    };
                    black_box(lod);
                }
            }
        });
    });

    group.bench_function("point_reduction", |b| {
        let mut gen = data_generator::DataGenerator::new(42);
        let data = gen.generate_line_data(100_000);
        let reduction_factor = 10;

        b.iter(|| {
            let reduced: Vec<_> = data.iter().step_by(reduction_factor).cloned().collect();

            black_box(reduced)
        });
    });

    group.finish();
}

fn benchmark_draw_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("draw_calls");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let gpu = rt.block_on(setup_gpu());

    group.bench_function("state_changes", |b| {
        struct MockPipeline {
            id: u32,
        }

        let pipelines: Vec<_> = (0..10).map(|i| MockPipeline { id: i }).collect();
        let mut current_pipeline = 0;

        b.iter(|| {
            // Simulate pipeline state changes
            for _ in 0..100 {
                let new_pipeline = (current_pipeline + 1) % pipelines.len();
                if new_pipeline != current_pipeline {
                    current_pipeline = new_pipeline;
                    black_box(&pipelines[current_pipeline]);
                }
            }
        });
    });

    group.bench_function("buffer_binding", |b| {
        let buffer_sizes = vec![1024, 4096, 16384, 65536];
        let mut current_size = 0;

        b.iter(|| {
            for _ in 0..100 {
                let new_size = buffer_sizes[(current_size + 1) % buffer_sizes.len()];
                if new_size != current_size {
                    current_size = new_size;
                    // Simulate buffer binding
                    black_box(vec![0u8; current_size]);
                }
            }
        });
    });

    group.finish();
}

fn benchmark_overlay_composition(c: &mut Criterion) {
    let mut group = c.benchmark_group("overlay_composition");

    group.bench_function("blend_overlays", |b| {
        let base_color = [1.0, 0.0, 0.0, 1.0];
        let overlay_colors = vec![
            [0.0, 1.0, 0.0, 0.5],
            [0.0, 0.0, 1.0, 0.3],
            [1.0, 1.0, 0.0, 0.7],
        ];

        b.iter(|| {
            let mut final_color = base_color;

            for overlay in &overlay_colors {
                // Alpha blending
                let alpha = overlay[3];
                final_color[0] = final_color[0] * (1.0 - alpha) + overlay[0] * alpha;
                final_color[1] = final_color[1] * (1.0 - alpha) + overlay[1] * alpha;
                final_color[2] = final_color[2] * (1.0 - alpha) + overlay[2] * alpha;
            }

            black_box(final_color)
        });
    });

    group.bench_function("multi_pass_rendering", |b| {
        let pass_count = 5;
        let vertices_per_pass = 10_000;

        b.iter(|| {
            let mut total_vertices = 0;

            for pass in 0..pass_count {
                // Simulate render pass
                total_vertices += vertices_per_pass;
                black_box(pass);
            }

            black_box(total_vertices)
        });
    });

    group.finish();
}

fn benchmark_gpu_shader_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_shader_execution");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let gpu = rt.block_on(setup_gpu());

    group.bench_function("vertex_shader_transform", |b| {
        let vertices = 100_000;

        b.iter(|| {
            // Simulate vertex shader transformation
            let mut transformed = Vec::with_capacity(vertices);
            for i in 0..vertices {
                let x = (i as f32) / vertices as f32;
                let y = x.sin();
                // MVP transformation
                let tx = x * 2.0 - 1.0;
                let ty = y * 2.0 - 1.0;
                transformed.push([tx, ty]);
            }
            black_box(transformed)
        });
    });

    group.bench_function("fragment_shader_color", |b| {
        let pixels = 1920 * 1080;

        b.iter(|| {
            // Simulate fragment shader color calculation
            let mut colors = Vec::with_capacity(pixels);
            for i in 0..pixels {
                let t = (i as f32) / pixels as f32;
                // Simple gradient
                colors.push([t, 1.0 - t, 0.5, 1.0]);
            }
            black_box(colors.len())
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_vertex_generation,
    benchmark_culling,
    benchmark_lod_selection,
    benchmark_draw_calls,
    benchmark_overlay_composition,
    benchmark_gpu_shader_execution
);
criterion_main!(benches);
