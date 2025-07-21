//! Performance metrics collection

use std::time::Duration;
use sysinfo::{get_current_pid, System};

/// Comprehensive performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    // Timing metrics
    pub frame_time: Duration,
    pub cpu_time: Duration,
    pub gpu_time: Duration,
    pub data_fetch_time: Duration,
    pub parse_time: Duration,
    pub render_time: Duration,

    // Count metrics
    pub draw_calls: u32,
    pub vertices_rendered: u64,
    pub triangles_rendered: u64,
    pub buffer_allocations: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,

    // Memory metrics
    pub cpu_memory_mb: f32,
    pub gpu_memory_mb: f32,
    pub buffer_pool_usage_mb: f32,
    pub cache_size_mb: f32,

    // Throughput metrics
    pub points_per_second: f64,
    pub bytes_per_second: f64,
    pub frames_per_second: f64,
}

impl PerformanceMetrics {
    /// Calculate FPS from frame time
    pub fn calculate_fps(&mut self) {
        if self.frame_time.as_secs_f64() > 0.0 {
            self.frames_per_second = 1.0 / self.frame_time.as_secs_f64();
        }
    }

    /// Calculate throughput
    pub fn calculate_throughput(&mut self, points: u64, bytes: u64) {
        let total_time = self.frame_time.as_secs_f64();
        if total_time > 0.0 {
            self.points_per_second = points as f64 / total_time;
            self.bytes_per_second = bytes as f64 / total_time;
        }
    }

    /// Merge another metrics instance
    pub fn merge(&mut self, other: &PerformanceMetrics) {
        self.frame_time += other.frame_time;
        self.cpu_time += other.cpu_time;
        self.gpu_time += other.gpu_time;
        self.data_fetch_time += other.data_fetch_time;
        self.parse_time += other.parse_time;
        self.render_time += other.render_time;

        self.draw_calls += other.draw_calls;
        self.vertices_rendered += other.vertices_rendered;
        self.triangles_rendered += other.triangles_rendered;
        self.buffer_allocations += other.buffer_allocations;
        self.cache_hits += other.cache_hits;
        self.cache_misses += other.cache_misses;

        self.cpu_memory_mb = self.cpu_memory_mb.max(other.cpu_memory_mb);
        self.gpu_memory_mb = self.gpu_memory_mb.max(other.gpu_memory_mb);
        self.buffer_pool_usage_mb = self.buffer_pool_usage_mb.max(other.buffer_pool_usage_mb);
        self.cache_size_mb = self.cache_size_mb.max(other.cache_size_mb);

        self.points_per_second += other.points_per_second;
        self.bytes_per_second += other.bytes_per_second;
        self.frames_per_second += other.frames_per_second;
    }

    /// Average multiple metrics
    pub fn average(metrics: &[PerformanceMetrics]) -> Self {
        if metrics.is_empty() {
            return Self::default();
        }

        let mut avg = Self::default();
        let count = metrics.len() as f64;

        for m in metrics {
            avg.merge(m);
        }

        // Average timing metrics
        avg.frame_time = Duration::from_secs_f64(avg.frame_time.as_secs_f64() / count);
        avg.cpu_time = Duration::from_secs_f64(avg.cpu_time.as_secs_f64() / count);
        avg.gpu_time = Duration::from_secs_f64(avg.gpu_time.as_secs_f64() / count);
        avg.data_fetch_time = Duration::from_secs_f64(avg.data_fetch_time.as_secs_f64() / count);
        avg.parse_time = Duration::from_secs_f64(avg.parse_time.as_secs_f64() / count);
        avg.render_time = Duration::from_secs_f64(avg.render_time.as_secs_f64() / count);

        // Average count metrics
        avg.draw_calls = (avg.draw_calls as f64 / count) as u32;
        avg.vertices_rendered = (avg.vertices_rendered as f64 / count) as u64;
        avg.triangles_rendered = (avg.triangles_rendered as f64 / count) as u64;
        avg.buffer_allocations = (avg.buffer_allocations as f64 / count) as u32;
        avg.cache_hits = (avg.cache_hits as f64 / count) as u32;
        avg.cache_misses = (avg.cache_misses as f64 / count) as u32;

        // Memory metrics stay as max
        // Throughput metrics are averaged
        avg.points_per_second /= count;
        avg.bytes_per_second /= count;
        avg.frames_per_second /= count;

        avg
    }

