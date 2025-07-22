use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Naive O(n) culling - checks every data point
fn naive_culling(timestamps: &[u32], viewport_start: u32, viewport_end: u32) -> (usize, usize) {
    let mut start_idx = None;
    let mut end_idx = None;

    // True naive approach - check every single data point
    for (i, &timestamp) in timestamps.iter().enumerate() {
        if timestamp >= viewport_start && timestamp <= viewport_end {
            if start_idx.is_none() {
                start_idx = Some(i);
            }
            end_idx = Some(i);
        }
    }

    (start_idx.unwrap_or(0), end_idx.unwrap_or(0))
}

/// Binary search culling - O(log n)
fn binary_search_culling(
    timestamps: &[u32],
    viewport_start: u32,
    viewport_end: u32,
) -> (usize, usize) {
    let len = timestamps.len();
    if len == 0 {
        return (0, 0);
    }

    // Binary search for start
    let mut left = 0;
    let mut right = len;

    while left < right {
        let mid = left + (right - left) / 2;
        if timestamps[mid] < viewport_start {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    let start_idx = left;

    // Binary search for end
    left = start_idx;
    right = len;

    while left < right {
        let mid = left + (right - left) / 2;
        if timestamps[mid] <= viewport_end {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    let end_idx = if left > 0 { left } else { 0 };

    (start_idx, end_idx)
}

fn culling_benchmark(c: &mut Criterion) {
    // Create test data - 1 million timestamps over 1 year
    let mut timestamps: Vec<u32> = Vec::with_capacity(1_000_000);
    let start_time = 1_600_000_000u32; // Sept 2020
    let interval = 31536; // ~31.5 seconds between points (1 year / 1M points)

    for i in 0..1_000_000 {
        timestamps.push(start_time + (i * interval));
    }

    // Test viewport in the middle 0.1% of data (1000 points visible)
    let viewport_start = start_time + (500_000 * interval);
    let viewport_end = viewport_start + (1000 * interval);

    let mut group = c.benchmark_group("culling");

    // Benchmark naive approach
    group.bench_function("naive_1M_points", |b| {
        b.iter(|| {
            naive_culling(
                black_box(&timestamps),
                black_box(viewport_start),
                black_box(viewport_end),
            )
        })
    });

    // Benchmark binary search approach
    group.bench_function("binary_search_1M_points", |b| {
        b.iter(|| {
            binary_search_culling(
                black_box(&timestamps),
                black_box(viewport_start),
                black_box(viewport_end),
            )
        })
    });

    // Test with different data sizes
    for size in [10_000, 100_000, 1_000_000, 10_000_000].iter() {
        let test_data: Vec<u32> = (0..*size).map(|i| start_time + (i * interval)).collect();

        group.bench_function(&format!("binary_search_{}_points", size), |b| {
            b.iter(|| {
                binary_search_culling(
                    black_box(&test_data),
                    black_box(viewport_start),
                    black_box(viewport_end),
                )
            })
        });
    }

    group.finish();
}

criterion_group!(benches, culling_benchmark);
criterion_main!(benches);
