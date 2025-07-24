//! Configuration system for GPU Charts
//! Manages presets, quality settings, and performance tuning

use serde::{Deserialize, Serialize};
use shared_types::{PerformanceConfig, QualityPreset};

pub use shared_types::GpuChartsConfig;

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
                    self.config.performance.max_data_points =
                        (self.config.performance.max_data_points as f32 * 0.75) as usize;
                }
            }
        }
        // If we're well above target, we could increase quality
        else if current_fps > target_fps * 1.5 && metrics.gpu_utilization < 0.7 {
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
pub struct RenderingPreset {
    pub name: String,
    pub description: String,
    pub chart_types: Vec<ChartPreset>,
}

/// Chart-specific preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartPreset {
    pub chart_type: String,
    pub data_columns: Vec<String>,
    pub overlays: Vec<String>,
    pub style: serde_json::Value,
}

/// Preset manager for common chart configurations
pub struct PresetManager {
    presets: Vec<RenderingPreset>,
}

impl Default for PresetManager {
    fn default() -> Self {
        Self {
            presets: Self::default_presets(),
        }
    }
}

impl PresetManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn default_presets() -> Vec<RenderingPreset> {
        vec![
            RenderingPreset {
                name: "Line Chart - Ask/Bid".to_string(),
                description: "Simple line chart showing ask and bid prices".to_string(),
                chart_types: vec![ChartPreset {
                    chart_type: "line".to_string(),
                    data_columns: vec![
                        "time".to_string(),
                        "best_ask".to_string(),
                        "best_bid".to_string(),
                    ],
                    overlays: vec![],
                    style: serde_json::json!({
                        "lines": [
                            {"color": [0.0, 0.5, 1.0], "width": 2.0},
                            {"color": [1.0, 0.5, 0.0], "width": 2.0}
                        ]
                    }),
                }],
            },
            RenderingPreset {
                name: "Candlestick - OHLC".to_string(),
                description: "Candlestick chart with OHLC data".to_string(),
                chart_types: vec![ChartPreset {
                    chart_type: "candlestick".to_string(),
                    data_columns: vec![
                        "time".to_string(),
                        "open".to_string(),
                        "high".to_string(),
                        "low".to_string(),
                        "close".to_string(),
                    ],
                    overlays: vec![],
                    style: serde_json::json!({
                        "up_color": [0.0, 1.0, 0.0],
                        "down_color": [1.0, 0.0, 0.0],
                        "body_width": 0.8
                    }),
                }],
            },
            RenderingPreset {
                name: "Volume Bars".to_string(),
                description: "Bar chart showing trading volume".to_string(),
                chart_types: vec![ChartPreset {
                    chart_type: "bar".to_string(),
                    data_columns: vec!["time".to_string(), "volume".to_string()],
                    overlays: vec![],
                    style: serde_json::json!({
                        "color": [0.5, 0.5, 1.0],
                        "width": 0.9
                    }),
                }],
            },
        ]
    }

    pub fn get_preset(&self, name: &str) -> Option<&RenderingPreset> {
        self.presets.iter().find(|p| p.name == name)
    }

    pub fn list_presets(&self) -> Vec<&str> {
        self.presets.iter().map(|p| p.name.as_str()).collect()
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

    #[test]
    fn test_auto_tune() {
        let mut config = ConfigManager::new();
        config.get_config_mut().enable_auto_tuning = true;

        let gpu_info = GpuInfo {
            name: "Test GPU".to_string(),
            memory_mb: 1024,
            compute_units: 16,
        };

        config.auto_tune(&gpu_info);
        assert_eq!(config.get_config().quality_preset, QualityPreset::Low);
    }
}
