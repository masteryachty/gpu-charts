//! Stress tests for extreme scenarios

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gpu_charts_benchmarks::{data_generator, scenarios};
use std::time::Duration;

fn stress_test_billion_points(c: &mut Criterion) {
    let mut group = c.benchmark_group("billion_points");
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(10);

    group.bench_function("simulate_billion_point_render", |b| {
        b.iter(|| {
            // Simulate rendering 1 billion points with aggressive LOD
            let total_points = 1_000_000_000u64;
            let lod_levels = vec![
                (1_000_000_000, 100_000), // Show 100k when viewing all
                (100_000_000, 500_000),   // Show 500k when zoomed 10x
                (10_000_000, 1_000_000),  // Show 1M when zoomed 100x
                (1_000_000, 1_000_000),   // Show all when zoomed 1000x
            ];

            let mut total_rendered = 0u64;
            for (threshold, render_count) in &lod_levels {
                if total_points >= *threshold {
                    total_rendered = *render_count;
                    break;
                }
            }

            black_box(total_rendered)
        });
    });

    group.finish();
}

fn stress_test_memory_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_limits");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(5);

    group.bench_function("gpu_memory_exhaustion", |b| {
        b.iter(|| {
            let max_gpu_memory = 2u64 * 1024 * 1024 * 1024; // 2GB
            let buffer_size = 100 * 1024 * 1024; // 100MB buffers
            let mut allocated = 0;
            let mut buffer_count = 0;

            while allocated + buffer_size <= max_gpu_memory {
                allocated += buffer_size;
                buffer_count += 1;
            }

            black_box((buffer_count, allocated))
        });
    });

    group.bench_function("cache_thrashing", |b| {
        use std::collections::HashMap;

        b.iter(|| {
            let mut cache: HashMap<u64, Vec<u8>> = HashMap::new();
            let cache_limit = 500 * 1024 * 1024; // 500MB
            let mut total_size = 0;
            let mut evictions = 0;

            for i in 0..1000 {
                let entry_size = 10 * 1024 * 1024; // 10MB entries

                // Evict if needed
                while total_size + entry_size > cache_limit && !cache.is_empty() {
                    let oldest = *cache.keys().next().unwrap();
                    if let Some(data) = cache.remove(&oldest) {
                        total_size -= data.len();
                        evictions += 1;
                    }
                }

                // Add new entry
                cache.insert(i, vec![0u8; entry_size]);
                total_size += entry_size;
            }

            black_box((cache.len(), evictions))
        });
    });

    group.finish();
}

fn stress_test_concurrent_charts(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_charts");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(5);

    group.bench_function("50_concurrent_charts", |b| {
        b.iter(|| {
            let chart_count = 50;
            let points_per_chart = 100_000;
            let mut total_memory = 0;
            let mut total_vertices = 0u64;

            for i in 0..chart_count {
                let memory_per_point = 8; // 2 floats
                let chart_memory = points_per_chart * memory_per_point;
                total_memory += chart_memory;
                total_vertices += points_per_chart as u64;

                // Simulate viewport culling per chart
                let visible_ratio = 0.1; // 10% visible
                let rendered = (points_per_chart as f32 * visible_ratio) as u64;
                black_box((i, rendered));
            }

            black_box((total_memory, total_vertices))
        });
    });

    group.finish();
}

fn stress_test_worst_case_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("all_nan_data", |b| {
        b.iter(|| {
            let data = vec![f32::NAN; 1_000_000];
            let mut valid_count = 0;

            for &value in &data {
                if !value.is_nan() {
                    valid_count += 1;
                }
            }

            black_box(valid_count)
        });
    });

    group.bench_function("highly_fragmented_data", |b| {
        b.iter(|| {
            // Simulate highly fragmented time series
            let mut fragments = Vec::new();

            for i in 0..1000 {
                // Random gaps in data
                if i % 3 != 0 {
                    fragments.push((i * 100, i * 100 + 50)); // Start, end
                }
            }

            black_box(fragments.len())
        });
    });

    group.bench_function("extreme_zoom_range", |b| {
        b.iter(|| {
            // Test zoom from microseconds to years
            let zoom_levels = vec![
                0.000001,   // Microsecond view
                0.001,      // Millisecond view
                1.0,        // Second view
                3600.0,     // Hour view
                86400.0,    // Day view
                31536000.0, // Year view
            ];

            let base_data_points = 100_000_000;

            for &zoom in &zoom_levels {
                let visible_points = (base_data_points as f64 / zoom).min(1_000_000.0) as u64;
                black_box(visible_points);
            }
        });
    });

    group.finish();
}

fn stress_test_sustained_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_load");
    group.measurement_time(Duration::from_secs(120)); // 2 minute sustained test
    group.sample_size(3);

    group.bench_function("continuous_60fps_rendering", |b| {
        let frame_budget = Duration::from_micros(16_667); // 60 FPS

        b.iter(|| {
            let start = std::time::Instant::now();
            let mut frames = 0;
            let mut missed_frames = 0;

            // Simulate 1 second of rendering
            while start.elapsed() < Duration::from_secs(1) {
                let frame_start = std::time::Instant::now();

                // Simulate frame rendering
                let _data_points = 1_000_000;
                let mut sum = 0.0f32;
                for i in 0..1000 {
                    sum += (i as f32).sin();
                }
                black_box(sum);

                let frame_time = frame_start.elapsed();
                if frame_time > frame_budget {
                    missed_frames += 1;
                }

                frames += 1;
            }

            black_box((frames, missed_frames))
        });
    });

    group.finish();
}

fn stress_test_gpu_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_limits");
    group.sample_size(5);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let gpu = rt.block_on(BenchmarkGpu::new());

    group.bench_function("max_buffer_count", |b| {
        b.iter(|| {
            let mut buffers = Vec::new();
            let buffer_size = 1024 * 1024; // 1MB each

            // Try to create many buffers
            for i in 0..1000 {
                let buffer = gpu.create_test_buffer(buffer_size);
                buffers.push(buffer);

                // Stop if we're using too much memory
                if i * buffer_size > 500 * 1024 * 1024 {
                    break;
                }
            }

            black_box(buffers.len())
        });
    });

    group.bench_function("max_single_buffer", |b| {
        b.iter(|| {
            // Try different buffer sizes to find the limit
            let sizes = vec![
                10 * 1024 * 1024,  // 10MB
                100 * 1024 * 1024, // 100MB
                256 * 1024 * 1024, // 256MB
                512 * 1024 * 1024, // 512MB
            ];

            let mut max_successful = 0;
            for &size in &sizes {
                // Try to create buffer
                let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Large Buffer"),
                    size: size as u64,
                    usage: wgpu::BufferUsages::STORAGE,
                    mapped_at_creation: false,
                });
                max_successful = size;
                // In real scenario, we'd check if allocation succeeded
                black_box(buffer);
            }

            black_box(max_successful)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    stress_test_billion_points,
    stress_test_memory_limits,
    stress_test_concurrent_charts,
    stress_test_worst_case_scenarios,
    stress_test_sustained_load,
    stress_test_gpu_limits
);
criterion_main!(benches);
