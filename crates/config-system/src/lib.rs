//! Phase 3 Configuration System with hot-reload and auto-tuning capabilities

pub mod auto_tuning;
pub mod file_watcher;
pub mod hot_reload;
pub mod parser;
pub mod presets;
pub mod schema;
pub mod system;
pub mod validation;

use gpu_charts_shared::{Error as SharedError, Result as SharedResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Configuration system errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Hot reload error: {0}")]
    HotReload(String),

    #[error("Auto-tuning error: {0}")]
    AutoTuning(String),

    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Shared error: {0}")]
    Shared(#[from] SharedError),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuChartsConfig {
    /// Version of the configuration schema
    pub version: String,

    /// Chart rendering configuration
    pub rendering: RenderingConfig,

    /// Data management configuration
    pub data: DataConfig,

    /// Performance optimization settings
    pub performance: PerformanceConfig,

    /// Feature flags
    pub features: FeatureFlags,

    /// Telemetry settings
    pub telemetry: TelemetryConfig,

    /// Custom presets
    pub presets: Option<Vec<PresetConfig>>,
}

/// Rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingConfig {
    /// Target FPS
    pub target_fps: u32,

    /// Resolution scale (0.5 - 2.0)
    pub resolution_scale: f32,

    /// Enable antialiasing
    pub antialiasing: bool,

    /// Enable vsync
    pub vsync: bool,

    /// Maximum concurrent render passes
    pub max_render_passes: u32,

    /// GPU memory limit in bytes
    pub gpu_memory_limit: Option<u64>,

    /// Chart-specific settings
    pub chart_settings: ChartSettings,
}

/// Chart-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSettings {
    /// Line chart settings
    pub line: LineChartSettings,

    /// Scatter plot settings
    pub scatter: ScatterPlotSettings,

    /// Heatmap settings
    pub heatmap: HeatmapSettings,

    /// 3D chart settings
    pub three_d: ThreeDSettings,
}

/// Line chart settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineChartSettings {
    pub line_width: f32,
    pub point_size: Option<f32>,
    pub smooth_lines: bool,
    pub area_fill: bool,
}

/// Scatter plot settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScatterPlotSettings {
    pub point_size: f32,
    pub point_shape: PointShape,
    pub enable_clustering: bool,
    pub cluster_threshold: f32,
}

/// Heatmap settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapSettings {
    pub interpolation: InterpolationMethod,
    pub color_scheme: String,
    pub show_contours: bool,
    pub contour_levels: u32,
}

/// 3D chart settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeDSettings {
    pub enable_shadows: bool,
    pub camera_type: CameraType,
    pub lighting_quality: LightingQuality,
    pub depth_test: bool,
}

/// Data management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    /// Cache size in bytes
    pub cache_size: u64,

    /// Enable prefetching
    pub prefetch_enabled: bool,

    /// Prefetch distance (in viewport units)
    pub prefetch_distance: f32,

    /// Compression settings
    pub compression: CompressionConfig,

    /// Streaming settings
    pub streaming: StreamingConfig,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithm: CompressionAlgorithm,
    pub level: u8,
}

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub chunk_size: u32,
    pub buffer_size: u32,
    pub enable_backpressure: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable GPU culling
    pub gpu_culling: bool,

    /// Enable LOD system
    pub lod_enabled: bool,

    /// LOD bias (higher = more aggressive)
    pub lod_bias: f32,

    /// Enable vertex compression
    pub vertex_compression: bool,

    /// Enable indirect drawing
    pub indirect_drawing: bool,

    /// Batch size for draw calls
    pub draw_call_batch_size: u32,

    /// Auto-tuning settings
    pub auto_tuning: AutoTuningConfig,
}

/// Auto-tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoTuningConfig {
    pub enabled: bool,
    pub profile_duration_ms: u32,
    pub adjustment_threshold: f32,
    pub max_quality_preset: QualityPreset,
    pub min_quality_preset: QualityPreset,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub scatter_plots: bool,
    pub heatmaps: bool,
    pub three_d_charts: bool,
    pub technical_indicators: bool,
    pub annotations: bool,
    pub custom_shaders: bool,
    pub experimental_features: bool,
}

