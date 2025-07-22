//! Data loading and parsing benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gpu_charts_benchmarks::*;
use std::time::Duration;

fn benchmark_data_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_parsing");
    group.measurement_time(Duration::from_secs(10));

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.bench_with_input(BenchmarkId::new("binary_to_f32", size), size, |b, &size| {
            let mut gen = data_generator::DataGenerator::new(42);
            let binary_data = gen.generate_binary_data(size);

            b.iter(|| {
                let data = &binary_data;
                let mut result = Vec::with_capacity(size * 2);

                for chunk in data.chunks_exact(8) {
                    let bytes: [u8; 4] = chunk[0..4].try_into().unwrap();
                    let time = f32::from_le_bytes(bytes);
                    let bytes: [u8; 4] = chunk[4..8].try_into().unwrap();
                    let value = f32::from_le_bytes(bytes);
                    result.push(time);
                    result.push(value);
                }

                black_box(result)
            });
        });

        group.bench_with_input(
            BenchmarkId::new("direct_gpu_buffer", size),
            size,
            |b, &size| {
                let mut gen = data_generator::DataGenerator::new(42);
                let binary_data = gen.generate_binary_data(size);

                b.iter(|| {
                    // Simulate direct binary to GPU buffer conversion
                    let data = &binary_data;
                    let gpu_data: Vec<f32> = data
                        .chunks_exact(4)
                        .map(|chunk| {
                            let bytes: [u8; 4] = chunk.try_into().unwrap();
                            f32::from_le_bytes(bytes)
                        })
                        .collect();

                    black_box(gpu_data)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_data_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_aggregation");

    for size in [10_000, 100_000, 1_000_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("ohlc_aggregation", size),
            size,
            |b, &size| {
                let mut gen = data_generator::DataGenerator::new(42);
                let data = gen.generate_line_data(size);
                let bucket_size = 100;

                b.iter(|| {
                    let mut ohlc_data = Vec::with_capacity(size / bucket_size);

                    for chunk in data.chunks(bucket_size) {
                        if chunk.is_empty() {
                            continue;
                        }

                        let open = chunk[0][1];
                        let close = chunk[chunk.len() - 1][1];
                        let (high, low) = chunk
                            .iter()
                            .fold((f32::MIN, f32::MAX), |(h, l), &[_, v]| (h.max(v), l.min(v)));

                        ohlc_data.push([chunk[0][0], open, high, low, close]);
                    }

                    black_box(ohlc_data)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("min_max_calculation", size),
            size,
            |b, &size| {
                let mut gen = data_generator::DataGenerator::new(42);
                let data = gen.generate_gpu_buffer_data(size);

                b.iter(|| {
                    let (min, max) = data
                        .chunks(2)
                        .map(|chunk| chunk[1])
                        .fold((f32::MAX, f32::MIN), |(min, max), v| {
                            (min.min(v), max.max(v))
                        });

                    black_box((min, max))
                });
            },
        );
    }

    group.finish();
}

fn benchmark_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    // Simulate cache with HashMap
    use std::collections::HashMap;

    group.bench_function("cache_lookup_hit", |b| {
        let mut cache = HashMap::new();
        for i in 0..1000 {
            cache.insert(format!("key_{}", i), vec![0u8; 1024 * 1024]); // 1MB entries
        }

        b.iter(|| {
            let key = "key_500";
            black_box(cache.get(key))
        });
    });

    group.bench_function("cache_lookup_miss", |b| {
        let mut cache = HashMap::new();
        for i in 0..1000 {
            cache.insert(format!("key_{}", i), vec![0u8; 1024 * 1024]);
        }

        b.iter(|| {
            let key = "key_9999";
            black_box(cache.get(key))
        });
    });

    group.bench_function("cache_eviction_lru", |b| {
        let mut cache = HashMap::new();
        let mut lru_order = Vec::new();

        for i in 0..100 {
            let key = format!("key_{}", i);
            cache.insert(key.clone(), vec![0u8; 1024 * 1024]);
            lru_order.push(key);
        }

        b.iter(|| {
            if cache.len() >= 100 {
                let oldest = &lru_order[0];
                cache.remove(oldest);
                lru_order.remove(0);
            }

            let new_key = format!("key_{}", cache.len());
            cache.insert(new_key.clone(), vec![0u8; 1024 * 1024]);
            lru_order.push(new_key);
        });
    });

    group.finish();
}

fn benchmark_data_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_validation");

    for size in [10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("validate_no_copy", size),
            size,
            |b, &size| {
                let mut gen = data_generator::DataGenerator::new(42);
                let data = gen.generate_gpu_buffer_data(size);

                b.iter(|| {
                    let mut valid = true;
                    let mut nan_count = 0;
                    let mut inf_count = 0;

                    for &value in data.iter() {
                        if value.is_nan() {
                            nan_count += 1;
                        } else if value.is_infinite() {
                            inf_count += 1;
                        }
                    }

                    if nan_count > size / 100 || inf_count > size / 100 {
                        valid = false;
                    }

                    black_box((valid, nan_count, inf_count))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_data_parsing,
    benchmark_data_aggregation,
    benchmark_cache_operations,
    benchmark_data_validation
);
criterion_main!(benches);