    /// Check if performance meets targets
    pub fn meets_targets(&self, targets: &PerformanceTargets) -> bool {
        self.frame_time <= targets.max_frame_time
            && self.gpu_time <= targets.max_gpu_time
            && self.draw_calls <= targets.max_draw_calls
            && self.frames_per_second >= targets.min_fps
    }
}

/// Performance targets from PERFORMANCE_GUIDE.md
#[derive(Debug, Clone)]
pub struct PerformanceTargets {
    pub max_frame_time: Duration,
    pub max_gpu_time: Duration,
    pub max_cpu_time: Duration,
    pub max_draw_calls: u32,
    pub min_fps: f64,
    pub max_memory_mb: f32,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            max_frame_time: Duration::from_millis(16),
            max_gpu_time: Duration::from_millis(14),
            max_cpu_time: Duration::from_millis(5),
            max_draw_calls: 100,
            min_fps: 60.0,
            max_memory_mb: 2048.0,
        }
    }
}

/// System metrics collector
pub struct MetricsCollector {
    system: System,
    process_id: sysinfo::Pid,
}

impl MetricsCollector {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let process_id = get_current_pid().expect("Failed to get current process ID");

        Self { system, process_id }
    }

    pub fn collect_system_metrics(&mut self) -> SystemMetrics {
        self.system.refresh_process(self.process_id);

        let process = self
            .system
            .process(self.process_id)
            .expect("Failed to get process info");

        SystemMetrics {
            cpu_usage: process.cpu_usage(),
            memory_usage_mb: process.memory() as f32 / 1024.0 / 1024.0,
            total_memory_mb: self.system.total_memory() as f32 / 1024.0 / 1024.0,
            available_memory_mb: self.system.available_memory() as f32 / 1024.0 / 1024.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub memory_usage_mb: f32,
    pub total_memory_mb: f32,
    pub available_memory_mb: f32,
}

/// Format metrics for reporting
pub fn format_metrics(metrics: &PerformanceMetrics) -> String {
    format!(
        r#"Performance Metrics:
  Frame Time: {:?} ({:.1} FPS)
  CPU Time: {:?}
  GPU Time: {:?}
  
  Data Operations:
    Fetch: {:?}
    Parse: {:?}
    
  Rendering:
    Draw Calls: {}
    Vertices: {}
    Triangles: {}
    
  Memory:
    CPU: {:.1} MB
    GPU: {:.1} MB
    Buffer Pool: {:.1} MB
    Cache: {:.1} MB
    
  Throughput:
    Points/sec: {:.0}
    MB/sec: {:.1}
    
  Cache:
    Hits: {} ({:.1}%)
    Misses: {}
"#,
        metrics.frame_time,
        metrics.frames_per_second,
        metrics.cpu_time,
        metrics.gpu_time,
        metrics.data_fetch_time,
        metrics.parse_time,
        metrics.draw_calls,
        metrics.vertices_rendered,
        metrics.triangles_rendered,
        metrics.cpu_memory_mb,
        metrics.gpu_memory_mb,
        metrics.buffer_pool_usage_mb,
        metrics.cache_size_mb,
        metrics.points_per_second,
        metrics.bytes_per_second / 1_000_000.0,
        metrics.cache_hits,
        if metrics.cache_hits + metrics.cache_misses > 0 {
            100.0 * metrics.cache_hits as f32 / (metrics.cache_hits + metrics.cache_misses) as f32
        } else {
            0.0
        },
        metrics.cache_misses
    )
}
