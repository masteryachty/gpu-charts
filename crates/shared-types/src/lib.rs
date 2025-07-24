//! Shared types used across all GPU Charts crates

pub mod chart_config;
pub mod data_types;
pub mod errors;
pub mod events;
pub mod store_state;

pub use chart_config::*;
pub use data_types::*;
pub use errors::*;
pub use events::*;
pub use store_state::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// Common data structures used across crates (as per architect.md)

/// Chart configuration for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfiguration {
    pub chart_type: ChartType,
    pub data_handles: Vec<DataHandle>,
    pub visual_config: VisualConfig,
    pub overlays: Vec<OverlayConfig>,
}

/// Types of charts supported
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChartType {
    Line,
    Candlestick,
    Bar,
    Area,
}

/// Handle to data stored in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataHandle {
    pub id: Uuid,
    pub metadata: DataMetadata,
}

/// Metadata about a data set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMetadata {
    pub symbol: String,
    pub start_time: u64,
    pub end_time: u64,
    pub columns: Vec<String>,
    pub row_count: usize,
}

/// Visual configuration for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualConfig {
    pub line_width: f32,
    pub colors: Vec<[f32; 3]>,
    pub background_color: [f32; 4],
    pub grid_color: [f32; 4],
    pub show_grid: bool,
    pub show_axes: bool,
}

/// Overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub overlay_type: String,
    pub params: HashMap<String, f32>,
}

/// Quality preset configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityPreset {
    Low,
    Medium,
    High,
    Ultra,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub target_fps: u32,
    pub max_data_points: usize,
    pub enable_culling: bool,
    pub enable_lod: bool,
}

/// Overall GPU Charts configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuChartsConfig {
    pub quality_preset: QualityPreset,
    pub enable_auto_tuning: bool,
    pub performance: PerformanceConfig,
    pub visual: VisualConfig,
}

impl Default for GpuChartsConfig {
    fn default() -> Self {
        Self {
            quality_preset: QualityPreset::Medium,
            enable_auto_tuning: true,
            performance: PerformanceConfig {
                target_fps: 60,
                max_data_points: 1_000_000,
                enable_culling: true,
                enable_lod: true,
            },
            visual: VisualConfig {
                line_width: 2.0,
                colors: vec![[0.0, 0.5, 1.0], [1.0, 0.5, 0.0], [0.0, 1.0, 0.5]],
                background_color: [0.0, 0.0, 0.0, 1.0],
                grid_color: [0.2, 0.2, 0.2, 1.0],
                show_grid: true,
                show_axes: true,
            },
        }
    }
}

/// Data parsing result from server
#[derive(Debug, Clone)]
pub struct ParsedData {
    pub time_data: Vec<u32>,
    pub value_data: HashMap<String, Vec<f32>>,
    pub metadata: DataMetadata,
}

/// World bounds for data visualization
#[derive(Debug, Clone, Copy)]
pub struct WorldBounds {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

/// Screen bounds for rendering
#[derive(Debug, Clone, Copy)]
pub struct ScreenBounds {
    pub width: f32,
    pub height: f32,
}

/// Result of a render operation
#[derive(Debug, Clone)]
pub struct RenderStats {
    pub frame_time_ms: f32,
    pub draw_calls: u32,
    pub vertices_rendered: u32,
    pub gpu_memory_used: usize,
}
