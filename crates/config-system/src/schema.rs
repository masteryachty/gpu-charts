//! Configuration schema validation

use crate::{ConfigError, GpuChartsConfig, Result};

/// Schema validator for configuration
pub struct SchemaValidator {
    // In WASM, we use manual validation instead of jsonschema
}

impl SchemaValidator {
    /// Create a new schema validator
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// Validate a configuration
    pub fn validate(&self, config: &GpuChartsConfig) -> Result<()> {
        // Perform manual validation for WASM compatibility

        // Validate version format
        if config.version.is_empty() {
            return Err(ConfigError::Schema("Version cannot be empty".to_string()));
        }

        // Validate rendering config
        if config.rendering.target_fps == 0 || config.rendering.target_fps > 240 {
            return Err(ConfigError::Schema(
                "Target FPS must be between 1 and 240".to_string(),
            ));
        }

        if config.rendering.resolution_scale <= 0.0 || config.rendering.resolution_scale > 4.0 {
            return Err(ConfigError::Schema(
                "Resolution scale must be between 0.1 and 4.0".to_string(),
            ));
        }

        // Validate data config
        if config.data.cache_size == 0 {
            return Err(ConfigError::Schema(
                "Cache size must be greater than 0".to_string(),
            ));
        }

        if config.data.prefetch_distance < 0.0 {
            return Err(ConfigError::Schema(
                "Prefetch distance cannot be negative".to_string(),
            ));
        }

        // Validate performance config
        if config.performance.lod_bias < 0.0 || config.performance.lod_bias > 10.0 {
            return Err(ConfigError::Schema(
                "LOD bias must be between 0.0 and 10.0".to_string(),
            ));
        }

        if config.performance.draw_call_batch_size == 0 {
            return Err(ConfigError::Schema(
                "Draw call batch size must be greater than 0".to_string(),
            ));
        }

        // Additional custom validation
        self.validate_custom_rules(config)?;

        Ok(())
    }

    /// Custom validation rules
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

        // Validate telemetry sampling rate
        if config.telemetry.sampling_rate <= 0.0 || config.telemetry.sampling_rate > 1.0 {
            return Err(ConfigError::Validation(
                "Telemetry sampling_rate must be between 0.0 and 1.0".to_string(),
            ));
        }

        Ok(())
    }
}
