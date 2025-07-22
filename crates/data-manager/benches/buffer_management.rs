//! Benchmarks for GPU buffer pool performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gpu_charts_data::buffer_pool::BufferPool;

fn bench_buffer_acquire_release(c: &mut Criterion) {
    // Note: These benchmarks would need a real wgpu device to run properly
    // For now, they're structured to show the benchmarking approach

    let mut group = c.benchmark_group("buffer_pool");

    group.bench_function("acquire_small", |b| {
        let mut pool = BufferPool::new(1024 * 1024 * 1024); // 1GB
        b.iter(|| {
            // In real benchmark, would acquire from pool
            let size = black_box(1024 * 1024); // 1MB
                                               // let buffer = pool.acquire(device, size);
            black_box(size);
        });
    });

    group.bench_function("acquire_large", |b| {
        let mut pool = BufferPool::new(1024 * 1024 * 1024); // 1GB
        b.iter(|| {
            // In real benchmark, would acquire from pool
            let size = black_box(128 * 1024 * 1024); // 128MB
                                                     // let buffer = pool.acquire(device, size);
            black_box(size);
        });
    });

    group.bench_function("acquire_release_cycle", |b| {
        let mut pool = BufferPool::new(1024 * 1024 * 1024); // 1GB
        b.iter(|| {
            // In real benchmark, would do full cycle
            let size = black_box(16 * 1024 * 1024); // 16MB
                                                    // let buffer = pool.acquire(device, size);
                                                    // pool.release(buffer);
            black_box(size);
        });
    });

    group.finish();
}

fn bench_cache_operations(c: &mut Criterion) {
    use gpu_charts_data::cache::{CacheKey, DataCache};
    use gpu_charts_shared::{DataRequest, TimeRange};

    let mut cache = DataCache::new(1024 * 1024 * 1024); // 1GB

    c.bench_function("cache_lookup", |b| {
        let request = DataRequest {
            symbol: "BTC-USD".to_string(),
            time_range: TimeRange::new(1000, 2000),
            columns: vec!["time".to_string(), "price".to_string()],
            aggregation: None,
            max_points: None,
        };
        let key = CacheKey::from_request(&request);

        b.iter(|| {
            let result = cache.get(black_box(&key));
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_buffer_acquire_release,
    bench_cache_operations
);
criterion_main!(benches);
