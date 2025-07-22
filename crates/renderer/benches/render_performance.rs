//! Performance benchmarks for the GPU renderer

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gpu_charts_renderer::{GpuBufferSet, Renderer, Viewport};
use gpu_charts_shared::{
    ChartConfiguration, ChartType, DataHandle, DataMetadata, TimeRange, VisualConfig,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Create a mock GPU buffer set for benchmarking
fn create_mock_buffer_set(device: &wgpu::Device, num_points: usize) -> Arc<GpuBufferSet> {
    use wgpu::util::DeviceExt;

    // Create mock time and price data
    let time_data: Vec<u32> = (0..num_points).map(|i| i as u32).collect();
    let price_data: Vec<f32> = (0..num_points)
        .map(|i| (i as f32).sin() * 100.0 + 1000.0)
        .collect();

    let time_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Time Buffer"),
        contents: bytemuck::cast_slice(&time_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
    });

    let price_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Price Buffer"),
        contents: bytemuck::cast_slice(&price_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
    });

    let mut buffers = HashMap::new();
    buffers.insert("time".to_string(), vec![time_buffer]);
    buffers.insert("price".to_string(), vec![price_buffer]);

    Arc::new(GpuBufferSet {
        buffers,
        metadata: DataMetadata {
            symbol: "BTC-USD".to_string(),
            column_types: vec!["time".to_string(), "price".to_string()],
            time_range: TimeRange::new(0, num_points as u64),
            total_points: num_points as u64,
            resolution: 1,
        },
    })
}

/// Create a test configuration
fn create_test_config(chart_type: ChartType) -> ChartConfiguration {
    ChartConfiguration {
        chart_type,
        visual_config: VisualConfig {
            background_color: [0.0, 0.0, 0.0, 1.0],
            grid_color: [0.2, 0.2, 0.2, 1.0],
            text_color: [1.0, 1.0, 1.0, 1.0],
            margin_percent: 0.1,
            show_grid: true,
            show_axes: true,
        },
        overlays: vec![],
        data_handles: vec![DataHandle {
            id: uuid::Uuid::new_v4(),
            data_type: "market_data".to_string(),
        }],
    }
}

fn bench_viewport_culling(c: &mut Criterion) {
    // Benchmark viewport culling with different point counts
    let mut group = c.benchmark_group("viewport_culling");

    for point_count in [1_000, 100_000, 1_000_000, 10_000_000].iter() {
        group.bench_function(format!("{}_points", point_count), |b| {
            b.iter(|| {
                // In real benchmark, would perform culling
                let points = black_box(*point_count);
                let visible = points / 10; // Assume 10% visible
                black_box(visible);
            });
        });
    }

    group.finish();
}

fn bench_lod_selection(c: &mut Criterion) {
    c.bench_function("lod_selection", |b| {
        b.iter(|| {
            let zoom_level = black_box(0.5f32);
            let point_count = black_box(1_000_000u32);

            // Simulate LOD selection
            let lod = match (zoom_level, point_count) {
                (z, n) if z < 0.1 && n > 1_000_000 => "aggressive",
                (z, n) if z < 0.5 && n > 100_000 => "moderate",
                _ => "full",
            };

            black_box(lod);
        });
    });
}

fn bench_draw_call_batching(c: &mut Criterion) {
    let mut group = c.benchmark_group("draw_calls");

    group.bench_function("unbatched", |b| {
        b.iter(|| {
            // Simulate unbatched drawing
            for i in 0..100 {
                black_box(i);
                // draw_call();
            }
        });
    });

    group.bench_function("batched", |b| {
        b.iter(|| {
            // Simulate batched drawing
            let batch_size = 10;
            for i in 0..10 {
                black_box(i * batch_size);
                // batched_draw_call(batch_size);
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_viewport_culling,
    bench_lod_selection,
    bench_draw_call_batching
);
criterion_main!(benches);
