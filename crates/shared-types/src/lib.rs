//! Shared types used across all GPU Charts crates

pub mod data_types;
pub mod errors;
pub mod events;

pub use data_types::*;
pub use errors::*;
pub use events::*;

use serde::{Deserialize, Serialize};
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
    pub start_time: u32,
    pub end_time: u32,
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
