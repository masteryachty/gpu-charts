//! Simplified WASM bridge for Phase 3 integration
//!
//! This provides a minimal integration that compiles to WASM
//! while exposing Phase 3 configuration capabilities.

use gpu_charts_config::{GpuChartsConfig, HotReloadManager};
use gpu_charts_shared::{Error, Result};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::console;

/// Log a message to the browser console
macro_rules! log {
    ($($t:tt)*) => {
        console::log_1(&format!($($t)*).into());
    };
}

/// Simplified chart system that provides Phase 3 configuration
#[wasm_bindgen]
pub struct ChartSystemSimple {
    config_manager: Arc<HotReloadManager>,
    canvas_id: String,
}

#[wasm_bindgen]
impl ChartSystemSimple {
    /// Initialize the chart system
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: String) -> Result<ChartSystemSimple> {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        log!("Initializing ChartSystemSimple for canvas: {}", canvas_id);

        // Initialize configuration system
        let default_config = GpuChartsConfig::default();
        let config_manager = Arc::new(HotReloadManager::new(default_config, |_| Ok(())));

        Ok(Self {
            config_manager,
            canvas_id,
        })
    }

    /// Update configuration from JSON
    #[wasm_bindgen]
    pub fn update_config(&self, config_json: &str) -> Result<()> {
        let new_config: GpuChartsConfig = serde_json::from_str(config_json)
            .map_err(|e| Error::InvalidConfiguration(e.to_string()))?;

        self.config_manager.update_config(new_config);
        log!("Configuration updated successfully");
        Ok(())
    }

    /// Get current configuration as JSON
    #[wasm_bindgen]
    pub fn get_config(&self) -> String {
        let config = self.config_manager.current();
        serde_json::to_string(&*config).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get rendering configuration
    #[wasm_bindgen]
    pub fn get_rendering_config(&self) -> String {
        let config = self.config_manager.current();
        serde_json::to_string(&config.rendering).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get performance configuration
    #[wasm_bindgen]
    pub fn get_performance_config(&self) -> String {
        let config = self.config_manager.current();
        serde_json::to_string(&config.performance).unwrap_or_else(|_| "{}".to_string())
    }

    /// Check if a feature is enabled
    #[wasm_bindgen]
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        let config = self.config_manager.current();
        match feature {
            "scatter_plots" => config.features.scatter_plots,
            "heatmaps" => config.features.heatmaps,
            "three_d_charts" => config.features.three_d_charts,
            "technical_indicators" => config.features.technical_indicators,
            "annotations" => config.features.annotations,
            "custom_shaders" => config.features.custom_shaders,
            _ => false,
        }
    }

    /// Set render quality preset
    #[wasm_bindgen]
    pub fn set_quality_preset(&self, preset: &str) -> Result<()> {
        let mut config = (*self.config_manager.current()).clone();

        match preset {
            "ultra" => {
                config.rendering.max_fps = 144;
                config.rendering.msaa_samples = 8;
                config.rendering.enable_bloom = true;
                config.rendering.enable_fxaa = true;
                config.rendering.enable_shadows = true;
                config.rendering.texture_filtering =
                    gpu_charts_config::TextureFiltering::Anisotropic16x;
            }
            "high" => {
                config.rendering.max_fps = 120;
                config.rendering.msaa_samples = 4;
                config.rendering.enable_bloom = true;
                config.rendering.enable_fxaa = true;
                config.rendering.enable_shadows = false;
                config.rendering.texture_filtering =
                    gpu_charts_config::TextureFiltering::Anisotropic8x;
            }
            "medium" => {
                config.rendering.max_fps = 60;
                config.rendering.msaa_samples = 2;
                config.rendering.enable_bloom = false;
                config.rendering.enable_fxaa = true;
                config.rendering.enable_shadows = false;
                config.rendering.texture_filtering =
                    gpu_charts_config::TextureFiltering::Anisotropic4x;
            }
            "low" => {
                config.rendering.max_fps = 30;
                config.rendering.msaa_samples = 1;
                config.rendering.enable_bloom = false;
                config.rendering.enable_fxaa = false;
                config.rendering.enable_shadows = false;
                config.rendering.texture_filtering = gpu_charts_config::TextureFiltering::Bilinear;
            }
            _ => {
                return Err(Error::InvalidConfiguration(format!(
                    "Unknown preset: {}",
                    preset
                )))
            }
        }

        self.config_manager.update_config(config);
        Ok(())
    }

    /// Get canvas ID
    #[wasm_bindgen]
    pub fn get_canvas_id(&self) -> String {
        self.canvas_id.clone()
    }
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
