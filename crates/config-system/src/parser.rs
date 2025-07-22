//! Configuration file parser for multiple formats

use crate::{ConfigError, GpuChartsConfig, Result};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

/// Configuration format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Yaml,
    Json,
    Toml,
}

/// Configuration parser
pub struct ConfigParser;

impl ConfigParser {
    /// Parse configuration from a file
    pub fn parse_file(path: impl AsRef<Path>) -> Result<GpuChartsConfig> {
        let path = path.as_ref();

        // Read file content
        let content = fs::read_to_string(path).map_err(|e| ConfigError::Io(e))?;

        // Detect format from extension
        let format = Self::detect_format(path)?;

        // Parse based on format
        Self::parse_string(&content, format)
    }

    /// Parse configuration from a string
    pub fn parse_string(content: &str, format: ConfigFormat) -> Result<GpuChartsConfig> {
        match format {
            ConfigFormat::Yaml => Self::parse_yaml(content),
            ConfigFormat::Json => Self::parse_json(content),
            ConfigFormat::Toml => Self::parse_toml(content),
        }
    }

    /// Parse YAML configuration
    fn parse_yaml(content: &str) -> Result<GpuChartsConfig> {
        serde_yaml::from_str(content)
            .map_err(|e| ConfigError::Parse(format!("YAML parse error: {}", e)))
    }

    /// Parse JSON configuration
    fn parse_json(content: &str) -> Result<GpuChartsConfig> {
        serde_json::from_str(content)
            .map_err(|e| ConfigError::Parse(format!("JSON parse error: {}", e)))
    }

    /// Parse TOML configuration
    fn parse_toml(content: &str) -> Result<GpuChartsConfig> {
        toml::from_str(content).map_err(|e| ConfigError::Parse(format!("TOML parse error: {}", e)))
    }

    /// Detect configuration format from file extension
    fn detect_format(path: &Path) -> Result<ConfigFormat> {
        let ext = path.extension().and_then(|e| e.to_str()).ok_or_else(|| {
            ConfigError::Parse("Cannot determine config format from file extension".to_string())
        })?;

        match ext.to_lowercase().as_str() {
            "yaml" | "yml" => Ok(ConfigFormat::Yaml),
            "json" => Ok(ConfigFormat::Json),
            "toml" => Ok(ConfigFormat::Toml),
            _ => Err(ConfigError::Parse(format!(
                "Unsupported config format: {}",
                ext
            ))),
        }
    }

    /// Merge two configurations, with the second overriding the first
    pub fn merge(base: GpuChartsConfig, override_config: GpuChartsConfig) -> GpuChartsConfig {
        // For now, simple replacement. Could implement deep merge later
        override_config
    }

    /// Parse partial configuration for updates
    pub fn parse_partial<T: DeserializeOwned>(content: &str, format: ConfigFormat) -> Result<T> {
        match format {
            ConfigFormat::Yaml => serde_yaml::from_str(content)
                .map_err(|e| ConfigError::Parse(format!("YAML parse error: {}", e))),
            ConfigFormat::Json => serde_json::from_str(content)
                .map_err(|e| ConfigError::Parse(format!("JSON parse error: {}", e))),
            ConfigFormat::Toml => toml::from_str(content)
                .map_err(|e| ConfigError::Parse(format!("TOML parse error: {}", e))),
        }
    }
}

/// Configuration serializer
pub struct ConfigSerializer;

impl ConfigSerializer {
    /// Serialize configuration to a file
    pub fn serialize_file(config: &GpuChartsConfig, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Detect format from extension
        let format = ConfigParser::detect_format(path)?;

        // Serialize to string
        let content = Self::serialize_string(config, format)?;

        // Write to file
        fs::write(path, content).map_err(|e| ConfigError::Io(e))
    }

    /// Serialize configuration to a string
    pub fn serialize_string(config: &GpuChartsConfig, format: ConfigFormat) -> Result<String> {
        match format {
            ConfigFormat::Yaml => Self::serialize_yaml(config),
            ConfigFormat::Json => Self::serialize_json(config),
            ConfigFormat::Toml => Self::serialize_toml(config),
        }
    }

    /// Serialize to YAML
    fn serialize_yaml(config: &GpuChartsConfig) -> Result<String> {
        serde_yaml::to_string(config)
            .map_err(|e| ConfigError::Parse(format!("YAML serialize error: {}", e)))
    }

    /// Serialize to JSON
    fn serialize_json(config: &GpuChartsConfig) -> Result<String> {
        serde_json::to_string_pretty(config)
            .map_err(|e| ConfigError::Parse(format!("JSON serialize error: {}", e)))
    }

    /// Serialize to TOML
    fn serialize_toml(config: &GpuChartsConfig) -> Result<String> {
        toml::to_string_pretty(config)
            .map_err(|e| ConfigError::Parse(format!("TOML serialize error: {}", e)))
    }
}

/// Template expander for configuration files
pub struct TemplateExpander;

impl TemplateExpander {
    /// Expand environment variables in configuration
    pub fn expand_env_vars(content: &str) -> String {
        let mut result = content.to_string();

        // Simple ${VAR} expansion
        let re = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();

        for cap in re.captures_iter(content) {
            if let Some(var_name) = cap.get(1) {
                if let Ok(value) = std::env::var(var_name.as_str()) {
                    result = result.replace(&cap[0], &value);
                }
            }
        }

        result
    }

