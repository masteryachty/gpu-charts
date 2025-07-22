//! Configuration schema validation

use crate::{ConfigError, GpuChartsConfig, Result};
use jsonschema::{Draft, JSONSchema};
use serde_json::{json, Value};

/// Schema validator for configuration
pub struct SchemaValidator {
    /// Compiled JSON schema
    schema: JSONSchema,
}

impl SchemaValidator {
    /// Create a new schema validator
    pub fn new() -> Result<Self> {
        let schema_json = Self::generate_schema();

        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_json)
            .map_err(|e| ConfigError::Schema(format!("Failed to compile schema: {}", e)))?;

        Ok(Self { schema })
    }

    /// Validate a configuration
    pub fn validate(&self, config: &GpuChartsConfig) -> Result<()> {
        let config_json = serde_json::to_value(config)
            .map_err(|e| ConfigError::Schema(format!("Failed to serialize config: {}", e)))?;

        let result = self.schema.validate(&config_json);

        if let Err(errors) = result {
            let error_messages: Vec<String> = errors
                .map(|e| format!("{}: {}", e.instance_path, e))
                .collect();

            return Err(ConfigError::Validation(format!(
                "Schema validation failed:\n{}",
                error_messages.join("\n")
            )));
        }

        // Additional custom validation
        self.validate_custom_rules(config)?;

        Ok(())
    }

    /// Generate the JSON schema
    fn generate_schema() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "GPU Charts Configuration",
            "type": "object",
            "required": ["version", "rendering", "data", "performance", "features", "telemetry"],
            "properties": {
                "version": {
                    "type": "string",
                    "pattern": "^\\d+\\.\\d+\\.\\d+$",
                    "description": "Configuration version (semver)"
                },
                "rendering": {
                    "type": "object",
                    "required": ["target_fps", "resolution_scale", "antialiasing", "vsync", "max_render_passes", "chart_settings"],
                    "properties": {
                        "target_fps": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 240,
                            "description": "Target frames per second"
                        },
                        "resolution_scale": {
                            "type": "number",
                            "minimum": 0.25,
                            "maximum": 4.0,
                            "description": "Render resolution scale"
                        },
                        "antialiasing": {
                            "type": "boolean",
                            "description": "Enable antialiasing"
                        },
                        "vsync": {
                            "type": "boolean",
                            "description": "Enable vertical sync"
                        },
                        "max_render_passes": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 16,
                            "description": "Maximum render passes"
                        },
                        "gpu_memory_limit": {
                            "type": ["integer", "null"],
                            "minimum": 0,
                            "description": "GPU memory limit in bytes"
                        },
                        "chart_settings": {
                            "type": "object",
                            "required": ["line", "scatter", "heatmap", "three_d"],
                            "properties": {
                                "line": {
                                    "$ref": "#/definitions/lineChartSettings"
                                },
                                "scatter": {
                                    "$ref": "#/definitions/scatterPlotSettings"
                                },
                                "heatmap": {
                                    "$ref": "#/definitions/heatmapSettings"
                                },
                                "three_d": {
                                    "$ref": "#/definitions/threeDSettings"
                                }
                            }
                        }
                    }
                },
                "data": {
                    "type": "object",
                    "required": ["cache_size", "prefetch_enabled", "prefetch_distance", "compression", "streaming"],
                    "properties": {
                        "cache_size": {
                            "type": "integer",
                            "minimum": 0,
                            "description": "Cache size in bytes"
                        },
                        "prefetch_enabled": {
                            "type": "boolean",
                            "description": "Enable data prefetching"
                        },
                        "prefetch_distance": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 10.0,
                            "description": "Prefetch distance in viewport units"
                        },
                        "compression": {
                            "$ref": "#/definitions/compressionConfig"
                        },
                        "streaming": {
                            "$ref": "#/definitions/streamingConfig"
                        }
                    }
                },
                "performance": {
                    "type": "object",
                    "required": ["gpu_culling", "lod_enabled", "lod_bias", "vertex_compression", "indirect_drawing", "draw_call_batch_size", "auto_tuning"],
                    "properties": {
                        "gpu_culling": {
                            "type": "boolean",
                            "description": "Enable GPU-based culling"
                        },
                        "lod_enabled": {
                            "type": "boolean",
                            "description": "Enable level of detail system"
                        },
                        "lod_bias": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 5.0,
                            "description": "LOD bias (higher = more aggressive)"
                        },
                        "vertex_compression": {
                            "type": "boolean",
                            "description": "Enable vertex compression"
                        },
                        "indirect_drawing": {
                            "type": "boolean",
                            "description": "Enable indirect drawing"
                        },
                        "draw_call_batch_size": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 10000,
                            "description": "Batch size for draw calls"
                        },
                        "auto_tuning": {
                            "$ref": "#/definitions/autoTuningConfig"
                        }
                    }
                },
                "features": {
                    "type": "object",
                    "required": ["scatter_plots", "heatmaps", "three_d_charts", "technical_indicators", "annotations", "custom_shaders", "experimental_features"],
                    "properties": {
                        "scatter_plots": { "type": "boolean" },
                        "heatmaps": { "type": "boolean" },
                        "three_d_charts": { "type": "boolean" },
                        "technical_indicators": { "type": "boolean" },
                        "annotations": { "type": "boolean" },
                        "custom_shaders": { "type": "boolean" },
                        "experimental_features": { "type": "boolean" }
                    }
                },
                "telemetry": {
                    "type": "object",
                    "required": ["enabled", "performance_tracking", "error_reporting", "usage_analytics", "custom_events", "sampling_rate"],
                    "properties": {
                        "enabled": { "type": "boolean" },
                        "performance_tracking": { "type": "boolean" },
                        "error_reporting": { "type": "boolean" },
                        "usage_analytics": { "type": "boolean" },
                        "custom_events": { "type": "boolean" },
                        "sampling_rate": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 1.0,
                            "description": "Telemetry sampling rate (0-1)"
                        }
                    }
                },
                "presets": {
                    "type": ["array", "null"],
                    "items": {
                        "$ref": "#/definitions/presetConfig"
                    }
                }
            },
            "definitions": {
                "lineChartSettings": {
                    "type": "object",
                    "required": ["line_width", "smooth_lines", "area_fill"],
                    "properties": {
                        "line_width": {
                            "type": "number",
                            "minimum": 0.1,
                            "maximum": 10.0
                        },
                        "point_size": {
                            "type": ["number", "null"],
                            "minimum": 0.1,
                            "maximum": 20.0
                        },
                        "smooth_lines": { "type": "boolean" },
                        "area_fill": { "type": "boolean" }
                    }
                },
                "scatterPlotSettings": {
                    "type": "object",
                    "required": ["point_size", "point_shape", "enable_clustering", "cluster_threshold"],
                    "properties": {
                        "point_size": {
                            "type": "number",
                            "minimum": 0.1,
                            "maximum": 20.0
                        },
                        "point_shape": {
                            "type": "string",
                            "enum": ["Circle", "Square", "Triangle", "Diamond", "Cross", "Plus"]
                        },
                        "enable_clustering": { "type": "boolean" },
                        "cluster_threshold": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 100.0
                        }
                    }
                },
                "heatmapSettings": {
                    "type": "object",
                    "required": ["interpolation", "color_scheme", "show_contours", "contour_levels"],
                    "properties": {
                        "interpolation": {
                            "type": "string",
                            "enum": ["Nearest", "Linear", "Cubic", "Lanczos"]
                        },
                        "color_scheme": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 50
                        },
                        "show_contours": { "type": "boolean" },
                        "contour_levels": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100
                        }
                    }
                },
                "threeDSettings": {
                    "type": "object",
                    "required": ["enable_shadows", "camera_type", "lighting_quality", "depth_test"],
                    "properties": {
                        "enable_shadows": { "type": "boolean" },
                        "camera_type": {
                            "type": "string",
                            "enum": ["Orbit", "Free", "Fixed"]
                        },
                        "lighting_quality": {
                            "type": "string",
                            "enum": ["Low", "Medium", "High", "Ultra"]
                        },
                        "depth_test": { "type": "boolean" }
                    }
                },
                "compressionConfig": {
                    "type": "object",
                    "required": ["enabled", "algorithm", "level"],
                    "properties": {
                        "enabled": { "type": "boolean" },
                        "algorithm": {
                            "type": "string",
                            "enum": ["None", "Lz4", "Zstd", "Gzip"]
                        },
                        "level": {
                            "type": "integer",
                            "minimum": 0,
                            "maximum": 22
                        }
                    }
                },
                "streamingConfig": {
                    "type": "object",
                    "required": ["chunk_size", "buffer_size", "enable_backpressure"],
                    "properties": {
                        "chunk_size": {
                            "type": "integer",
                            "minimum": 1024,
                            "maximum": 10485760
                        },
                        "buffer_size": {
                            "type": "integer",
                            "minimum": 1024,
                            "maximum": 1073741824
                        },
                        "enable_backpressure": { "type": "boolean" }
                    }
                },
                "autoTuningConfig": {
                    "type": "object",
                    "required": ["enabled", "profile_duration_ms", "adjustment_threshold", "max_quality_preset", "min_quality_preset"],
                    "properties": {
                        "enabled": { "type": "boolean" },
                        "profile_duration_ms": {
                            "type": "integer",
                            "minimum": 100,
                            "maximum": 60000
                        },
                        "adjustment_threshold": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 1.0
                        },
                        "max_quality_preset": {
                            "type": "string",
                            "enum": ["Potato", "Low", "Medium", "High", "Ultra", "Extreme"]
                        },
                        "min_quality_preset": {
                            "type": "string",
                            "enum": ["Potato", "Low", "Medium", "High", "Ultra", "Extreme"]
                        }
                    }
                },
                "presetConfig": {
                    "type": "object",
                    "required": ["name", "description"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "minLength": 1,
                            "maxLength": 50
                        },
                        "description": {
                            "type": "string",
                            "maxLength": 200
                        },
                        "extends": {
                            "type": ["string", "null"]
                        },
                        "overrides": {
                            "type": "object"
                        }
                    }
                }
            }
        })
    }

    /// Custom validation rules beyond JSON schema
    fn validate_custom_rules(&self, config: &GpuChartsConfig) -> Result<()> {
        // Validate that vsync and target_fps make sense together
        if config.rendering.vsync && config.rendering.target_fps > 60 {
            log::warn!("VSync is enabled but target FPS is set to {}. VSync may limit FPS to display refresh rate.", 
                      config.rendering.target_fps);
        }

        // Validate auto-tuning quality preset range
        if config.performance.auto_tuning.min_quality_preset
            > config.performance.auto_tuning.max_quality_preset
        {
            return Err(ConfigError::Validation(
                "Auto-tuning min_quality_preset cannot be higher than max_quality_preset"
                    .to_string(),
            ));
        }

        // Validate compression settings
        if config.data.compression.enabled
            && config.data.compression.algorithm == crate::CompressionAlgorithm::None
        {
            return Err(ConfigError::Validation(
                "Compression is enabled but algorithm is set to None".to_string(),
            ));
        }

        // Validate streaming buffer size
        if config.data.streaming.buffer_size < config.data.streaming.chunk_size {
            return Err(ConfigError::Validation(
                "Streaming buffer_size must be at least as large as chunk_size".to_string(),
            ));
        }

        // Validate telemetry settings
        if !config.telemetry.enabled
            && (config.telemetry.performance_tracking
                || config.telemetry.error_reporting
                || config.telemetry.usage_analytics
                || config.telemetry.custom_events)
        {
            log::warn!("Telemetry is disabled but some telemetry features are enabled. They will have no effect.");
        }

        // Validate feature dependencies
        if config.features.custom_shaders && !config.features.experimental_features {
            log::warn!("Custom shaders are enabled but experimental features are disabled. Some shader features may not work.");
        }

        Ok(())
    }
}
