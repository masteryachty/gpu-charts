//! Configuration system for GPU Charts
//! Manages presets, quality settings, and performance tuning

use serde::{ Deserialize, Serialize };
use shared_types::{ PerformanceConfig, QualityPreset };

pub use shared_types::GpuChartsConfig;

pub mod presets;

/// Configuration manager for GPU Charts
#[derive(Default)]
pub struct ConfigManager {
    config: GpuChartsConfig,
}

impl ConfigManager {
    /// Create a new configuration manager with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a specific quality preset
    pub fn with_preset(preset: QualityPreset) -> Self {
        let mut manager = Self::new();
        manager.apply_preset(preset);
        manager
    }

    /// Apply a quality preset
    pub fn apply_preset(&mut self, preset: QualityPreset) {
        self.config.quality_preset = preset;

        match preset {
            QualityPreset::Low => {
                self.config.performance = PerformanceConfig {
                    target_fps: 30,
                    max_data_points: 100_000,
                    enable_culling: true,
                    enable_lod: true,
                };
                self.config.visual.line_width = 1.0;
                self.config.visual.show_grid = false;
            }
            QualityPreset::Medium => {
                self.config.performance = PerformanceConfig {
                    target_fps: 60,
                    max_data_points: 1_000_000,
                    enable_culling: true,
                    enable_lod: true,
                };
                self.config.visual.line_width = 2.0;
                self.config.visual.show_grid = true;
            }
            QualityPreset::High => {
                self.config.performance = PerformanceConfig {
                    target_fps: 60,
                    max_data_points: 10_000_000,
                    enable_culling: true,
                    enable_lod: false,
                };
                self.config.visual.line_width = 2.0;
                self.config.visual.show_grid = true;
            }
            QualityPreset::Ultra => {
                self.config.performance = PerformanceConfig {
                    target_fps: 120,
                    max_data_points: 100_000_000,
                    enable_culling: true,
                    enable_lod: false,
                };
                self.config.visual.line_width = 3.0;
                self.config.visual.show_grid = true;
            }
        }
    }

    /// Get the current configuration
    pub fn get_config(&self) -> &GpuChartsConfig {
        &self.config
    }

    /// Get mutable configuration for custom adjustments
    pub fn get_config_mut(&mut self) -> &mut GpuChartsConfig {
        &mut self.config
    }

    /// Auto-tune configuration based on hardware capabilities
    pub fn auto_tune(&mut self, gpu_info: &GpuInfo) {
        if !self.config.enable_auto_tuning {
            return;
        }

        // Simple auto-tuning based on GPU memory
        if gpu_info.memory_mb < 2048 {
            self.apply_preset(QualityPreset::Low);
        } else if gpu_info.memory_mb < 4096 {
            self.apply_preset(QualityPreset::Medium);
        } else if gpu_info.memory_mb < 8192 {
            self.apply_preset(QualityPreset::High);
        } else {
            self.apply_preset(QualityPreset::Ultra);
        }
    }

    /// Adjust quality based on performance metrics
    pub fn adjust_for_performance(&mut self, metrics: &PerformanceMetrics) {
        if !self.config.enable_auto_tuning {
            return;
        }

        let target_fps = self.config.performance.target_fps as f32;
        let current_fps = metrics.average_fps;

        // If we're significantly below target, reduce quality
        if current_fps < target_fps * 0.8 {
            match self.config.quality_preset {
                QualityPreset::Ultra => self.apply_preset(QualityPreset::High),
                QualityPreset::High => self.apply_preset(QualityPreset::Medium),
                QualityPreset::Medium => self.apply_preset(QualityPreset::Low),
                QualityPreset::Low => {
                    // Already at lowest, reduce data points
                    self.config.performance.max_data_points = ((
                        self.config.performance.max_data_points as f32
                    ) * 0.75) as usize;
                }
            }
        } else if
            // If we're well above target, we could increase quality
            current_fps > target_fps * 1.5 &&
            metrics.gpu_utilization < 0.7
        {
            match self.config.quality_preset {
                QualityPreset::Low => self.apply_preset(QualityPreset::Medium),
                QualityPreset::Medium => self.apply_preset(QualityPreset::High),
                QualityPreset::High => self.apply_preset(QualityPreset::Ultra),
                QualityPreset::Ultra => {} // Already at highest
            }
        }
    }
}

/// GPU information for auto-tuning
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub memory_mb: u32,
    pub compute_units: u32,
}

/// Performance metrics for quality adjustment
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub average_fps: f32,
    pub frame_time_ms: f32,
    pub gpu_utilization: f32,
    pub memory_usage_mb: u32,
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    WeightedAverage {
        weights: Vec<f32>,
    },
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

        // Add legacy volume bars preset
        let mut all_presets = presets;
        all_presets.push(ChartPreset {
            name: "Volume Bars".to_string(),
            description: "Bar chart showing trading volume".to_string(),
            chart_types: vec![RenderPreset {
                render_type: RenderType::Bar,
                data_columns: vec![("md".to_string(), "volume".to_string())],
                additional_data_columns: None,
                visible: true,
                label: "Volume".to_string(),
                style: RenderStyle {
                    color: Some([0.5, 0.5, 1.0, 1.0]),
                    color_options: None,
                    size: 0.9, // Bar width
                },
                compute_op: None,
            }],
        });

        Self {
            presets: all_presets,
        }
    }
}

impl PresetManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_preset(&self, name: &str) -> Option<&ChartPreset> {
        self.presets.iter().find(|p| p.name == name)
    }

    pub fn list_presets(&self) -> Vec<&str> {
        self.presets
            .iter()
            .map(|p| p.name.as_str())
            .collect()
    }

    /// Get all presets
    pub fn get_all_presets(&self) -> &[ChartPreset] {
        &self.presets
    }

    /// Update a preset with new state
    pub fn update_preset(&mut self, name: &str, updated_preset: ChartPreset) -> bool {
        if let Some(index) = self.presets.iter().position(|p| p.name == name) {
            self.presets[index] = updated_preset;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_presets() {
        let mut config = ConfigManager::new();

        config.apply_preset(QualityPreset::Low);
        assert_eq!(config.get_config().performance.target_fps, 30);

        config.apply_preset(QualityPreset::Ultra);
        assert_eq!(config.get_config().performance.target_fps, 120);
    }
}
