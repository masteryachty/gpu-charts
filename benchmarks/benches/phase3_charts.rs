//! Phase 3 New Chart Types benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rand::prelude::*;
use std::sync::Arc;

/// Mock data point structures
#[derive(Clone, Copy)]
struct Point2D {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct Point3D {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy)]
struct HeatmapCell {
    x: u32,
    y: u32,
    value: f32,
}

/// Benchmark scatter plot rendering
fn benchmark_scatter_plot(c: &mut Criterion) {
    let mut group = c.benchmark_group("scatter_plot");

    for &point_count in &[1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(point_count as u64));

        // Generate random points
        let points = generate_scatter_points(point_count);

        // Benchmark point cloud vertex generation
        group.bench_with_input(
            BenchmarkId::new("vertex_generation", point_count),
            &points,
            |b, points| {
                b.iter(|| {
                    let mut vertices = Vec::with_capacity(points.len() * 6);
                    for point in points {
                        // Generate quad vertices for each point
                        let size = 2.0;
                        vertices.push(point.x - size);
                        vertices.push(point.y - size);
                        vertices.push(point.x + size);
                        vertices.push(point.y - size);
                        vertices.push(point.x + size);
                        vertices.push(point.y + size);
                    }
                    black_box(vertices)
                });
            },
        );

        // Benchmark density-based clustering for large datasets
        if point_count >= 10_000 {
            group.bench_with_input(
                BenchmarkId::new("density_clustering", point_count),
                &points,
                |b, points| {
                    b.iter(|| perform_density_clustering(points, 100.0));
                },
            );
        }

        // Benchmark point selection/hit testing
        group.bench_with_input(
            BenchmarkId::new("hit_testing", point_count),
            &points,
            |b, points| {
                let cursor_x = 0.5;
                let cursor_y = 0.5;
                let radius = 0.05;

                b.iter(|| {
                    let mut selected = Vec::new();
                    for (i, point) in points.iter().enumerate() {
                        let dx = point.x - cursor_x;
                        let dy = point.y - cursor_y;
                        if dx * dx + dy * dy <= radius * radius {
                            selected.push(i);
                        }
                    }
                    black_box(selected)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark heatmap rendering
fn benchmark_heatmap(c: &mut Criterion) {
    let mut group = c.benchmark_group("heatmap");

    let resolutions = vec![
        ("64x64", 64, 64),
        ("256x256", 256, 256),
        ("512x512", 512, 512),
        ("1024x1024", 1024, 1024),
    ];

    for (name, width, height) in resolutions {
        group.throughput(Throughput::Elements((width * height) as u64));

        let cells = generate_heatmap_data(width, height);

        // Benchmark 2D density calculation
        group.bench_with_input(
            BenchmarkId::new("density_calculation", name),
            &cells,
            |b, cells| {
                b.iter(|| calculate_2d_density(cells, width, height));
            },
        );

        // Benchmark color mapping
        group.bench_with_input(
            BenchmarkId::new("color_mapping", name),
            &cells,
            |b, cells| {
                b.iter(|| {
                    let mut colors = Vec::with_capacity(cells.len() * 4);
                    for cell in cells {
                        let color = value_to_color(cell.value);
                        colors.extend_from_slice(&color);
                    }
                    black_box(colors)
                });
            },
        );

        // Benchmark smooth interpolation
        group.bench_with_input(
            BenchmarkId::new("bilinear_interpolation", name),
            &(width, height),
            |b, &(w, h)| {
                b.iter(|| perform_bilinear_interpolation(w, h));
            },
        );
    }

    group.finish();
}

/// Benchmark 3D chart rendering
fn benchmark_3d_charts(c: &mut Criterion) {
    let mut group = c.benchmark_group("3d_charts");

    for &point_count in &[1_000, 10_000, 50_000, 100_000] {
        group.throughput(Throughput::Elements(point_count as u64));

        let points = generate_3d_points(point_count);

        // Benchmark 3D transformation pipeline
        group.bench_with_input(
            BenchmarkId::new("transform_pipeline", point_count),
            &points,
            |b, points| {
                let view_matrix = create_view_matrix();
                let proj_matrix = create_projection_matrix();

                b.iter(|| {
                    let mut transformed = Vec::with_capacity(points.len());
                    for point in points {
                        let transformed_point =
                            transform_3d_point(*point, &view_matrix, &proj_matrix);
                        transformed.push(transformed_point);
                    }
                    black_box(transformed)
                });
            },
        );

        // Benchmark depth sorting for transparency
        group.bench_with_input(
            BenchmarkId::new("depth_sorting", point_count),
            &points,
            |b, points| {
                b.iter(|| {
                    let mut indexed_points: Vec<(usize, f32)> =
                        points.iter().enumerate().map(|(i, p)| (i, p.z)).collect();
                    indexed_points.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                    black_box(indexed_points)
                });
            },
        );

        // Benchmark camera controls (orbit calculation)
        group.bench_function(BenchmarkId::new("camera_orbit", point_count), |b| {
            let mut azimuth = 0.0;
            let mut elevation = 0.0;

            b.iter(|| {
                azimuth += 0.01;
                elevation += 0.005;
                let camera_pos = calculate_orbit_position(azimuth, elevation, 10.0);
                black_box(camera_pos)
            });
        });
    }

    group.finish();
}

// Helper functions
fn generate_scatter_points(count: usize) -> Vec<Point2D> {
    let mut rng = thread_rng();
    (0..count)
        .map(|_| Point2D {
            x: rng.gen_range(-1.0..1.0),
            y: rng.gen_range(-1.0..1.0),
        })
        .collect()
}

fn generate_3d_points(count: usize) -> Vec<Point3D> {
    let mut rng = thread_rng();
    (0..count)
        .map(|_| Point3D {
            x: rng.gen_range(-1.0..1.0),
            y: rng.gen_range(-1.0..1.0),
            z: rng.gen_range(-1.0..1.0),
        })
        .collect()
}

fn generate_heatmap_data(width: u32, height: u32) -> Vec<HeatmapCell> {
    let mut rng = thread_rng();
    let mut cells = Vec::with_capacity((width * height) as usize);

    for y in 0..height {
        for x in 0..width {
            cells.push(HeatmapCell {
                x,
                y,
                value: rng.gen_range(0.0..1.0),
            });
        }
    }

    cells
}

fn perform_density_clustering(points: &[Point2D], grid_size: f32) -> Vec<Vec<usize>> {
    // Simple grid-based clustering
    let mut clusters = std::collections::HashMap::new();

    for (i, point) in points.iter().enumerate() {
        let grid_x = (point.x / grid_size) as i32;
        let grid_y = (point.y / grid_size) as i32;
        clusters
            .entry((grid_x, grid_y))
            .or_insert(Vec::new())
            .push(i);
    }

    clusters.into_values().collect()
}

fn calculate_2d_density(cells: &[HeatmapCell], width: u32, height: u32) -> Vec<f32> {
    let mut density = vec![0.0; (width * height) as usize];
    let kernel_size = 3;

    for cell in cells {
        let idx = (cell.y * width + cell.x) as usize;

        // Apply Gaussian kernel
        for dy in -(kernel_size as i32)..=kernel_size as i32 {
            for dx in -(kernel_size as i32)..=kernel_size as i32 {
                let nx = cell.x as i32 + dx;
                let ny = cell.y as i32 + dy;

                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let nidx = (ny * width as i32 + nx) as usize;
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    let weight = (-dist * dist / (2.0 * kernel_size as f32)).exp();
                    density[nidx] += cell.value * weight;
                }
            }
        }
    }

    density
}

fn value_to_color(value: f32) -> [f32; 4] {
    // Simple gradient: blue -> green -> red
    let r = if value > 0.5 {
        (value - 0.5) * 2.0
    } else {
        0.0
    };
    let g = if value > 0.5 {
        1.0 - (value - 0.5) * 2.0
    } else {
        value * 2.0
    };
    let b = if value < 0.5 { 1.0 - value * 2.0 } else { 0.0 };
    [r, g, b, 1.0]
}

fn perform_bilinear_interpolation(width: u32, height: u32) -> Vec<f32> {
    let scale = 2;
    let new_width = width * scale;
    let new_height = height * scale;
    let mut interpolated = vec![0.0; (new_width * new_height) as usize];

    // Simple bilinear interpolation
    for y in 0..new_height {
        for x in 0..new_width {
            let sx = x as f32 / scale as f32;
            let sy = y as f32 / scale as f32;

            let x0 = sx.floor() as u32;
            let y0 = sy.floor() as u32;
            let x1 = (x0 + 1).min(width - 1);
            let y1 = (y0 + 1).min(height - 1);

            let fx = sx - x0 as f32;
            let fy = sy - y0 as f32;

            let idx = (y * new_width + x) as usize;
            interpolated[idx] = fx * fy;
        }
    }

    interpolated
}

fn create_view_matrix() -> [[f32; 4]; 4] {
    // Simple view matrix
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, -5.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn create_projection_matrix() -> [[f32; 4]; 4] {
    // Simple perspective projection
    let fov = 45.0_f32.to_radians();
    let aspect = 16.0 / 9.0;
    let near = 0.1;
    let far = 100.0;

    let f = 1.0 / (fov / 2.0).tan();

    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [
            0.0,
            0.0,
            (far + near) / (near - far),
            (2.0 * far * near) / (near - far),
        ],
        [0.0, 0.0, -1.0, 0.0],
    ]
}

fn transform_3d_point(point: Point3D, view: &[[f32; 4]; 4], proj: &[[f32; 4]; 4]) -> Point3D {
    // Transform point through view and projection matrices
    let mut result = Point3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    // Apply view matrix
    let vx = point.x * view[0][0] + point.y * view[1][0] + point.z * view[2][0] + view[3][0];
    let vy = point.x * view[0][1] + point.y * view[1][1] + point.z * view[2][1] + view[3][1];
    let vz = point.x * view[0][2] + point.y * view[1][2] + point.z * view[2][2] + view[3][2];

    // Apply projection matrix
    result.x = vx * proj[0][0] + vy * proj[1][0] + vz * proj[2][0];
    result.y = vx * proj[0][1] + vy * proj[1][1] + vz * proj[2][1];
    result.z = vx * proj[0][2] + vy * proj[1][2] + vz * proj[2][2];

    result
}

fn calculate_orbit_position(azimuth: f32, elevation: f32, distance: f32) -> Point3D {
    Point3D {
        x: distance * elevation.cos() * azimuth.sin(),
        y: distance * elevation.sin(),
        z: distance * elevation.cos() * azimuth.cos(),
    }
}

criterion_group!(
    chart_benches,
    benchmark_scatter_plot,
    benchmark_heatmap,
    benchmark_3d_charts
);
criterion_main!(chart_benches);
