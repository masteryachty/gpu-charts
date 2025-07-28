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

use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use uuid::Uuid;

// Common data structures used across crates (as per architect.md)

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

/// Quality preset levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityPreset {
    Low,
    Medium,
    High,
    Ultra,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub quality_preset: QualityPreset,
    pub target_fps: u32,
    pub enable_adaptive_quality: bool,
    pub max_data_points: usize,
    pub enable_culling: bool,
    pub enable_lod: bool,
}

/// Main GPU Charts configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuChartsConfig {
    pub visual: VisualConfig,
    pub performance: PerformanceConfig,
    pub enable_auto_tuning: bool,
}

impl Default for GpuChartsConfig {
    fn default() -> Self {
        Self {
            visual: VisualConfig {
                line_width: 2.0,
                colors: vec![[0.0, 0.5, 1.0], [1.0, 0.2, 0.2], [0.0, 1.0, 0.0]],
                background_color: [0.05, 0.05, 0.05, 1.0],
                grid_color: [0.2, 0.2, 0.2, 0.5],
                show_grid: true,
                show_axes: true,
            },
            performance: PerformanceConfig {
                quality_preset: QualityPreset::High,
                target_fps: 60,
                enable_adaptive_quality: true,
                max_data_points: 1_000_000,
                enable_culling: true,
                enable_lod: true,
            },
            enable_auto_tuning: false,
        }
    }
}

/// Overlay configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    pub overlay_type: String,
    pub params: HashMap<String, f32>,
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
