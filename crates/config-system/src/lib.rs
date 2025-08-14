//! Configuration system for GPU Charts
//! Manages presets, quality settings, and performance tuning

use serde::{Deserialize, Serialize};

pub mod presets;

/// Rendering preset configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPreset {
    pub name: String,
    pub description: String,
    pub chart_types: Vec<RenderPreset>,
}

/// Render type for chart elements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RenderType {
    Line,
    Bar,
    Candlestick,
    Triangle, // For trade markers
    Area,
}

impl std::fmt::Display for RenderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderType::Line => write!(f, "Line"),
            RenderType::Bar => write!(f, "Bar"),
            RenderType::Candlestick => write!(f, "Candlestick"),
            RenderType::Triangle => write!(f, "Triangle"),
            RenderType::Area => write!(f, "Area"),
        }
    }
}

/// Style configuration for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<[f32; 4]>, // Single color (for most render types)
    #[serde(rename = "colorOptions", skip_serializing_if = "Option::is_none")]
    pub color_options: Option<Vec<[f32; 4]>>, // Multiple colors (e.g., for trades buy/sell)
    pub size: f32, // Line width, triangle size, bar width, etc.
}

/// Compute operation for calculated fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComputeOp {
    /// Average of all inputs: (a + b + ...) / n
    Average,
    /// Sum of all inputs: a + b + ...
    Sum,
    /// Difference: a - b
    Difference,
    /// Product: a * b * ...
    Product,
    /// Ratio: a / b
    Ratio,
    /// Min value
    Min,
    /// Max value
    Max,
    /// Weighted average: (a * weight_a + b * weight_b) / (weight_a + weight_b)
    WeightedAverage { weights: Vec<f32> },
    /// Relative Strength Index with period
    Rsi { period: u32 },
}

/// Chart-specific preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPreset {
    pub render_type: RenderType,
    pub data_columns: Vec<(String, String)>, // (data_type, column_name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_data_columns: Option<Vec<(String, String)>>, // Additional columns not used for Y bounds (e.g., side for coloring)
    pub visible: bool,
    pub label: String,
    #[serde(flatten)]
    pub style: RenderStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_op: Option<ComputeOp>, // For calculated fields like mid price
}

/// Preset manager for common chart configurations
pub struct PresetManager {
    presets: Vec<ChartPreset>,
}

impl Default for PresetManager {
    fn default() -> Self {
        let presets = presets::get_all_presets();
        Self { presets }
    }
}

impl PresetManager {
    pub fn new() -> Self {
        Self::default()
    }

    // pub fn get_preset(&self, name: &str) -> Option<&ChartPreset> {
    //     self.presets.iter().find(|p| p.name == name)
    // }

    pub fn list_presets_by_name(&self) -> Vec<&str> {
        self.presets.iter().map(|p| p.name.as_str()).collect()
    }

    /// Get all presets
    pub fn get_all_presets(&self) -> &[ChartPreset] {
        &self.presets
    }

    pub fn find_preset(&self, name: &str) -> Option<&ChartPreset> {
        self.presets.iter().find(|p| p.name == name)
    }

    pub fn get_metrics_for_preset(&self, name: &str) -> Vec<&str> {
        let preset = self.presets.iter().find(|p| p.name == name);
        preset
            .unwrap()
            .chart_types
            .iter()
            .map(|metric| metric.label.as_str())
            .collect()
    }

    // /// Update a preset with new state
    // pub fn update_preset(&mut self, name: &str, updated_preset: ChartPreset) -> bool {
    //     if let Some(index) = self.presets.iter().position(|p| p.name == name) {
    //         self.presets[index] = updated_preset;
    //         true
    //     } else {
    //         false
    //     }
    // }
}
