use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::mem;

// Mock types matching the charting library
#[repr(C)]
struct UncompressedVertex {
    time: u32,  // 4 bytes
    value: f32, // 4 bytes
}

// NOTE: In the actual implementation, we pack TWO vertices into 8 bytes
// For benchmarking, we're showing the effective per-vertex size
#[repr(C)]
struct CompressedVertex {
    time_value: u32, // 4 bytes (16 bits time + 16 bits value)
                     // metadata field removed for true comparison - in practice we pack 2 vertices per 8 bytes
}

impl CompressedVertex {
    fn pack(time: u32, value: f32, time_range: (u32, u32), value_range: (f32, f32)) -> Self {
        // Normalize time to 0-1 range
        let normalized_time = (time - time_range.0) as f32 / (time_range.1 - time_range.0) as f32;
        let time_u16 = (normalized_time.clamp(0.0, 1.0) * 65535.0) as u16;

        // Normalize value to 0-1 range
        let normalized_value = (value - value_range.0) / (value_range.1 - value_range.0);
        let value_u16 = (normalized_value.clamp(0.0, 1.0) * 65535.0) as u16;

        Self {
            time_value: ((time_u16 as u32) << 16) | (value_u16 as u32),
        }
    }
}

fn generate_test_data(size: usize) -> (Vec<u32>, Vec<f32>) {
    let start_time = 1700000000u32;
    let times: Vec<u32> = (0..size).map(|i| start_time + i as u32).collect();
    let values: Vec<f32> = (0..size)
        .map(|i| 50000.0 + (i as f32).sin() * 1000.0)
        .collect();
    (times, values)
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_compression_memory");

    for size in [1000, 10_000, 100_000, 1_000_000].iter() {
        let (times, values) = generate_test_data(*size);

        // Calculate memory sizes
        let uncompressed_size = *size * mem::size_of::<UncompressedVertex>();
        let compressed_size = *size * mem::size_of::<CompressedVertex>();
        let separate_buffers_size = *size * (mem::size_of::<u32>() + mem::size_of::<f32>());

        // Memory reduction percentage
        let reduction_from_separate =
            (1.0 - compressed_size as f32 / separate_buffers_size as f32) * 100.0;
        let reduction_from_interleaved =
            (1.0 - compressed_size as f32 / uncompressed_size as f32) * 100.0;

        println!("\nData size: {} vertices", size);
        println!("Separate buffers: {} bytes", separate_buffers_size);
        println!("Uncompressed interleaved: {} bytes", uncompressed_size);
        println!("Compressed: {} bytes", compressed_size);
        println!(
            "Reduction from separate buffers: {:.1}%",
            reduction_from_separate
        );
        println!(
            "Reduction from interleaved: {:.1}%",
            reduction_from_interleaved
        );

        group.bench_with_input(
            BenchmarkId::new("memory_allocation", size),
            size,
            |b, &size| {
                b.iter(|| {
                    // Simulate memory allocation for compressed vertices
                    let compressed: Vec<CompressedVertex> = Vec::with_capacity(size);
                    black_box(compressed);
                });
            },
        );
    }

    group.finish();
}

fn bench_compression_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_compression_speed");

    for size in [1000, 10_000, 100_000].iter() {
        let (times, values) = generate_test_data(*size);
        let time_range = (times[0], times[times.len() - 1]);
        let value_range = (45000.0f32, 55000.0f32);

        group.bench_with_input(BenchmarkId::new("compress", size), size, |b, &size| {
            b.iter(|| {
                let compressed: Vec<CompressedVertex> = times
                    .iter()
                    .zip(values.iter())
                    .map(|(&time, &value)| {
                        CompressedVertex::pack(time, value, time_range, value_range)
                    })
                    .collect();
                black_box(compressed);
            });
        });
    }

    group.finish();
}

