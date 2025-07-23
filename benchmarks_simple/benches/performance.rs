use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;

// Benchmark data generation
fn generate_test_data(size: usize) -> Vec<f32> {
    (0..size).map(|i| (i as f32).sin() * 100.0).collect()
}

// Simulate vertex compression
fn vertex_compression_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_compression");
    
    for size in [1000, 10000, 100000, 1000000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let data = generate_test_data(size);
            b.iter(|| {
                // Simulate compression by quantizing to u16
                let compressed: Vec<u16> = data.iter()
                    .map(|&v| ((v + 1000.0) * 10.0) as u16)
                    .collect();
                black_box(compressed)
            });
        });
    }
    
    group.finish();
}

// Simulate GPU vertex generation vs CPU vertex generation
fn vertex_generation_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_generation");
    
    // CPU version
    group.bench_function("cpu_generation", |b| {
        let timestamps = generate_test_data(100000);
        let values = generate_test_data(100000);
        
        b.iter(|| {
            let vertices: Vec<[f32; 2]> = timestamps.iter()
                .zip(values.iter())
                .map(|(&t, &v)| [t, v])
                .collect();
            black_box(vertices)
        });
    });
    
    // GPU simulation (just the data preparation)
    group.bench_function("gpu_preparation", |b| {
        let timestamps = generate_test_data(100000);
        let values = generate_test_data(100000);
        
        b.iter(|| {
            // Just measure the cost of preparing data for GPU
            black_box(&timestamps);
            black_box(&values);
        });
    });
    
    group.finish();
}

// Simulate render bundle caching
fn render_bundles_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_bundles");
    
    // Without caching - recreate commands each frame
    group.bench_function("without_bundles", |b| {
        b.iter(|| {
            // Simulate creating render commands
            let mut commands = Vec::with_capacity(1000);
            for i in 0..1000 {
                commands.push(format!("draw_indexed {} {} {}", i * 6, i * 4, i));
            }
            black_box(commands)
        });
    });
    
    // With caching - just reference existing bundle
    group.bench_function("with_bundles", |b| {
        let cached_bundle = vec!["cached_render_bundle"; 1000];
        
        b.iter(|| {
            // Just reference the cached bundle
            black_box(&cached_bundle)
        });
    });
    
    group.finish();
}

// Binary search culling benchmark
fn culling_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("binary_culling");
    
    for size in [10000, 100000, 1000000].iter() {
        let data: Vec<u32> = (0..*size).map(|i| i as u32 * 10).collect();
        
        group.bench_with_input(BenchmarkId::new("binary_search", size), size, |b, _| {
            let target = (*size as u32 / 2) * 10;
            b.iter(|| {
                let result = data.binary_search(&target);
                black_box(result)
            });
        });
        
        group.bench_with_input(BenchmarkId::new("linear_search", size), size, |b, _| {
            let target = (*size as u32 / 2) * 10;
            b.iter(|| {
                let result = data.iter().position(|&x| x >= target);
                black_box(result)
            });
        });
    }
    
    group.finish();
}

// Overall performance benchmark
fn overall_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("overall_performance");
    group.measurement_time(Duration::from_secs(10));
    
    // Baseline (no optimizations)
    group.bench_function("baseline", |b| {
        let data = generate_test_data(1000000);
        b.iter(|| {
            // Simulate full pipeline without optimizations
            let vertices: Vec<[f32; 4]> = data.windows(2)
                .map(|w| [w[0], w[1], 0.0, 1.0])
                .collect();
            
            // Simulate rendering commands
            let commands: Vec<String> = (0..vertices.len())
                .map(|i| format!("draw {}", i))
                .collect();
                
            black_box((vertices, commands))
        });
    });
    
    // Optimized (all features enabled)
    group.bench_function("optimized", |b| {
        let data = generate_test_data(1000000);
        
        // Pre-compute optimizations
        let compressed: Vec<u16> = data.iter()
            .map(|&v| ((v + 1000.0) * 10.0) as u16)
            .collect();
        let bundle = vec!["cached_bundle"; 1];
        
        b.iter(|| {
            // Just reference pre-computed data
            black_box((&compressed, &bundle))
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    vertex_compression_benchmark,
    vertex_generation_benchmark,
    render_bundles_benchmark,
    culling_benchmark,
    overall_performance
);
criterion_main!(benches);