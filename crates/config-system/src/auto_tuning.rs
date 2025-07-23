//! Auto-tuning system for hardware-based optimization

use crate::{
    ConfigError, GpuChartsConfig, PerformanceConfig, QualityPreset, RenderingConfig, Result,
};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Hardware capabilities detection
#[derive(Debug, Clone)]
pub struct HardwareCapabilities {
    /// GPU memory in bytes
    pub gpu_memory: u64,

    /// GPU compute units
    pub gpu_compute_units: u32,

    /// GPU clock speed in MHz
    pub gpu_clock_mhz: u32,

    /// CPU cores
    pub cpu_cores: usize,

    /// CPU frequency in MHz
    pub cpu_freq_mhz: u64,

    /// System RAM in bytes
    pub system_memory: u64,

    /// Available RAM in bytes
    pub available_memory: u64,

    /// Display resolution
    pub display_width: u32,
    pub display_height: u32,

    /// Platform info
    pub platform: PlatformInfo,
}

#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os: String,
    pub browser: Option<String>,
    pub webgpu_limits: WebGPULimits,
}

#[derive(Debug, Clone)]
pub struct WebGPULimits {
    pub max_texture_dimension_2d: u32,
    pub max_texture_dimension_3d: u32,
    pub max_buffer_size: u64,
    pub max_vertex_buffers: u32,
    pub max_bind_groups: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroups_per_dimension: u32,
}

/// Performance metrics for auto-tuning
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Average FPS over measurement period
    pub avg_fps: f32,

    /// Minimum FPS
    pub min_fps: f32,

    /// Maximum FPS
    pub max_fps: f32,

    /// Frame time percentiles (ms)
    pub frame_time_p50: f32,
    pub frame_time_p90: f32,
    pub frame_time_p99: f32,

    /// GPU utilization (0-100%)
    pub gpu_utilization: f32,

    /// GPU memory usage in bytes
    pub gpu_memory_used: u64,

    /// CPU utilization (0-100%)
    pub cpu_utilization: f32,

    /// Draw calls per frame
    pub avg_draw_calls: f32,

    /// Vertices processed per frame
    pub avg_vertices: u64,
}

/// Auto-tuning engine
pub struct AutoTuner {
    /// Current hardware capabilities
    hardware: Arc<RwLock<HardwareCapabilities>>,

    /// Performance history
    perf_history: Arc<RwLock<Vec<(Instant, PerformanceMetrics)>>>,

    /// Tuning parameters
    params: AutoTuneParams,

    /// Quality presets
    presets: QualityPresetLibrary,
}

#[derive(Debug, Clone)]
struct AutoTuneParams {
    /// Target FPS
    target_fps: u32,

    /// FPS tolerance (e.g., 0.1 = 10%)
    fps_tolerance: f32,

    /// Minimum acceptable FPS
    min_acceptable_fps: u32,

    /// GPU utilization target (0-100%)
    gpu_utilization_target: f32,

    /// Memory headroom (percentage to keep free)
    memory_headroom: f32,

    /// Adjustment aggressiveness (0-1)
    adjustment_speed: f32,
}

impl Default for AutoTuneParams {
    fn default() -> Self {
        Self {
            target_fps: 60,
            fps_tolerance: 0.1,
            min_acceptable_fps: 30,
            gpu_utilization_target: 80.0,
            memory_headroom: 0.2,
            adjustment_speed: 0.5,
        }
    }
}

impl AutoTuner {
    /// Create a new auto-tuner
    pub fn new() -> Self {
        Self {
            hardware: Arc::new(RwLock::new(Self::detect_hardware())),
            perf_history: Arc::new(RwLock::new(Vec::new())),
            params: AutoTuneParams::default(),
            presets: QualityPresetLibrary::new(),
        }
    }

    /// Detect hardware capabilities
    fn detect_hardware() -> HardwareCapabilities {
        // In WASM, we use browser APIs to detect capabilities
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::window;

            let window = window().unwrap();
            let navigator = window.navigator();

            // Get CPU cores from navigator.hardwareConcurrency
            let cpu_cores = navigator.hardware_concurrency() as usize;

            // Memory detection via navigator.deviceMemory (if available)
            // This is an approximation in GB
            let device_memory_gb = js_sys::Reflect::get(&navigator, &"deviceMemory".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(8.0);
            let system_memory = (device_memory_gb * 1024.0 * 1024.0 * 1024.0) as u64;

            // Get display dimensions
            let display_width = window
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1920.0) as u32;
            let display_height = window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1080.0) as u32;

