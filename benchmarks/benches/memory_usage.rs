//! Memory usage and allocation benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gpu_charts_benchmarks::BenchmarkGpu;
use std::collections::VecDeque;

fn benchmark_buffer_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_pool");

    group.bench_function("pool_allocation", |b| {
        // Simulate buffer pool
        let mut pool: Vec<Vec<u8>> = Vec::new();
        let buffer_size = 1024 * 1024; // 1MB

        // Pre-allocate some buffers
        for _ in 0..10 {
            pool.push(vec![0u8; buffer_size]);
        }

        b.iter(|| {
            let buffer = if let Some(buf) = pool.pop() {
                buf
            } else {
                vec![0u8; buffer_size]
            };

            // Use buffer
            black_box(&buffer);

            // Return to pool
            pool.push(buffer);
        });
    });

    group.bench_function("pool_vs_allocation", |b| {
        let buffer_size = 1024 * 1024;
        let mut use_pool = true;
        let mut pool: Vec<Vec<u8>> = Vec::new();

        for _ in 0..5 {
            pool.push(vec![0u8; buffer_size]);
        }

        b.iter(|| {
            if use_pool {
                // Pool allocation
                let buffer = pool.pop().unwrap_or_else(|| vec![0u8; buffer_size]);
                black_box(&buffer);
                pool.push(buffer);
            } else {
                // Direct allocation
                let buffer = vec![0u8; buffer_size];
                black_box(&buffer);
            }
            use_pool = !use_pool;
        });
    });

    group.finish();
}

fn benchmark_memory_fragmentation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_fragmentation");

    group.bench_function("sequential_allocation", |b| {
        b.iter(|| {
            let mut buffers = Vec::new();

            // Allocate sequentially
            for i in 0..100 {
                buffers.push(vec![0u8; 1024 * (i + 1)]);
            }

            black_box(buffers)
        });
    });

    group.bench_function("interleaved_allocation", |b| {
        b.iter(|| {
            let mut small_buffers = Vec::new();
            let mut large_buffers = Vec::new();

            // Interleave small and large allocations
            for _i in 0..50 {
                small_buffers.push(vec![0u8; 1024]);
                large_buffers.push(vec![0u8; 1024 * 1024]);
            }

            black_box((small_buffers, large_buffers))
        });
    });

    group.finish();
}

fn benchmark_cache_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_memory");

    for size in [100, 1_000, 10_000].iter() {
        group.bench_with_input(BenchmarkId::new("lru_cache", size), size, |b, &size| {
            use std::collections::HashMap;

            b.iter(|| {
                let mut cache = HashMap::new();
                let mut lru_queue = VecDeque::new();
                let max_size = size / 2;

                for i in 0..size {
                    let key = format!("key_{}", i);
                    let value = vec![0u8; 1024]; // 1KB per entry

                    // LRU eviction
                    if cache.len() >= max_size {
                        if let Some(old_key) = lru_queue.pop_front() {
                            cache.remove(&old_key);
                        }
                    }

                    cache.insert(key.clone(), value);
                    lru_queue.push_back(key);
                }

                black_box(cache.len())
            });
        });
    }

    group.finish();
}

fn benchmark_gpu_memory_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu_memory_transfer");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let gpu = rt.block_on(BenchmarkGpu::new());

    for size in [1024, 1024 * 1024, 10 * 1024 * 1024].iter() {
        group.bench_with_input(BenchmarkId::new("cpu_to_gpu", size), size, |b, &size| {
            let data = vec![0u8; size];

            b.iter(|| {
                // Create and write to GPU buffer
                let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Transfer Buffer"),
                    size: size as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
                    mapped_at_creation: false,
                });

                gpu.queue.write_buffer(&buffer, 0, &data);
                gpu.queue.submit([]);

                black_box(buffer);
            });
        });
    }

    group.finish();
}

fn benchmark_memory_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure");
    group.sample_size(10); // Reduce sample size for memory-intensive tests

    group.bench_function("high_allocation_rate", |b| {
        b.iter(|| {
            let mut allocations = Vec::new();

            // Rapid allocation and deallocation
            for _ in 0..1000 {
                allocations.push(vec![0u8; 1024]);
                if allocations.len() > 100 {
                    allocations.remove(0);
                }
            }

            black_box(allocations)
        });
    });

    group.bench_function("memory_churn", |b| {
        b.iter(|| {
            let mut buffers: Vec<Vec<u8>> = Vec::new();

            // Simulate memory churn
            for i in 0..100 {
                if i % 2 == 0 {
                    buffers.push(vec![0u8; 1024 * 1024]);
                } else if !buffers.is_empty() {
                    buffers.swap_remove(buffers.len() / 2);
                }
            }

            black_box(buffers)
        });
    });

    group.finish();
}

fn benchmark_zero_copy(c: &mut Criterion) {
    let mut group = c.benchmark_group("zero_copy");

    group.bench_function("slice_reference", |b| {
        let large_buffer = vec![0u8; 10 * 1024 * 1024]; // 10MB

        b.iter(|| {
            // Zero-copy slice
            let slice = &large_buffer[1024..2048];
            black_box(slice)
        });
    });

    group.bench_function("data_copy", |b| {
        let large_buffer = vec![0u8; 10 * 1024 * 1024];

        b.iter(|| {
            // Actual copy
            let copy = large_buffer[1024..2048].to_vec();
            black_box(copy)
        });
    });

    group.finish();
}

fn benchmark_wgpu_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("wgpu_memory_patterns");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let gpu = rt.block_on(BenchmarkGpu::new());

    group.bench_function("buffer_reuse", |b| {
        let size = 1024 * 1024; // 1MB
        let buffer = gpu.create_test_buffer(size);
        let data = vec![0u8; size];

        b.iter(|| {
            // Reuse same buffer multiple times
            for _ in 0..10 {
                gpu.queue.write_buffer(&buffer, 0, &data);
            }
            gpu.queue.submit([]);
            black_box(&buffer);
        });
    });

    group.bench_function("mapped_buffer_creation", |b| {
        let size = 1024 * 1024;

        b.iter(|| {
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Mapped Buffer"),
                size: size as u64,
                usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            });

            {
                let mut view = buffer.slice(..).get_mapped_range_mut();
                view.fill(42);
            }
            buffer.unmap();

            black_box(buffer);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_buffer_pool,
    benchmark_memory_fragmentation,
    benchmark_cache_memory,
    benchmark_gpu_memory_transfer,
    benchmark_memory_pressure,
    benchmark_zero_copy,
    benchmark_wgpu_memory_patterns
);
criterion_main!(benches);
