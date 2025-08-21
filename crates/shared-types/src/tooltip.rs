//! Tooltip state and data structures for GPU-accelerated tooltip rendering

use serde::{Deserialize, Serialize};

/// Represents the state of the tooltip system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipState {
    /// Whether the tooltip is currently active (right mouse button held)
    pub active: bool,
    
    /// X position in screen space (pixels from left)
    pub x_position: f32,
    
    /// Y position in screen space (pixels from top) for mouse cursor
    pub y_position: f32,
    
    /// The timestamp at the current x position (data space)
    pub timestamp: Option<u32>,
    
    /// Labels to display for each data series
    pub labels: Vec<TooltipLabel>,
    
    /// Last update timestamp for throttling (milliseconds)
    pub last_update_ms: f64,
}

impl Default for TooltipState {
    fn default() -> Self {
        Self {
            active: false,
            x_position: 0.0,
            y_position: 0.0,
            timestamp: None,
            labels: Vec::new(),
            last_update_ms: 0.0,
        }
    }
}

/// Represents a single label in the tooltip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipLabel {
    /// The name/key of this data series
    pub series_name: String,
    
    /// The value at the current position
    pub value: f32,
    
    /// Y position in screen space where this label should be drawn
    pub screen_y: f32,
    
    /// Color of this series (RGBA, each 0-1)
    pub color: [f32; 4],
    
    /// Whether this label is currently visible
    pub visible: bool,
    
    /// Index of the data point in the buffer
    pub data_index: u32,
}

/// GPU buffer layout for tooltip rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TooltipVertex {
    /// Position in NDC space (-1 to 1)
    pub position: [f32; 2],
    /// Color (RGBA)
    pub color: [f32; 4],
}

/// GPU buffer for tooltip label data
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TooltipLabelGpu {
    /// Screen position (x, y) in pixels
    pub position: [f32; 2],
    /// Value to display
    pub value: f32,
    /// Color (RGBA)
    pub color: [f32; 4],
    /// Padding for alignment
    pub _padding: f32,
}

/// Configuration for tooltip behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TooltipConfig {
    /// Minimum milliseconds between tooltip updates (for throttling)
    pub update_throttle_ms: f64,
    
    /// Width of the vertical line in pixels
    pub line_width: f32,
    
    /// Padding between stacked labels in pixels
    pub label_padding: f32,
    
    /// Label box padding in pixels
    pub box_padding: f32,
    
    /// Label font size in pixels
    pub font_size: f32,
    
    /// Opacity of label backgrounds (0-1)
    pub background_opacity: f32,
}

impl Default for TooltipConfig {
    fn default() -> Self {
        Self {
            update_throttle_ms: 8.0, // ~120 FPS for more responsive updates
            line_width: 1.0,
            label_padding: 2.0,
            box_padding: 4.0,
            font_size: 12.0,
            background_opacity: 0.9,
        }
    }
}