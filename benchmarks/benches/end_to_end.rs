//! End-to-end performance benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gpu_charts_benchmarks::{data_generator, metrics, scenarios, BenchmarkGpu, GpuTimer};
use gpu_charts_renderer::PerformanceMetrics;
use std::time::{Duration, Instant};

async fn benchmark_full_pipeline(points: usize) -> PerformanceMetrics {
    let mut metrics = PerformanceMetrics::default();
    let start = Instant::now();

    // Setup GPU
    let gpu = BenchmarkGpu::new().await;

    // 1. Data generation (simulating fetch)
    let fetch_start = Instant::now();
    let mut gen = data_generator::DataGenerator::new(42);
    let data = gen.generate_line_data(points);
    metrics.data_fetch_time = fetch_start.elapsed();

    // 2. Data parsing and GPU buffer creation
    let parse_start = Instant::now();
    let gpu_data: Vec<f32> = data.iter().flat_map(|&[x, y]| vec![x, y]).collect();

    // Create GPU buffer
    let buffer = gpu.create_buffer_with_data(&gpu_data);
    metrics.parse_time = parse_start.elapsed();

    // 3. Simulate rendering
    let render_start = Instant::now();

    // Create command encoder for GPU work
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Benchmark Encoder"),
        });

    // Simulate render pass
    {
        let texture_desc = wgpu::TextureDescriptor {
            label: Some("Benchmark Texture"),
            size: wgpu::Extent3d {
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let texture = gpu.device.create_texture(&texture_desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Benchmark Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    gpu.queue.submit(Some(encoder.finish()));
    gpu.device.poll(wgpu::Maintain::Wait);

    metrics.render_time = render_start.elapsed();

    // Update metrics
    metrics.draw_calls = 5;
    metrics.vertices_rendered = data.len() as u64;
    metrics.frame_time = start.elapsed();
    metrics.calculate_fps();
    metrics.calculate_throughput(data.len() as u64, gpu_data.len() as u64 * 4);

    metrics
}

fn benchmark_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end_scenarios");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(20);

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Small dataset scenario
    group.bench_function("small_dataset_1k", |b| {
        b.to_async(&rt).iter(|| async {
            let metrics = benchmark_full_pipeline(1_000).await;
            black_box(metrics)
        });
    });

    // Medium dataset scenario
    group.bench_function("medium_dataset_100k", |b| {
        b.to_async(&rt).iter(|| async {
            let metrics = benchmark_full_pipeline(100_000).await;
            black_box(metrics)
        });
    });

    // Large dataset scenario
    group.bench_function("large_dataset_1m", |b| {
        b.to_async(&rt).iter(|| async {
            let metrics = benchmark_full_pipeline(1_000_000).await;
            black_box(metrics)
        });
    });

    group.finish();
}

fn benchmark_interactive_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("interactive_scenarios");
    group.measurement_time(Duration::from_secs(20));

    // Rapid zoom scenario
    group.bench_function("rapid_zoom", |b| {
        let zoom_levels = scenarios::ScenarioRunner::simulate_zoom(1.5, 20);
        let mut gen = data_generator::DataGenerator::new(42);
        let base_data = gen.generate_line_data(100_000);

        b.iter(|| {
            for &zoom in &zoom_levels {
                // Simulate viewport calculation
                let viewport_start = 0.5 - (0.5 / zoom);
                let viewport_end = 0.5 + (0.5 / zoom);

                // Cull data to viewport
                let visible_data: Vec<_> = base_data
                    .iter()
                    .filter(|&&[x, _]| x >= viewport_start && x <= viewport_end)
                    .collect();

                black_box(visible_data.len());
            }
        });
    });

    // Continuous pan scenario
    group.bench_function("continuous_pan", |b| {
        let pan_positions = scenarios::ScenarioRunner::simulate_pan(1.0, Duration::from_secs(2));
        let mut gen = data_generator::DataGenerator::new(42);
        let base_data = gen.generate_line_data(100_000);

        b.iter(|| {
            for &offset in &pan_positions {
                // Simulate viewport with pan offset
                let viewport_start = offset;
                let viewport_end = offset + 0.5;

                let visible_data: Vec<_> = base_data
                    .iter()
                    .filter(|&&[x, _]| x >= viewport_start && x <= viewport_end)
                    .collect();

                black_box(visible_data.len());
            }
        });
    });

    group.finish();
}

fn benchmark_gpu_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_operations");
    group.sample_size(10);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let gpu = rt.block_on(BenchmarkGpu::new());

    // Buffer creation
    group.bench_function("gpu_buffer_creation_1mb", |b| {
        b.iter(|| {
            let buffer = gpu.create_test_buffer(1024 * 1024);
            black_box(buffer);
        });
    });

    // Buffer write
    group.bench_function("gpu_buffer_write_1mb", |b| {
        let data = vec![0.0f32; 256 * 1024]; // 1MB of f32
        b.iter(|| {
            let buffer = gpu.create_buffer_with_data(&data);
            black_box(buffer);
        });
    });

    // GPU timing test
    group.bench_function("gpu_timer_overhead", |b| {
        b.to_async(&rt).iter(|| async {
            let timer = GpuTimer::new(&gpu.device);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());

            timer.start(&mut encoder);
            // Minimal GPU work
            timer.end(&mut encoder);

            gpu.queue.submit(Some(encoder.finish()));
            let time = timer.read_time(&gpu.device, &gpu.queue).await;
            black_box(time)
        });
    });

    group.finish();
}

