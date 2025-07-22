//! Configuration validation utilities

use crate::{ConfigError, GpuChartsConfig, Result};
use gpu_charts_shared::ChartConfiguration;

/// Configuration validator with comprehensive checks
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate a complete configuration
    pub fn validate(config: &GpuChartsConfig) -> Result<()> {
        // Validate rendering settings
        Self::validate_rendering(&config.rendering)?;

        // Validate data settings
        Self::validate_data(&config.data)?;

        // Validate performance settings
        Self::validate_performance(&config.performance)?;

        // Validate feature flags
        Self::validate_features(&config.features)?;

        // Validate telemetry
        Self::validate_telemetry(&config.telemetry)?;

        // Cross-field validation
        Self::validate_cross_field(config)?;

        Ok(())
    }

    /// Validate rendering configuration
    fn validate_rendering(rendering: &crate::RenderingConfig) -> Result<()> {
        // Target FPS validation
        if rendering.target_fps == 0 || rendering.target_fps > 240 {
            return Err(ConfigError::Validation(format!(
                "Invalid target_fps: {}. Must be between 1 and 240",
                rendering.target_fps
            )));
        }

        // Resolution scale validation
        if rendering.resolution_scale <= 0.0 || rendering.resolution_scale > 4.0 {
            return Err(ConfigError::Validation(format!(
                "Invalid resolution_scale: {}. Must be between 0.1 and 4.0",
                rendering.resolution_scale
            )));
        }

        // Max render passes validation
        if rendering.max_render_passes == 0 || rendering.max_render_passes > 16 {
            return Err(ConfigError::Validation(format!(
                "Invalid max_render_passes: {}. Must be between 1 and 16",
                rendering.max_render_passes
            )));
        }

        // GPU memory limit validation
        if let Some(limit) = rendering.gpu_memory_limit {
            if limit < 100 * 1024 * 1024 {
                return Err(ConfigError::Validation(format!(
                    "GPU memory limit too low: {} bytes. Minimum is 100MB",
                    limit
                )));
            }
        }

        // Chart settings validation
        Self::validate_chart_settings(&rendering.chart_settings)?;

        Ok(())
    }

    /// Validate chart-specific settings
    fn validate_chart_settings(settings: &crate::ChartSettings) -> Result<()> {
        // Line chart validation
        if settings.line.line_width <= 0.0 || settings.line.line_width > 20.0 {
            return Err(ConfigError::Validation(format!(
                "Invalid line width: {}. Must be between 0.1 and 20.0",
                settings.line.line_width
            )));
        }

        if let Some(point_size) = settings.line.point_size {
            if point_size <= 0.0 || point_size > 50.0 {
                return Err(ConfigError::Validation(format!(
                    "Invalid point size: {}. Must be between 0.1 and 50.0",
                    point_size
                )));
            }
        }

        // Scatter plot validation
        if settings.scatter.point_size <= 0.0 || settings.scatter.point_size > 50.0 {
            return Err(ConfigError::Validation(format!(
                "Invalid scatter point size: {}. Must be between 0.1 and 50.0",
                settings.scatter.point_size
            )));
        }

        if settings.scatter.cluster_threshold < 0.0 || settings.scatter.cluster_threshold > 1000.0 {
            return Err(ConfigError::Validation(format!(
                "Invalid cluster threshold: {}. Must be between 0.0 and 1000.0",
                settings.scatter.cluster_threshold
            )));
        }

        // Heatmap validation
        if settings.heatmap.contour_levels == 0 || settings.heatmap.contour_levels > 100 {
            return Err(ConfigError::Validation(format!(
                "Invalid contour levels: {}. Must be between 1 and 100",
                settings.heatmap.contour_levels
            )));
        }

        if settings.heatmap.color_scheme.is_empty() {
            return Err(ConfigError::Validation(
                "Color scheme cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate data configuration
    fn validate_data(data: &crate::DataConfig) -> Result<()> {
        // Cache size validation
        if data.cache_size < 1024 * 1024 {
            return Err(ConfigError::Validation(format!(
                "Cache size too small: {} bytes. Minimum is 1MB",
                data.cache_size
            )));
        }

        // Prefetch distance validation
        if data.prefetch_distance < 0.0 || data.prefetch_distance > 10.0 {
            return Err(ConfigError::Validation(format!(
                "Invalid prefetch distance: {}. Must be between 0.0 and 10.0",
                data.prefetch_distance
            )));
        }

        // Compression validation
        if data.compression.enabled && data.compression.level > 22 {
            return Err(ConfigError::Validation(format!(
                "Invalid compression level: {}. Maximum is 22",
                data.compression.level
            )));
        }

        // Streaming validation
        if data.streaming.chunk_size < 1024 || data.streaming.chunk_size > 10 * 1024 * 1024 {
            return Err(ConfigError::Validation(format!(
                "Invalid chunk size: {}. Must be between 1KB and 10MB",
                data.streaming.chunk_size
            )));
        }

        if data.streaming.buffer_size < data.streaming.chunk_size {
            return Err(ConfigError::Validation(
                "Streaming buffer size must be at least as large as chunk size".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate performance configuration
    fn validate_performance(performance: &crate::PerformanceConfig) -> Result<()> {
        // LOD bias validation
        if performance.lod_bias < 0.0 || performance.lod_bias > 5.0 {
            return Err(ConfigError::Validation(format!(
                "Invalid LOD bias: {}. Must be between 0.0 and 5.0",
                performance.lod_bias
            )));
        }

        // Draw call batch size validation
        if performance.draw_call_batch_size == 0 || performance.draw_call_batch_size > 10000 {
            return Err(ConfigError::Validation(format!(
                "Invalid draw call batch size: {}. Must be between 1 and 10000",
                performance.draw_call_batch_size
            )));
        }

        // Auto-tuning validation
        let auto_tuning = &performance.auto_tuning;
        if auto_tuning.profile_duration_ms < 100 || auto_tuning.profile_duration_ms > 60000 {
            return Err(ConfigError::Validation(format!(
                "Invalid profile duration: {}ms. Must be between 100ms and 60s",
                auto_tuning.profile_duration_ms
            )));
        }

        if auto_tuning.adjustment_threshold < 0.0 || auto_tuning.adjustment_threshold > 1.0 {
            return Err(ConfigError::Validation(format!(
                "Invalid adjustment threshold: {}. Must be between 0.0 and 1.0",
                auto_tuning.adjustment_threshold
            )));
        }

        if auto_tuning.min_quality_preset > auto_tuning.max_quality_preset {
            return Err(ConfigError::Validation(
                "Auto-tuning min quality preset cannot be higher than max quality preset"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Validate feature flags
    fn validate_features(features: &crate::FeatureFlags) -> Result<()> {
        // Check feature dependencies
        if features.custom_shaders && !features.experimental_features {
            log::warn!("Custom shaders require experimental features to be enabled");
        }

        if features.three_d_charts && !features.scatter_plots {
            log::warn!("3D charts work best with scatter plots enabled");
        }

        Ok(())
    }

    /// Validate telemetry configuration
    fn validate_telemetry(telemetry: &crate::TelemetryConfig) -> Result<()> {
        // Sampling rate validation
        if telemetry.sampling_rate < 0.0 || telemetry.sampling_rate > 1.0 {
            return Err(ConfigError::Validation(format!(
                "Invalid sampling rate: {}. Must be between 0.0 and 1.0",
                telemetry.sampling_rate
            )));
        }

        // Check consistency
        if !telemetry.enabled {
            if telemetry.performance_tracking
                || telemetry.error_reporting
                || telemetry.usage_analytics
                || telemetry.custom_events
            {
                log::warn!(
                    "Telemetry is disabled but sub-features are enabled. They will have no effect."
                );
            }
        }

        Ok(())
    }

    /// Cross-field validation
    fn validate_cross_field(config: &GpuChartsConfig) -> Result<()> {
        // Check memory consistency
        if let Some(gpu_limit) = config.rendering.gpu_memory_limit {
            if config.data.cache_size as u64 > gpu_limit {
                return Err(ConfigError::Validation(format!(
                    "Data cache size ({}) exceeds GPU memory limit ({})",
                    config.data.cache_size, gpu_limit
                )));
            }
        }

        // Check performance settings consistency
        if config.rendering.vsync && config.rendering.target_fps > 60 {
            log::warn!("VSync enabled with target FPS > 60. Actual FPS may be limited by display refresh rate.");
        }

        // Check resolution scale with antialiasing
        if config.rendering.resolution_scale < 1.0 && config.rendering.antialiasing {
            log::warn!(
                "Antialiasing with resolution scale < 1.0 may produce poor quality results."
            );
        }

        // Check indirect drawing with small batch sizes
        if config.performance.indirect_drawing && config.performance.draw_call_batch_size < 10 {
            log::warn!("Indirect drawing with small batch sizes may hurt performance.");
        }

        Ok(())
    }

    /// Validate for specific hardware capabilities
    pub fn validate_for_hardware(
        config: &GpuChartsConfig,
        hardware: &crate::auto_tuning::HardwareCapabilities,
    ) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Check GPU memory requirements
        if let Some(limit) = config.rendering.gpu_memory_limit {
            if limit > hardware.gpu_memory {
                warnings.push(format!(
                    "GPU memory limit ({} GB) exceeds available GPU memory ({} GB)",
                    limit / (1024 * 1024 * 1024),
                    hardware.gpu_memory / (1024 * 1024 * 1024)
                ));
            }
        }

        // Check resolution requirements
        let pixels = hardware.display_width * hardware.display_height;
        let scaled_pixels = (pixels as f32
            * config.rendering.resolution_scale
            * config.rendering.resolution_scale) as u32;

        if scaled_pixels > 16 * 1024 * 1024 {
            warnings.push(format!(
                "Resolution scale {} with display {}x{} results in {} megapixels, which may impact performance",
                config.rendering.resolution_scale,
                hardware.display_width,
                hardware.display_height,
                scaled_pixels / (1024 * 1024)
            ));
        }

        // Check WebGPU limits
        if config.data.streaming.buffer_size as u64
            > hardware.platform.webgpu_limits.max_buffer_size
        {
            warnings.push(format!(
                "Streaming buffer size exceeds WebGPU max buffer size ({} MB)",
                hardware.platform.webgpu_limits.max_buffer_size / (1024 * 1024)
            ));
        }

        // Check compute requirements
        if config.performance.gpu_culling && hardware.gpu_compute_units < 16 {
            warnings.push(
                "GPU culling enabled but GPU has limited compute units. Performance may be suboptimal.".to_string()
            );
        }

        Ok(warnings)
    }

    /// Convert to shared configuration format
    pub fn to_shared_config(config: &GpuChartsConfig) -> Result<ChartConfiguration> {
        // This would convert our comprehensive config to the simpler shared format
        // For now, return a placeholder
        Ok(ChartConfiguration {
            chart_type: gpu_charts_shared::ChartType::Line,
            visual_config: gpu_charts_shared::VisualConfig {
                background_color: [0.0, 0.0, 0.0, 1.0],
                grid_color: [0.2, 0.2, 0.2, 1.0],
                text_color: [1.0, 1.0, 1.0, 1.0],
                margin_percent: 0.05,
                show_grid: true,
                show_axes: true,
            },
            data_handles: vec![],
            overlays: vec![],
        })
    }
}