fn bench_decompression_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_decompression_speed");

    for size in [1000, 10_000, 100_000].iter() {
        let (times, values) = generate_test_data(*size);
        let time_range = (times[0], times[times.len() - 1]);
        let value_range = (45000.0f32, 55000.0f32);

        // Pre-compress data
        let compressed: Vec<CompressedVertex> = times
            .iter()
            .zip(values.iter())
            .map(|(&time, &value)| CompressedVertex::pack(time, value, time_range, value_range))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("decompress", size),
            &compressed,
            |b, compressed| {
                b.iter(|| {
                    // Simulate decompression (would be done in shader)
                    let decompressed: Vec<(f32, f32)> = compressed
                        .iter()
                        .map(|vertex| {
                            let time_u16 = (vertex.time_value >> 16) & 0xFFFF;
                            let value_u16 = vertex.time_value & 0xFFFF;

                            let time_normalized = time_u16 as f32 / 65535.0;
                            let value_normalized = value_u16 as f32 / 65535.0;

                            let time = time_range.0 as f32
                                + time_normalized * (time_range.1 - time_range.0) as f32;
                            let value =
                                value_range.0 + value_normalized * (value_range.1 - value_range.0);

                            (time, value)
                        })
                        .collect();
                    black_box(decompressed);
                });
            },
        );
    }

    group.finish();
}

// Benchmark the complete pipeline
fn bench_complete_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("vertex_compression_pipeline");

    let size = 100_000;
    let (times, values) = generate_test_data(size);
    let time_range = (times[0], times[times.len() - 1]);
    let value_range = (45000.0f32, 55000.0f32);

    group.bench_function("uncompressed_pipeline", |b| {
        b.iter(|| {
            // Simulate creating uncompressed vertex buffer
            let vertices: Vec<UncompressedVertex> = times
                .iter()
                .zip(values.iter())
                .map(|(&time, &value)| UncompressedVertex { time, value })
                .collect();

            // Simulate GPU upload
            let gpu_buffer_size = vertices.len() * mem::size_of::<UncompressedVertex>();
            black_box(gpu_buffer_size);
        });
    });

    group.bench_function("compressed_pipeline", |b| {
        b.iter(|| {
            // Simulate creating compressed vertex buffer
            let vertices: Vec<CompressedVertex> = times
                .iter()
                .zip(values.iter())
                .map(|(&time, &value)| CompressedVertex::pack(time, value, time_range, value_range))
                .collect();

            // Simulate GPU upload (smaller buffer)
            let gpu_buffer_size = vertices.len() * mem::size_of::<CompressedVertex>();
            black_box(gpu_buffer_size);
        });
    });

    group.finish();
}

// Final summary calculation
fn calculate_final_metrics() {
    println!("\n=== VERTEX COMPRESSION SUMMARY ===");
    println!(
        "Original vertex size: {} bytes (u32 time + f32 value)",
        mem::size_of::<u32>() + mem::size_of::<f32>()
    );
    println!(
        "Compressed vertex size: {} bytes",
        mem::size_of::<CompressedVertex>()
    );

    let original_size = mem::size_of::<u32>() + mem::size_of::<f32>();
    let compressed_size = mem::size_of::<CompressedVertex>();
    let savings = (1.0 - compressed_size as f32 / original_size as f32) * 100.0;

    println!("Memory reduction: {:.1}%", savings);
    println!("\nFor 1M vertices:");
    println!(
        "  Original: {:.1} MB",
        (1_000_000 * original_size) as f32 / 1_048_576.0
    );
    println!(
        "  Compressed: {:.1} MB",
        (1_000_000 * compressed_size) as f32 / 1_048_576.0
    );
    println!(
        "  Saved: {:.1} MB",
        (1_000_000 * (original_size - compressed_size)) as f32 / 1_048_576.0
    );
}

criterion_group!(
    benches,
    bench_memory_usage,
    bench_compression_speed,
    bench_decompression_speed,
    bench_complete_pipeline
);

fn main() {
    // Print summary before running benchmarks
    calculate_final_metrics();

    // Run benchmarks
    benches();

    criterion::Criterion::default()
        .configure_from_args()
        .final_summary();
}