    /// Expand include directives
    pub fn expand_includes(content: &str, base_path: &Path) -> Result<String> {
        let mut result = content.to_string();

        // Simple !include expansion
        let re = regex::Regex::new(r"!include\s+([^\s]+)").unwrap();

        for cap in re.captures_iter(content) {
            if let Some(file_path) = cap.get(1) {
                let include_path = if file_path.as_str().starts_with('/') {
                    Path::new(file_path.as_str()).to_path_buf()
                } else {
                    base_path.join(file_path.as_str())
                };

                let include_content =
                    fs::read_to_string(&include_path).map_err(|e| ConfigError::Io(e))?;

                result = result.replace(&cap[0], &include_content);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_parsing() {
        let yaml = r#"
version: "1.0.0"
rendering:
  target_fps: 60
  resolution_scale: 1.0
  antialiasing: true
  vsync: true
  max_render_passes: 4
  chart_settings:
    line:
      line_width: 2.0
      smooth_lines: false
      area_fill: false
    scatter:
      point_size: 4.0
      point_shape: Circle
      enable_clustering: true
      cluster_threshold: 10.0
    heatmap:
      interpolation: Linear
      color_scheme: viridis
      show_contours: false
      contour_levels: 10
    three_d:
      enable_shadows: true
      camera_type: Orbit
      lighting_quality: Medium
      depth_test: true
data:
  cache_size: 104857600
  prefetch_enabled: true
  prefetch_distance: 2.0
  compression:
    enabled: true
    algorithm: Lz4
    level: 3
  streaming:
    chunk_size: 65536
    buffer_size: 1048576
    enable_backpressure: true
performance:
  gpu_culling: true
  lod_enabled: true
  lod_bias: 1.0
  vertex_compression: true
  indirect_drawing: true
  draw_call_batch_size: 100
  auto_tuning:
    enabled: true
    profile_duration_ms: 5000
    adjustment_threshold: 0.1
    max_quality_preset: Ultra
    min_quality_preset: Low
features:
  scatter_plots: true
  heatmaps: true
  three_d_charts: true
  technical_indicators: true
  annotations: true
  custom_shaders: false
  experimental_features: false
telemetry:
  enabled: true
  performance_tracking: true
  error_reporting: true
  usage_analytics: false
  custom_events: true
  sampling_rate: 0.1
"#;

        let config = ConfigParser::parse_string(yaml, ConfigFormat::Yaml).unwrap();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.rendering.target_fps, 60);
        assert!(config.features.scatter_plots);
    }

    #[test]
    fn test_json_parsing() {
        let json = r#"{
            "version": "1.0.0",
            "rendering": {
                "target_fps": 120,
                "resolution_scale": 1.5,
                "antialiasing": true,
                "vsync": false,
                "max_render_passes": 8,
                "chart_settings": {
                    "line": {
                        "line_width": 3.0,
                        "smooth_lines": true,
                        "area_fill": false
                    },
                    "scatter": {
                        "point_size": 5.0,
                        "point_shape": "Square",
                        "enable_clustering": false,
                        "cluster_threshold": 20.0
                    },
                    "heatmap": {
                        "interpolation": "Cubic",
                        "color_scheme": "plasma",
                        "show_contours": true,
                        "contour_levels": 15
                    },
                    "three_d": {
                        "enable_shadows": false,
                        "camera_type": "Free",
                        "lighting_quality": "High",
                        "depth_test": true
                    }
                }
            },
            "data": {
                "cache_size": 209715200,
                "prefetch_enabled": false,
                "prefetch_distance": 3.0,
                "compression": {
                    "enabled": false,
                    "algorithm": "None",
                    "level": 0
                },
                "streaming": {
                    "chunk_size": 131072,
                    "buffer_size": 2097152,
                    "enable_backpressure": false
                }
            },
            "performance": {
                "gpu_culling": false,
                "lod_enabled": false,
                "lod_bias": 0.5,
                "vertex_compression": false,
                "indirect_drawing": false,
                "draw_call_batch_size": 50,
                "auto_tuning": {
                    "enabled": false,
                    "profile_duration_ms": 10000,
                    "adjustment_threshold": 0.2,
                    "max_quality_preset": "Extreme",
                    "min_quality_preset": "Medium"
                }
            },
            "features": {
                "scatter_plots": false,
                "heatmaps": false,
                "three_d_charts": false,
                "technical_indicators": false,
                "annotations": false,
                "custom_shaders": true,
                "experimental_features": true
            },
            "telemetry": {
                "enabled": false,
                "performance_tracking": false,
                "error_reporting": false,
                "usage_analytics": true,
                "custom_events": false,
                "sampling_rate": 1.0
            }
        }"#;

        let config = ConfigParser::parse_string(json, ConfigFormat::Json).unwrap();
        assert_eq!(config.rendering.target_fps, 120);
        assert_eq!(config.rendering.resolution_scale, 1.5);
        assert!(config.features.custom_shaders);
    }
}
