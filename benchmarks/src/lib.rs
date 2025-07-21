//! GPU Charts benchmarking utilities

use gpu_charts_renderer::{GpuBufferSet, Viewport};
use gpu_charts_shared::{DataMetadata, TimeRange};
use std::sync::Arc;
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;

pub mod data_generator;
pub mod metrics;
pub mod optimized_benchmark;
pub mod scenarios;

/// Performance measurement result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub duration: Duration,
    pub iterations: u32,
    pub mean: Duration,
    pub std_dev: Duration,
    pub min: Duration,
    pub max: Duration,
    pub metrics: metrics::PerformanceMetrics,
}

/// GPU setup for benchmarks
pub struct BenchmarkGpu {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub adapter_info: wgpu::AdapterInfo,
}

impl BenchmarkGpu {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .expect("Failed to find adapter");

        let adapter_info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Benchmark Device"),
                    required_features: wgpu::Features::TIMESTAMP_QUERY
                        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
        }
    }

    pub fn create_test_buffer(&self, size: usize) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Test Buffer"),
            size: size as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        })
    }

    pub fn create_buffer_with_data(&self, data: &[f32]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Test Buffer"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE,
            })
    }

    pub fn create_gpu_buffer_set(&self, data: Vec<Vec<f32>>, columns: Vec<String>) -> GpuBufferSet {
        let mut buffers = std::collections::HashMap::new();

        // Store metadata before consuming data
        let row_count = data.get(0).map(|v| v.len() as u32).unwrap_or(0);
        let byte_size = data.get(0).map(|v| v.len() * 4).unwrap_or(0) as u64;

        for (_i, (column_data, column_name)) in data.into_iter().zip(columns.iter()).enumerate() {
            let buffer = self.create_buffer_with_data(&column_data);
            buffers.insert(column_name.clone(), vec![buffer]);
        }

        let metadata = DataMetadata {
            symbol: "TEST".to_string(),
            time_range: TimeRange::new(0, 1000),
            columns: columns.clone(),
            row_count,
            byte_size,
            creation_time: 0,
        };

        GpuBufferSet { buffers, metadata }
    }
}

/// Timer for GPU operations
pub struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    read_buffer: wgpu::Buffer,
    start_idx: u32,
    end_idx: u32,
}

impl GpuTimer {
    pub fn new(device: &wgpu::Device) -> Self {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("GPU Timer Query Set"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });

        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Timer Resolve Buffer"),
            size: 16, // 2 timestamps * 8 bytes
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GPU Timer Read Buffer"),
            size: 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            query_set,
            resolve_buffer,
            read_buffer,
            start_idx: 0,
            end_idx: 1,
        }
    }

    pub fn start(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.query_set, self.start_idx);
    }

    pub fn end(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.query_set, self.end_idx);
    }

    pub async fn read_time(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Duration {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU Timer Resolve"),
        });

        encoder.resolve_query_set(&self.query_set, 0..2, &self.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(&self.resolve_buffer, 0, &self.read_buffer, 0, 16);

        queue.submit(Some(encoder.finish()));

        let buffer_slice = self.read_buffer.slice(..);
        let (sender, receiver) = tokio::sync::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        device.poll(wgpu::Maintain::Wait);
        receiver.await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let timestamps: &[u64] = bytemuck::cast_slice(&data);
        let start = timestamps[0];
        let end = timestamps[1];

        drop(data);
        self.read_buffer.unmap();

        Duration::from_nanos(end - start)
    }
}

/// Run a benchmark with warmup
pub async fn run_benchmark<F, Fut>(
    name: &str,
    warmup_iterations: u32,
    benchmark_iterations: u32,
    mut f: F,
) -> BenchmarkResult
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = metrics::PerformanceMetrics>,
{
    // Warmup
    for _ in 0..warmup_iterations {
        let _ = f().await;
    }

    // Actual benchmark
    let mut durations = Vec::with_capacity(benchmark_iterations as usize);
    let mut all_metrics = Vec::with_capacity(benchmark_iterations as usize);
    let start = Instant::now();

    for _ in 0..benchmark_iterations {
        let iter_start = Instant::now();
        let metrics = f().await;
        let iter_duration = iter_start.elapsed();
        durations.push(iter_duration);
        all_metrics.push(metrics);
    }

    let total_duration = start.elapsed();

    // Calculate statistics
    let sum: Duration = durations.iter().sum();
    let mean = sum / benchmark_iterations;

    let variance: Duration = durations
        .iter()
        .map(|d| {
            let diff = d.as_nanos() as i128 - mean.as_nanos() as i128;
            Duration::from_nanos((diff * diff) as u64)
        })
        .sum::<Duration>()
        / benchmark_iterations;

    let std_dev = Duration::from_nanos((variance.as_nanos() as f64).sqrt() as u64);
    let min = durations.iter().min().copied().unwrap_or(Duration::ZERO);
    let max = durations.iter().max().copied().unwrap_or(Duration::ZERO);

    // Average metrics
    let avg_metrics = metrics::PerformanceMetrics::average(&all_metrics);

    BenchmarkResult {
        name: name.to_string(),
        duration: total_duration,
        iterations: benchmark_iterations,
        mean,
        std_dev,
        min,
        max,
        metrics: avg_metrics,
    }
}

/// Format benchmark result for display
pub fn format_result(result: &BenchmarkResult) -> String {
    format!(
        "{}: mean={:?} (σ={:?}) min={:?} max={:?} [{}x]",
        result.name, result.mean, result.std_dev, result.min, result.max, result.iterations
    )
}

/// Create a test viewport
pub fn create_test_viewport(width: f32, height: f32) -> Viewport {
    Viewport {
        x: 0.0,
        y: 0.0,
        width,
        height,
        zoom_level: 1.0,
        time_range: TimeRange::new(0, 1000),
    }
}
