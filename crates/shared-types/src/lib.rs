//! Shared types for GPU Charts architecture
//!
//! This crate contains all types that are shared between the data-manager,
//! renderer, and wasm-bridge crates. These types are designed for maximum
//! performance with zero-copy serialization where possible.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "typescript")]
use tsify::Tsify;

/// Time range for data queries
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct TimeRange {
    pub start: u64,
    pub end: u64,
}

impl TimeRange {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    pub fn duration(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub fn contains(&self, timestamp: u64) -> bool {
        timestamp >= self.start && timestamp <= self.end
    }
}

/// Handle to GPU data buffers
/// This is passed between WASM modules without copying data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct DataHandle {
    pub id: Uuid,
    pub metadata: DataMetadata,
}

/// Metadata about a data buffer set
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct DataMetadata {
    pub symbol: String,
    pub time_range: TimeRange,
    pub columns: Vec<String>,
    pub row_count: u32,
    pub byte_size: u64,
    pub creation_time: u64,
}

/// Chart types supported by the renderer
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[serde(rename_all = "lowercase")]
pub enum ChartType {
    Line,
    Candlestick,
    Area,
    Bar,
}

/// Visual configuration for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct VisualConfig {
    pub background_color: [f32; 4],
    pub grid_color: [f32; 4],
    pub text_color: [f32; 4],
    pub margin_percent: f32,
    pub show_grid: bool,
    pub show_axes: bool,
}

impl Default for VisualConfig {
    fn default() -> Self {
        Self {
            background_color: [0.0, 0.0, 0.0, 1.0],
            grid_color: [0.2, 0.2, 0.2, 1.0],
            text_color: [1.0, 1.0, 1.0, 1.0],
            margin_percent: 0.1,
            show_grid: true,
            show_axes: true,
        }
    }
}

/// Complete chart configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct ChartConfiguration {
    pub chart_type: ChartType,
    pub data_handles: Vec<DataHandle>,
    pub visual_config: VisualConfig,
    pub overlays: Vec<OverlayConfig>,
}

/// Overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct OverlayConfig {
    pub overlay_type: String,
    pub data_handle: Option<DataHandle>,
    pub parameters: serde_json::Value,
    pub render_location: RenderLocation,
}

/// Where to render an overlay
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum RenderLocation {
    MainChart,
    SubChart,
}

/// Data request from UI to data manager
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct DataRequest {
    pub symbol: String,
    pub time_range: TimeRange,
    pub columns: Vec<String>,
    pub aggregation: Option<AggregationConfig>,
    pub max_points: Option<u32>,
}

/// Aggregation configuration for data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct AggregationConfig {
    pub aggregation_type: AggregationType,
    pub timeframe: u32, // in seconds
}

/// Types of aggregation supported
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[serde(rename_all = "lowercase")]
pub enum AggregationType {
    Ohlc,
    Average,
    Sum,
    Min,
    Max,
}

/// Performance hints for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub struct PerformanceHints {
    pub target_fps: u32,
    pub max_points_per_series: Option<u32>,
    pub enable_lod: bool,
    pub enable_gpu_culling: bool,
}

impl Default for PerformanceHints {
    fn default() -> Self {
        Self {
            target_fps: 60,
            max_points_per_series: None,
            enable_lod: true,
            enable_gpu_culling: true,
        }
    }
}

/// GPU buffer information for zero-copy sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuBufferInfo {
    pub buffer_id: u64,
    pub size: u64,
    pub usage: u32,
}

/// Result type for operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for the system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
pub enum Error {
    DataNotFound(String),
    NetworkError(String),
    GpuError(String),
    ParseError(String),
    InvalidConfiguration(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DataNotFound(msg) => write!(f, "Data not found: {}", msg),
            Error::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Error::GpuError(msg) => write!(f, "GPU error: {}", msg),
            Error::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Error::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range() {
        let range = TimeRange::new(1000, 2000);
        assert_eq!(range.duration(), 1000);
        assert!(range.contains(1500));
        assert!(!range.contains(500));
        assert!(!range.contains(2500));
    }

    #[test]
    fn test_serialization() {
        let config = ChartConfiguration {
            chart_type: ChartType::Candlestick,
            data_handles: vec![],
            visual_config: VisualConfig::default(),
            overlays: vec![],
        };

        let serialized = bincode::serialize(&config).unwrap();
        let deserialized: ChartConfiguration = bincode::deserialize(&serialized).unwrap();

        assert_eq!(config.chart_type, deserialized.chart_type);
    }
}
