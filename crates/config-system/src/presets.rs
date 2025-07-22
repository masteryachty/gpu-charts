//! Configuration presets library

use crate::{
    AutoTuningConfig, CameraType, ChartSettings, CompressionAlgorithm, CompressionConfig,
    ConfigError, DataConfig, FeatureFlags, GpuChartsConfig, HeatmapSettings, InterpolationMethod,
    LightingQuality, LineChartSettings, PerformanceConfig, PointShape, QualityPreset,
    RenderingConfig, Result, ScatterPlotSettings, StreamingConfig, TelemetryConfig, ThreeDSettings,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Preset manager for configuration templates
pub struct PresetManager {
    /// Built-in presets
    builtin_presets: HashMap<String, GpuChartsConfig>,

    /// User-defined presets
    user_presets: HashMap<String, GpuChartsConfig>,
}

impl PresetManager {
    /// Create a new preset manager
    pub fn new() -> Self {
        let mut manager = Self {
            builtin_presets: HashMap::new(),
            user_presets: HashMap::new(),
        };

        // Load built-in presets
        manager.load_builtin_presets();

        manager
    }

    /// Get a preset by name
    pub fn get(&self, name: &str) -> Option<&GpuChartsConfig> {
        self.builtin_presets
            .get(name)
            .or_else(|| self.user_presets.get(name))
    }

    /// Add a user preset
    pub fn add_user_preset(&mut self, name: String, config: GpuChartsConfig) -> Result<()> {
        if self.builtin_presets.contains_key(&name) {
            return Err(ConfigError::Validation(format!(
                "Cannot override built-in preset: {}",
                name
            )));
        }

        self.user_presets.insert(name, config);
        Ok(())
    }

    /// Remove a user preset
    pub fn remove_user_preset(&mut self, name: &str) -> Result<()> {
        if self.builtin_presets.contains_key(name) {
            return Err(ConfigError::Validation(format!(
                "Cannot remove built-in preset: {}",
                name
            )));
        }

        self.user_presets.remove(name);
        Ok(())
    }

    /// List all available presets
    pub fn list_presets(&self) -> Vec<PresetInfo> {
        let mut presets = Vec::new();

        // Add built-in presets
        for (name, config) in &self.builtin_presets {
            presets.push(PresetInfo {
                name: name.clone(),
                description: Self::get_preset_description(name),
                is_builtin: true,
                target_fps: config.rendering.target_fps,
                gpu_memory_limit: config.rendering.gpu_memory_limit,
            });
        }

        // Add user presets
        for (name, config) in &self.user_presets {
            presets.push(PresetInfo {
                name: name.clone(),
                description: format!("User-defined preset"),
                is_builtin: false,
                target_fps: config.rendering.target_fps,
                gpu_memory_limit: config.rendering.gpu_memory_limit,
            });
        }

        presets.sort_by(|a, b| a.name.cmp(&b.name));
        presets
    }

    /// Apply a preset with overrides
    pub fn apply_preset(
        &self,
        preset_name: &str,
        overrides: Option<serde_json::Value>,
    ) -> Result<GpuChartsConfig> {
        let base_config = self
            .get(preset_name)
            .ok_or_else(|| ConfigError::Validation(format!("Unknown preset: {}", preset_name)))?
            .clone();

        if let Some(overrides) = overrides {
            // Apply overrides using JSON merge
            let mut base_json = serde_json::to_value(&base_config)
                .map_err(|e| ConfigError::Parse(format!("Serialization error: {}", e)))?;

            Self::merge_json(&mut base_json, overrides);

            serde_json::from_value(base_json)
                .map_err(|e| ConfigError::Parse(format!("Deserialization error: {}", e)))
        } else {
            Ok(base_config)
        }
    }

    /// Load all built-in presets
    fn load_builtin_presets(&mut self) {
        // Performance preset - maximum FPS
        self.builtin_presets
            .insert("performance".to_string(), Self::create_performance_preset());

        // Quality preset - best visual quality
        self.builtin_presets
            .insert("quality".to_string(), Self::create_quality_preset());

        // Balanced preset - good performance and quality
        self.builtin_presets
            .insert("balanced".to_string(), Self::create_balanced_preset());

        // Mobile preset - optimized for low-power devices
        self.builtin_presets
            .insert("mobile".to_string(), Self::create_mobile_preset());

        // Real-time preset - for live data streaming
        self.builtin_presets
            .insert("realtime".to_string(), Self::create_realtime_preset());

        // Big data preset - for handling large datasets
        self.builtin_presets
            .insert("bigdata".to_string(), Self::create_bigdata_preset());

        // Development preset - with debug features
        self.builtin_presets
            .insert("development".to_string(), Self::create_development_preset());

        // Production preset - optimized for deployment
        self.builtin_presets
            .insert("production".to_string(), Self::create_production_preset());
    }

    /// Create performance preset
    fn create_performance_preset() -> GpuChartsConfig {
        GpuChartsConfig {
            version: "1.0.0".to_string(),
            rendering: RenderingConfig {
                target_fps: 144,
                resolution_scale: 0.8,
                antialiasing: false,
                vsync: false,
                max_render_passes: 2,
                gpu_memory_limit: None,
                chart_settings: ChartSettings {
                    line: LineChartSettings {
                        line_width: 1.5,
                        point_size: None,
                        smooth_lines: false,
                        area_fill: false,
                    },
                    scatter: ScatterPlotSettings {
                        point_size: 2.0,
                        point_shape: PointShape::Square,
                        enable_clustering: true,
                        cluster_threshold: 5.0,
                    },
                    heatmap: HeatmapSettings {
                        interpolation: InterpolationMethod::Nearest,
                        color_scheme: "turbo".to_string(),
                        show_contours: false,
                        contour_levels: 5,
                    },
                    three_d: ThreeDSettings {
                        enable_shadows: false,
                        camera_type: CameraType::Orbit,
                        lighting_quality: LightingQuality::Low,
                        depth_test: true,
                    },
                },
            },
            data: DataConfig {
                cache_size: 200 * 1024 * 1024,
                prefetch_enabled: true,
                prefetch_distance: 3.0,
                compression: CompressionConfig {
                    enabled: true,
                    algorithm: CompressionAlgorithm::Lz4,
                    level: 1,
                },
                streaming: StreamingConfig {
                    chunk_size: 131072,
                    buffer_size: 2 * 1024 * 1024,
                    enable_backpressure: true,
                },
            },
            performance: PerformanceConfig {
                gpu_culling: true,
                lod_enabled: true,
                lod_bias: 1.5,
                vertex_compression: true,
                indirect_drawing: true,
                draw_call_batch_size: 200,
                auto_tuning: AutoTuningConfig {
                    enabled: true,
                    profile_duration_ms: 3000,
                    adjustment_threshold: 0.15,
                    max_quality_preset: QualityPreset::High,
                    min_quality_preset: QualityPreset::Potato,
                },
            },
            features: FeatureFlags {
                scatter_plots: true,
                heatmaps: true,
                three_d_charts: false,
                technical_indicators: true,
                annotations: true,
                custom_shaders: false,
                experimental_features: false,
            },
            telemetry: TelemetryConfig {
                enabled: false,
                performance_tracking: false,
                error_reporting: true,
                usage_analytics: false,
                custom_events: false,
                sampling_rate: 0.01,
            },
            presets: None,
        }
    }

    /// Create quality preset
    fn create_quality_preset() -> GpuChartsConfig {
        GpuChartsConfig {
            version: "1.0.0".to_string(),
            rendering: RenderingConfig {
                target_fps: 60,
                resolution_scale: 1.5,
                antialiasing: true,
                vsync: true,
                max_render_passes: 8,
                gpu_memory_limit: Some(8 * 1024 * 1024 * 1024),
                chart_settings: ChartSettings {
                    line: LineChartSettings {
                        line_width: 3.0,
                        point_size: Some(6.0),
                        smooth_lines: true,
                        area_fill: true,
                    },
                    scatter: ScatterPlotSettings {
                        point_size: 6.0,
                        point_shape: PointShape::Circle,
                        enable_clustering: true,
                        cluster_threshold: 15.0,
                    },
                    heatmap: HeatmapSettings {
                        interpolation: InterpolationMethod::Cubic,
                        color_scheme: "viridis".to_string(),
                        show_contours: true,
                        contour_levels: 20,
                    },
                    three_d: ThreeDSettings {
                        enable_shadows: true,
                        camera_type: CameraType::Orbit,
                        lighting_quality: LightingQuality::Ultra,
                        depth_test: true,
                    },
                },
            },
            data: DataConfig {
                cache_size: 500 * 1024 * 1024,
                prefetch_enabled: true,
                prefetch_distance: 2.0,
                compression: CompressionConfig {
                    enabled: false,
                    algorithm: CompressionAlgorithm::None,
                    level: 0,
                },
                streaming: StreamingConfig {
                    chunk_size: 262144,
                    buffer_size: 4 * 1024 * 1024,
                    enable_backpressure: true,
                },
            },
            performance: PerformanceConfig {
                gpu_culling: true,
                lod_enabled: true,
                lod_bias: 0.5,
                vertex_compression: false,
                indirect_drawing: true,
                draw_call_batch_size: 50,
                auto_tuning: AutoTuningConfig {
                    enabled: false,
                    profile_duration_ms: 5000,
                    adjustment_threshold: 0.2,
                    max_quality_preset: QualityPreset::Extreme,
                    min_quality_preset: QualityPreset::High,
                },
            },
            features: FeatureFlags {
                scatter_plots: true,
                heatmaps: true,
                three_d_charts: true,
                technical_indicators: true,
                annotations: true,
                custom_shaders: true,
                experimental_features: true,
            },
            telemetry: TelemetryConfig {
                enabled: true,
                performance_tracking: true,
                error_reporting: true,
                usage_analytics: false,
                custom_events: true,
                sampling_rate: 0.1,
            },
            presets: None,
        }
    }

    /// Create balanced preset
    fn create_balanced_preset() -> GpuChartsConfig {
        GpuChartsConfig::default()
    }

    /// Create mobile preset
    fn create_mobile_preset() -> GpuChartsConfig {
        GpuChartsConfig {
            version: "1.0.0".to_string(),
            rendering: RenderingConfig {
                target_fps: 30,
                resolution_scale: 0.75,
                antialiasing: false,
                vsync: true,
                max_render_passes: 2,
                gpu_memory_limit: Some(2 * 1024 * 1024 * 1024),
                chart_settings: ChartSettings {
                    line: LineChartSettings {
                        line_width: 2.0,
                        point_size: None,
                        smooth_lines: false,
                        area_fill: false,
                    },
                    scatter: ScatterPlotSettings {
                        point_size: 3.0,
                        point_shape: PointShape::Square,
                        enable_clustering: true,
                        cluster_threshold: 3.0,
                    },
                    heatmap: HeatmapSettings {
                        interpolation: InterpolationMethod::Nearest,
                        color_scheme: "turbo".to_string(),
                        show_contours: false,
                        contour_levels: 5,
                    },
                    three_d: ThreeDSettings {
                        enable_shadows: false,
                        camera_type: CameraType::Fixed,
                        lighting_quality: LightingQuality::Low,
                        depth_test: true,
                    },
                },
            },
            data: DataConfig {
                cache_size: 50 * 1024 * 1024,
                prefetch_enabled: false,
                prefetch_distance: 1.0,
                compression: CompressionConfig {
                    enabled: true,
                    algorithm: CompressionAlgorithm::Lz4,
                    level: 1,
                },
                streaming: StreamingConfig {
                    chunk_size: 32768,
                    buffer_size: 512 * 1024,
                    enable_backpressure: true,
                },
            },
            performance: PerformanceConfig {
                gpu_culling: true,
                lod_enabled: true,
                lod_bias: 2.0,
                vertex_compression: true,
                indirect_drawing: false,
                draw_call_batch_size: 50,
                auto_tuning: AutoTuningConfig {
                    enabled: true,
                    profile_duration_ms: 5000,
                    adjustment_threshold: 0.1,
                    max_quality_preset: QualityPreset::Low,
                    min_quality_preset: QualityPreset::Potato,
                },
            },
            features: FeatureFlags {
                scatter_plots: true,
                heatmaps: false,
                three_d_charts: false,
                technical_indicators: true,
                annotations: false,
                custom_shaders: false,
                experimental_features: false,
            },
            telemetry: TelemetryConfig {
                enabled: false,
                performance_tracking: false,
                error_reporting: true,
                usage_analytics: false,
                custom_events: false,
                sampling_rate: 0.01,
            },
            presets: None,
        }
    }

    /// Create real-time preset
    fn create_realtime_preset() -> GpuChartsConfig {
        let mut config = Self::create_performance_preset();
        config.data.streaming.chunk_size = 16384;
        config.data.streaming.buffer_size = 256 * 1024;
        config.data.prefetch_enabled = false;
        config.data.cache_size = 10 * 1024 * 1024;
        config.rendering.target_fps = 60;
        config.rendering.vsync = true;
        config
    }

    /// Create big data preset
    fn create_bigdata_preset() -> GpuChartsConfig {
        GpuChartsConfig {
            version: "1.0.0".to_string(),
            rendering: RenderingConfig {
                target_fps: 30,
                resolution_scale: 1.0,
                antialiasing: false,
                vsync: true,
                max_render_passes: 4,
                gpu_memory_limit: Some(16 * 1024 * 1024 * 1024),
                chart_settings: ChartSettings::default(),
            },
            data: DataConfig {
                cache_size: 2 * 1024 * 1024 * 1024,
                prefetch_enabled: true,
                prefetch_distance: 5.0,
                compression: CompressionConfig {
                    enabled: true,
                    algorithm: CompressionAlgorithm::Zstd,
                    level: 6,
                },
                streaming: StreamingConfig {
                    chunk_size: 1024 * 1024,
                    buffer_size: 16 * 1024 * 1024,
                    enable_backpressure: true,
                },
            },
            performance: PerformanceConfig {
                gpu_culling: true,
                lod_enabled: true,
                lod_bias: 1.0,
                vertex_compression: true,
                indirect_drawing: true,
                draw_call_batch_size: 500,
                auto_tuning: AutoTuningConfig::default(),
            },
            features: FeatureFlags {
                scatter_plots: true,
                heatmaps: true,
                three_d_charts: false,
                technical_indicators: false,
                annotations: false,
                custom_shaders: false,
                experimental_features: false,
            },
            telemetry: TelemetryConfig::default(),
            presets: None,
        }
    }

    /// Create development preset
    fn create_development_preset() -> GpuChartsConfig {
        let mut config = Self::create_balanced_preset();
        config.telemetry.enabled = true;
        config.telemetry.performance_tracking = true;
        config.telemetry.error_reporting = true;
        config.telemetry.custom_events = true;
        config.telemetry.sampling_rate = 1.0;
        config.features.experimental_features = true;
        config.features.custom_shaders = true;
        config
    }

    /// Create production preset
    fn create_production_preset() -> GpuChartsConfig {
        let mut config = Self::create_balanced_preset();
        config.telemetry.sampling_rate = 0.01;
        config.telemetry.usage_analytics = false;
        config.features.experimental_features = false;
        config.features.custom_shaders = false;
        config.performance.auto_tuning.enabled = true;
        config
    }

    /// Get preset description
    fn get_preset_description(name: &str) -> String {
        match name {
            "performance" => "Maximum FPS with reduced visual quality".to_string(),
            "quality" => "Best visual quality with stable 60 FPS".to_string(),
            "balanced" => "Good balance of performance and quality".to_string(),
            "mobile" => "Optimized for low-power mobile devices".to_string(),
            "realtime" => "Optimized for real-time data streaming".to_string(),
            "bigdata" => "Optimized for large datasets (>1B points)".to_string(),
            "development" => "Development mode with all debug features".to_string(),
            "production" => "Production-ready with stability focus".to_string(),
            _ => "Custom preset".to_string(),
        }
    }

    /// Merge JSON values recursively
    fn merge_json(base: &mut serde_json::Value, overrides: serde_json::Value) {
        match (base, overrides) {
            (serde_json::Value::Object(base_map), serde_json::Value::Object(override_map)) => {
                for (key, value) in override_map {
                    match base_map.get_mut(&key) {
                        Some(base_value) => Self::merge_json(base_value, value),
                        None => {
                            base_map.insert(key, value);
                        }
                    }
                }
            }
            (base, override_value) => {
                *base = override_value;
            }
        }
    }
}

/// Preset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetInfo {
    pub name: String,
    pub description: String,
    pub is_builtin: bool,
    pub target_fps: u32,
    pub gpu_memory_limit: Option<u64>,
}
