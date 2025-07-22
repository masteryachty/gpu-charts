//! Render configuration management and validation

use gpu_charts_shared::{ChartConfiguration, Error, Result, VisualConfig};
use std::collections::HashSet;

/// Extended render configuration with performance hints
#[derive(Debug, Clone)]
pub struct RenderConfiguration {
    pub base_config: ChartConfiguration,
    pub performance_hints: PerformanceHints,
    pub debug_options: DebugOptions,
}

/// Performance hints for optimizing rendering
#[derive(Debug, Clone)]
pub struct PerformanceHints {
    pub target_fps: u32,
    pub max_points_per_series: Option<u32>,
    pub enable_lod: bool,
    pub enable_gpu_culling: bool,
    pub enable_instancing: bool,
    pub prefer_quality: bool,
}

impl Default for PerformanceHints {
    fn default() -> Self {
        Self {
            target_fps: 60,
            max_points_per_series: None,
            enable_lod: true,
            enable_gpu_culling: true,
            enable_instancing: true,
            prefer_quality: false,
        }
    }
}

/// Debug visualization options
#[derive(Debug, Clone, Default)]
pub struct DebugOptions {
    pub show_wireframe: bool,
    pub show_bounding_boxes: bool,
    pub show_performance_overlay: bool,
    pub highlight_draw_calls: bool,
    pub show_lod_levels: bool,
}

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate a chart configuration
    pub fn validate(config: &ChartConfiguration) -> Result<()> {
        // Validate data handles
        if config.data_handles.is_empty() {
            return Err(Error::InvalidConfiguration(
                "No data handles provided".to_string(),
            ));
        }

        // Validate visual config
        Self::validate_visual_config(&config.visual_config)?;

        // Validate overlays
        for overlay in &config.overlays {
            Self::validate_overlay(overlay)?;
        }

        Ok(())
    }

    fn validate_visual_config(config: &VisualConfig) -> Result<()> {
        // Validate colors
        for color in [
            &config.background_color,
            &config.grid_color,
            &config.text_color,
        ] {
            for component in color {
                if !component.is_finite() || *component < 0.0 || *component > 1.0 {
                    return Err(Error::InvalidConfiguration(
                        "Color components must be between 0 and 1".to_string(),
                    ));
                }
            }
        }

        // Validate margin
        if config.margin_percent < 0.0 || config.margin_percent > 0.5 {
            return Err(Error::InvalidConfiguration(
                "Margin percent must be between 0 and 0.5".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_overlay(overlay: &gpu_charts_shared::OverlayConfig) -> Result<()> {
        // Check overlay type is supported
        let supported_types: HashSet<&str> = ["volume", "moving_average", "bollinger_bands"]
            .iter()
            .cloned()
            .collect();

        if !supported_types.contains(overlay.overlay_type.as_str()) {
            return Err(Error::InvalidConfiguration(format!(
                "Unsupported overlay type: {}",
                overlay.overlay_type
            )));
        }

        Ok(())
    }
}

/// Configuration diff for efficient updates
pub struct ConfigurationDiff {
    pub visual_changed: bool,
    pub chart_type_changed: bool,
    pub overlays_changed: bool,
    pub data_handles_changed: bool,
}

impl ConfigurationDiff {
    /// Calculate diff between two configurations
    pub fn calculate(old: &ChartConfiguration, new: &ChartConfiguration) -> Self {
        Self {
            visual_changed: Self::visual_config_changed(&old.visual_config, &new.visual_config),
            chart_type_changed: old.chart_type != new.chart_type,
            overlays_changed: old.overlays.len() != new.overlays.len()
                || old.overlays.iter().zip(&new.overlays).any(|(o, n)| {
                    o.overlay_type != n.overlay_type || o.render_location != n.render_location
                }),
            data_handles_changed: old.data_handles.len() != new.data_handles.len()
                || old
                    .data_handles
                    .iter()
                    .zip(&new.data_handles)
                    .any(|(o, n)| o.id != n.id),
        }
    }

    fn visual_config_changed(old: &VisualConfig, new: &VisualConfig) -> bool {
        old.background_color != new.background_color
            || old.grid_color != new.grid_color
            || old.text_color != new.text_color
            || old.margin_percent != new.margin_percent
            || old.show_grid != new.show_grid
            || old.show_axes != new.show_axes
    }

    /// Check if any significant change requires re-render
    pub fn requires_update(&self) -> bool {
        self.visual_changed
            || self.chart_type_changed
            || self.overlays_changed
            || self.data_handles_changed
    }
}