fn benchmark_stress_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("stress_scenarios");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    // Multiple charts scenario
    group.bench_function("multiple_charts_10x100k", |b| {
        b.iter(|| {
            let mut charts = Vec::new();
            let mut total_points = 0u64;

            // Simulate 10 charts with 100k points each
            for i in 0..10 {
                let mut gen = data_generator::DataGenerator::new(i as u64);
                let data = gen.generate_line_data(100_000);
                total_points += data.len() as u64;
                charts.push(data);
            }

            black_box((charts.len(), total_points))
        });
    });

    // Memory pressure scenario
    group.bench_function("memory_pressure", |b| {
        b.iter(|| {
            let mut allocations = Vec::new();
            let mut total_memory = 0usize;

            // Simulate high memory usage
            for i in 0..50 {
                let size = 1024 * 1024 * (i % 10 + 1); // 1-10 MB
                allocations.push(vec![0u8; size]);
                total_memory += size;

                // Simulate memory pressure - free some allocations
                if total_memory > 100 * 1024 * 1024 && !allocations.is_empty() {
                    let freed = allocations.remove(0);
                    total_memory -= freed.len();
                }
            }

            black_box((allocations.len(), total_memory))
        });
    });

    group.finish();
}

fn benchmark_performance_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_targets");
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Test against performance guide targets
    group.bench_function("meets_60fps_target", |b| {
        b.to_async(&rt).iter(|| async {
            let metrics = benchmark_full_pipeline(100_000).await;
            let targets = metrics::PerformanceTargets::default();

            let meets_target = metrics.meets_targets(&targets);
            black_box((meets_target, metrics.frame_time))
        });
    });

    // Viewport culling performance
    group.bench_function("viewport_culling_binary_search", |b| {
        let mut gen = data_generator::DataGenerator::new(42);
        let data = gen.generate_line_data(1_000_000);

        b.iter(|| {
            let viewport_start = 0.25;
            let viewport_end = 0.75;

            // Binary search for viewport bounds
            let start_idx = data
                .binary_search_by(|point| point[0].partial_cmp(&viewport_start).unwrap())
                .unwrap_or_else(|x| x);

            let end_idx = data
                .binary_search_by(|point| point[0].partial_cmp(&viewport_end).unwrap())
                .unwrap_or_else(|x| x);

            black_box(end_idx - start_idx)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_scenarios,
    benchmark_interactive_scenarios,
    benchmark_gpu_operations,
    benchmark_stress_scenarios,
    benchmark_performance_targets
);
criterion_main!(benches);