/// Telemetry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub performance_tracking: bool,
    pub error_reporting: bool,
    pub usage_analytics: bool,
    pub custom_events: bool,
    pub sampling_rate: f32,
}

/// Preset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetConfig {
    pub name: String,
    pub description: String,
    pub extends: Option<String>,
    pub overrides: serde_json::Value,
}

/// Point shapes for scatter plots
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PointShape {
    Circle,
    Square,
    Triangle,
    Diamond,
    Cross,
    Plus,
}

/// Interpolation methods
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InterpolationMethod {
    Nearest,
    Linear,
    Cubic,
    Lanczos,
}

/// Camera types for 3D
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CameraType {
    Orbit,
    Free,
    Fixed,
}

/// Lighting quality levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LightingQuality {
    Low,
    Medium,
    High,
    Ultra,
}

/// Compression algorithms
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    Lz4,
    Zstd,
    Gzip,
}

/// Quality presets
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityPreset {
    Potato,
    Low,
    Medium,
    High,
    Ultra,
    Extreme,
}

impl Default for GpuChartsConfig {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            rendering: RenderingConfig::default(),
            data: DataConfig::default(),
            performance: PerformanceConfig::default(),
            features: FeatureFlags::default(),
            telemetry: TelemetryConfig::default(),
            presets: None,
        }
    }
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            target_fps: 60,
            resolution_scale: 1.0,
            antialiasing: true,
            vsync: true,
            max_render_passes: 4,
            gpu_memory_limit: None,
            chart_settings: ChartSettings::default(),
        }
    }
}

impl Default for ChartSettings {
    fn default() -> Self {
        Self {
            line: LineChartSettings::default(),
            scatter: ScatterPlotSettings::default(),
            heatmap: HeatmapSettings::default(),
            three_d: ThreeDSettings::default(),
        }
    }
}

impl Default for LineChartSettings {
    fn default() -> Self {
        Self {
            line_width: 2.0,
            point_size: None,
            smooth_lines: false,
            area_fill: false,
        }
    }
}

impl Default for ScatterPlotSettings {
    fn default() -> Self {
        Self {
            point_size: 4.0,
            point_shape: PointShape::Circle,
            enable_clustering: true,
            cluster_threshold: 10.0,
        }
    }
}

impl Default for HeatmapSettings {
    fn default() -> Self {
        Self {
            interpolation: InterpolationMethod::Linear,
            color_scheme: "viridis".to_string(),
            show_contours: false,
            contour_levels: 10,
        }
    }
}

impl Default for ThreeDSettings {
    fn default() -> Self {
        Self {
            enable_shadows: true,
            camera_type: CameraType::Orbit,
            lighting_quality: LightingQuality::Medium,
            depth_test: true,
        }
    }
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            cache_size: 100 * 1024 * 1024, // 100MB
            prefetch_enabled: true,
            prefetch_distance: 2.0,
            compression: CompressionConfig::default(),
            streaming: StreamingConfig::default(),
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: CompressionAlgorithm::Lz4,
            level: 3,
        }
    }
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 65536,
            buffer_size: 1024 * 1024,
            enable_backpressure: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            gpu_culling: true,
            lod_enabled: true,
            lod_bias: 1.0,
            vertex_compression: true,
            indirect_drawing: true,
            draw_call_batch_size: 100,
            auto_tuning: AutoTuningConfig::default(),
        }
    }
}

impl Default for AutoTuningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            profile_duration_ms: 5000,
            adjustment_threshold: 0.1,
            max_quality_preset: QualityPreset::Ultra,
            min_quality_preset: QualityPreset::Low,
        }
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            scatter_plots: true,
            heatmaps: true,
            three_d_charts: true,
            technical_indicators: true,
            annotations: true,
            custom_shaders: false,
            experimental_features: false,
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            performance_tracking: true,
            error_reporting: true,
            usage_analytics: false,
            custom_events: true,
            sampling_rate: 0.1,
        }
    }
}