            // Browser detection
            let user_agent = navigator.user_agent().unwrap_or_default();
            let browser = if user_agent.contains("Chrome") {
                Some("Chrome".to_string())
            } else if user_agent.contains("Firefox") {
                Some("Firefox".to_string())
            } else if user_agent.contains("Safari") {
                Some("Safari".to_string())
            } else {
                Some("Unknown".to_string())
            };

            HardwareCapabilities {
                gpu_memory: 8_000_000_000, // Will be detected from WebGPU limits
                gpu_compute_units: 32,     // Default, actual detection requires WebGPU
                gpu_clock_mhz: 1500,       // Default
                cpu_cores,
                cpu_freq_mhz: 2000, // Default estimate
                system_memory,
                available_memory: system_memory, // Assume all available in browser
                display_width,
                display_height,
                platform: PlatformInfo {
                    os: "Web".to_string(),
                    browser,
                    webgpu_limits: WebGPULimits {
                        max_texture_dimension_2d: 16384,
                        max_texture_dimension_3d: 2048,
                        max_buffer_size: 256 * 1024 * 1024,
                        max_vertex_buffers: 8,
                        max_bind_groups: 4,
                        max_compute_workgroup_size_x: 256,
                        max_compute_workgroups_per_dimension: 65535,
                    },
                },
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // For non-WASM builds, use defaults
            HardwareCapabilities {
                gpu_memory: 8_000_000_000,
                gpu_compute_units: 32,
                gpu_clock_mhz: 1500,
                cpu_cores: 4,
                cpu_freq_mhz: 2000,
                system_memory: 16_000_000_000,
                available_memory: 16_000_000_000,
                display_width: 1920,
                display_height: 1080,
                platform: PlatformInfo {
                    os: "Native".to_string(),
                    browser: None,
                    webgpu_limits: WebGPULimits {
                        max_texture_dimension_2d: 16384,
                        max_texture_dimension_3d: 2048,
                        max_buffer_size: 256 * 1024 * 1024,
                        max_vertex_buffers: 8,
                        max_bind_groups: 4,
                        max_compute_workgroup_size_x: 256,
                        max_compute_workgroups_per_dimension: 65535,
                    },
                },
            }
        }
    }

    /// Update hardware capabilities (e.g., after display change)
    pub fn update_hardware(&self) {
        let new_hardware = Self::detect_hardware();
        *self.hardware.write() = new_hardware;
    }

    /// Analyze performance and suggest configuration
    pub fn analyze_and_tune(
        &self,
        current_config: &GpuChartsConfig,
        metrics: PerformanceMetrics,
    ) -> Result<Option<GpuChartsConfig>> {
        // Add to history
        {
            let mut history = self.perf_history.write();
            history.push((Instant::now(), metrics.clone()));

            // Keep last 100 samples
            if history.len() > 100 {
                history.drain(0..50);
            }
        }

        // Check if tuning is needed
        if !self.needs_tuning(&metrics) {
            return Ok(None);
        }

        // Generate new configuration
        let hardware = self.hardware.read();
        let new_config = self.generate_optimized_config(current_config, &metrics, &hardware)?;

        Ok(Some(new_config))
    }

    /// Check if tuning is needed
    fn needs_tuning(&self, metrics: &PerformanceMetrics) -> bool {
        let fps_delta = (metrics.avg_fps - self.params.target_fps as f32).abs();
        let fps_threshold = self.params.target_fps as f32 * self.params.fps_tolerance;

        // Need tuning if:
        // 1. FPS is outside tolerance range
        fps_delta > fps_threshold ||
        // 2. FPS is below minimum acceptable
        metrics.avg_fps < self.params.min_acceptable_fps as f32 ||
        // 3. GPU is underutilized (could increase quality)
        (metrics.gpu_utilization < self.params.gpu_utilization_target - 20.0
         && metrics.avg_fps > self.params.target_fps as f32) ||
        // 4. GPU memory is critically high
        metrics.gpu_memory_used as f32 >
            self.hardware.read().gpu_memory as f32 * (1.0 - self.params.memory_headroom)
    }

    /// Generate optimized configuration
    fn generate_optimized_config(
        &self,
        current: &GpuChartsConfig,
        metrics: &PerformanceMetrics,
        hardware: &HardwareCapabilities,
    ) -> Result<GpuChartsConfig> {
        let mut config = current.clone();

        // Determine quality direction
        let quality_direction = if metrics.avg_fps < self.params.target_fps as f32 {
            QualityDirection::Decrease
        } else if metrics.gpu_utilization < self.params.gpu_utilization_target - 20.0 {
            QualityDirection::Increase
        } else {
            QualityDirection::Maintain
        };

        // Apply performance adjustments
        match quality_direction {
            QualityDirection::Decrease => {
                self.reduce_quality(&mut config, metrics, hardware);
            }
            QualityDirection::Increase => {
                self.increase_quality(&mut config, metrics, hardware);
            }
            QualityDirection::Maintain => {
                // Fine-tune existing settings
                self.fine_tune(&mut config, metrics, hardware);
            }
        }

        // Apply hardware-specific optimizations
        self.apply_hardware_optimizations(&mut config, hardware);

        Ok(config)
    }

    /// Reduce quality settings to improve performance
    fn reduce_quality(
        &self,
        config: &mut GpuChartsConfig,
        metrics: &PerformanceMetrics,
        hardware: &HardwareCapabilities,
    ) {
        let severity =
            (self.params.target_fps as f32 - metrics.avg_fps) / self.params.target_fps as f32;

        // Prioritized quality reductions
        if severity > 0.5 {
            // Severe performance issues - aggressive reductions
            config.rendering.resolution_scale = (config.rendering.resolution_scale * 0.75).max(0.5);
            config.rendering.antialiasing = false;
            config.performance.lod_bias = 2.0;
            config.performance.vertex_compression = true;
            config.rendering.chart_settings.three_d.enable_shadows = false;
            config.rendering.chart_settings.three_d.lighting_quality = crate::LightingQuality::Low;
        } else if severity > 0.25 {
            // Moderate issues
            config.rendering.resolution_scale = (config.rendering.resolution_scale * 0.9).max(0.75);
            config.performance.lod_bias = 1.5;
            config.rendering.chart_settings.three_d.lighting_quality =
                match config.rendering.chart_settings.three_d.lighting_quality {
                    crate::LightingQuality::Ultra => crate::LightingQuality::High,
                    crate::LightingQuality::High => crate::LightingQuality::Medium,
                    _ => crate::LightingQuality::Low,
                };
        } else {
            // Minor adjustments
            config.performance.lod_bias = (config.performance.lod_bias * 1.1).min(2.0);
            config.performance.draw_call_batch_size =
                ((config.performance.draw_call_batch_size as f32 * 1.5) as u32).min(1000);
        }
    }

    /// Increase quality settings when performance allows
    fn increase_quality(
        &self,
        config: &mut GpuChartsConfig,
        metrics: &PerformanceMetrics,
        hardware: &HardwareCapabilities,
    ) {
        let headroom =
            (metrics.avg_fps - self.params.target_fps as f32) / self.params.target_fps as f32;

        // Prioritized quality increases
        if headroom > 0.5 && hardware.gpu_memory > 4_000_000_000 {
            // Lots of headroom - increase aggressively
            config.rendering.resolution_scale = (config.rendering.resolution_scale * 1.25).min(2.0);
            config.rendering.antialiasing = true;
            config.rendering.chart_settings.three_d.enable_shadows = true;
            config.rendering.chart_settings.three_d.lighting_quality = crate::LightingQuality::High;
        } else if headroom > 0.25 {
            // Moderate headroom
            config.rendering.resolution_scale = (config.rendering.resolution_scale * 1.1).min(1.5);
            config.performance.lod_bias = (config.performance.lod_bias * 0.9).max(0.5);
        } else {
            // Minor improvements
            config.performance.lod_bias = (config.performance.lod_bias * 0.95).max(0.75);
        }
    }

    /// Fine-tune settings for optimal performance
    fn fine_tune(
        &self,
        config: &mut GpuChartsConfig,
        metrics: &PerformanceMetrics,
        hardware: &HardwareCapabilities,
    ) {
        // Adjust batch sizes based on draw call performance
        if metrics.avg_draw_calls > 100.0 {
            config.performance.draw_call_batch_size =
                ((config.performance.draw_call_batch_size as f32 * 1.2) as u32).min(500);
        }

        // Optimize cache size based on memory usage
        let memory_usage_ratio = metrics.gpu_memory_used as f32 / hardware.gpu_memory as f32;
        if memory_usage_ratio < 0.5 {
            config.data.cache_size = (config.data.cache_size as f32 * 1.1) as u64;
        } else if memory_usage_ratio > 0.8 {
            config.data.cache_size = (config.data.cache_size as f32 * 0.9) as u64;
        }
    }

    /// Apply hardware-specific optimizations
    fn apply_hardware_optimizations(
        &self,
        config: &mut GpuChartsConfig,
        hardware: &HardwareCapabilities,
    ) {
        // Mobile/integrated GPU optimizations
        if hardware.gpu_memory < 2_000_000_000 {
            config.performance.vertex_compression = true;
            config.performance.indirect_drawing = false; // May be slower on weak GPUs
            config.data.compression.enabled = true;
            config.data.compression.algorithm = crate::CompressionAlgorithm::Lz4;
        }

        // High-end GPU optimizations
        if hardware.gpu_memory > 8_000_000_000 && hardware.gpu_compute_units > 40 {
            config.performance.gpu_culling = true;
            config.performance.indirect_drawing = true;
            config.rendering.max_render_passes = 8;
            config.data.prefetch_distance = 3.0;
        }

        // CPU-based optimizations
        if hardware.cpu_cores >= 8 {
            config.data.streaming.enable_backpressure = true;
            config.data.streaming.buffer_size = 2 * 1024 * 1024;
        }

        // Display-based optimizations
        let pixel_count = hardware.display_width * hardware.display_height;
        if pixel_count > 3840 * 2160 {
            // 4K+ displays
            config.rendering.resolution_scale = config.rendering.resolution_scale.min(1.0);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum QualityDirection {
    Increase,
    Decrease,
    Maintain,
}

/// Quality preset library
struct QualityPresetLibrary {
    presets: Vec<(QualityPreset, PresetConfig)>,
}

#[derive(Debug, Clone)]
struct PresetConfig {
    min_gpu_memory: u64,
    resolution_scale: f32,
    antialiasing: bool,
    lod_bias: f32,
    vertex_compression: bool,
    shadows: bool,
    lighting_quality: crate::LightingQuality,
}

impl QualityPresetLibrary {
    fn new() -> Self {
        let presets = vec![
            (
                QualityPreset::Potato,
                PresetConfig {
                    min_gpu_memory: 1_000_000_000,
                    resolution_scale: 0.5,
                    antialiasing: false,
                    lod_bias: 2.0,
                    vertex_compression: true,
                    shadows: false,
                    lighting_quality: crate::LightingQuality::Low,
                },
            ),
            (
                QualityPreset::Low,
                PresetConfig {
                    min_gpu_memory: 2_000_000_000,
                    resolution_scale: 0.75,
                    antialiasing: false,
                    lod_bias: 1.5,
                    vertex_compression: true,
                    shadows: false,
                    lighting_quality: crate::LightingQuality::Low,
                },
            ),
            (
                QualityPreset::Medium,
                PresetConfig {
                    min_gpu_memory: 4_000_000_000,
                    resolution_scale: 1.0,
                    antialiasing: true,
                    lod_bias: 1.0,
                    vertex_compression: false,
                    shadows: false,
                    lighting_quality: crate::LightingQuality::Medium,
                },
            ),
            (
                QualityPreset::High,
                PresetConfig {
                    min_gpu_memory: 6_000_000_000,
                    resolution_scale: 1.0,
                    antialiasing: true,
                    lod_bias: 0.75,
                    vertex_compression: false,
                    shadows: true,
                    lighting_quality: crate::LightingQuality::High,
                },
            ),
            (
                QualityPreset::Ultra,
                PresetConfig {
                    min_gpu_memory: 8_000_000_000,
                    resolution_scale: 1.5,
                    antialiasing: true,
                    lod_bias: 0.5,
                    vertex_compression: false,
                    shadows: true,
                    lighting_quality: crate::LightingQuality::Ultra,
                },
            ),
            (
                QualityPreset::Extreme,
                PresetConfig {
                    min_gpu_memory: 12_000_000_000,
                    resolution_scale: 2.0,
                    antialiasing: true,
                    lod_bias: 0.25,
                    vertex_compression: false,
                    shadows: true,
                    lighting_quality: crate::LightingQuality::Ultra,
                },
            ),
        ];

        Self { presets }
    }

    /// Get recommended preset for hardware
    fn recommend_preset(&self, hardware: &HardwareCapabilities) -> QualityPreset {
        for (preset, config) in self.presets.iter().rev() {
            if hardware.gpu_memory >= config.min_gpu_memory {
                return *preset;
            }
        }
        QualityPreset::Potato
    }
}
