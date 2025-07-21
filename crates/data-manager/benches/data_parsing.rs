//! Benchmarks for data parsing performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use gpu_charts_data::parser::{BinaryHeader, BinaryParser, ColumnType};

fn generate_test_data(rows: u32) -> Vec<u8> {
    let header_size = 100;
    let row_size = 8; // u32 + f32
    let total_size = header_size + (rows as usize * row_size);

    let mut data = vec![0u8; total_size];

    // Fill with dummy data
    for i in 0..rows as usize {
        let offset = header_size + i * row_size;
        // Write u32 timestamp
        data[offset..offset + 4].copy_from_slice(&(i as u32).to_le_bytes());
        // Write f32 value
        data[offset + 4..offset + 8].copy_from_slice(&(i as f32).to_le_bytes());
    }

    data
}

fn bench_parse_header(c: &mut Criterion) {
    let data = generate_test_data(1_000_000);

    c.bench_function("parse_header", |b| {
        b.iter(|| {
            let result = BinaryParser::parse_header(black_box(&data));
            black_box(result);
        });
    });
}

fn bench_validate_data(c: &mut Criterion) {
    let data_1m = generate_test_data(1_000_000);
    let data_10m = generate_test_data(10_000_000);
    let data_100m = generate_test_data(100_000_000);

    let header = BinaryHeader {
        columns: vec!["time".to_string(), "price".to_string()],
        row_count: 1_000_000,
        column_types: vec![ColumnType::U32, ColumnType::F32],
    };

    let mut group = c.benchmark_group("validate_data");
    group.throughput(Throughput::Bytes(data_1m.len() as u64));
    group.bench_function("1M_points", |b| {
        b.iter(|| {
            let result = BinaryParser::validate_data(black_box(&data_1m), black_box(&header), 100);
            black_box(result);
        });
    });

    group.throughput(Throughput::Bytes(data_10m.len() as u64));
    group.bench_function("10M_points", |b| {
        b.iter(|| {
            let header_10m = BinaryHeader {
                row_count: 10_000_000,
                ..header.clone()
            };
            let result =
                BinaryParser::validate_data(black_box(&data_10m), black_box(&header_10m), 100);
            black_box(result);
        });
    });

    group.throughput(Throughput::Bytes(data_100m.len() as u64));
    group.bench_function("100M_points", |b| {
        b.iter(|| {
            let header_100m = BinaryHeader {
                row_count: 100_000_000,
                ..header.clone()
            };
            let result =
                BinaryParser::validate_data(black_box(&data_100m), black_box(&header_100m), 100);
            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_parse_header, bench_validate_data);
criterion_main!(benches);
