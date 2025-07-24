//! Chart configuration types specific to rendering

use serde::{Deserialize, Serialize};

/// Axis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxisConfig {
    pub show_labels: bool,
    pub label_format: LabelFormat,
    pub grid_lines: GridLineConfig,
    pub color: [f32; 4],
}

/// Label format for axes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabelFormat {
    Auto,
    Time,
    Number,
    Scientific,
    Currency,
}

/// Grid line configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridLineConfig {
    pub show: bool,
    pub color: [f32; 4],
    pub width: f32,
    pub dash_pattern: Option<Vec<f32>>,
}

/// Line style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineStyle {
    pub width: f32,
    pub color: [f32; 3],
    pub dash_pattern: Option<Vec<f32>>,
    pub smooth: bool,
}

/// Candlestick style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandlestickStyle {
    pub body_width: f32,
    pub wick_width: f32,
    pub up_color: [f32; 3],
    pub down_color: [f32; 3],
    pub neutral_color: [f32; 3],
}

/// Bar chart style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarStyle {
    pub width: f32,
    pub color: [f32; 3],
    pub border_color: Option<[f32; 3]>,
    pub border_width: f32,
}

/// Area chart style configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaStyle {
    pub fill_color: [f32; 4],
    pub line_color: [f32; 3],
    pub line_width: f32,
    pub gradient: Option<GradientConfig>,
}

/// Gradient configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientConfig {
    pub start_color: [f32; 4],
    pub end_color: [f32; 4],
    pub direction: GradientDirection,
}

/// Gradient direction
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradientDirection {
    Vertical,
    Horizontal,
    Diagonal,
}

/// Chart margin configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChartMargins {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Default for ChartMargins {
    fn default() -> Self {
        Self {
            top: 20.0,
            right: 60.0,
            bottom: 40.0,
            left: 60.0,
        }
    }
}

/// Interaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionConfig {
    pub enable_zoom: bool,
    pub enable_pan: bool,
    pub enable_crosshair: bool,
    pub enable_tooltip: bool,
    pub zoom_sensitivity: f32,
    pub pan_sensitivity: f32,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            enable_zoom: true,
            enable_pan: true,
            enable_crosshair: false,
            enable_tooltip: false,
            zoom_sensitivity: 1.0,
            pan_sensitivity: 1.0,
        }
    }
}
