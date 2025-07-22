//! Minimal WASM bridge demonstrating Phase 3 configuration integration
//!
//! This is a simplified version that compiles cleanly to WASM and shows
//! how Phase 3 configuration features can be exposed to the React app.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::console;

/// Log a message to the browser console
macro_rules! log {
    ($($t:tt)*) => {
        console::log_1(&format!($($t)*).into());
    };
}

/// Simplified configuration structure demonstrating Phase 3 capabilities
#[derive(Serialize, Deserialize, Clone)]
pub struct ChartConfig {
    // Rendering settings
    pub max_fps: u32,
    pub msaa_samples: u8,
    pub enable_bloom: bool,
    pub enable_fxaa: bool,
    pub texture_filtering: String,

    // Performance settings
    pub chunk_size: u32,
    pub prefetch_distance: u32,
    pub cache_size_mb: u32,

    // Features
    pub scatter_plots: bool,
    pub heatmaps: bool,
    pub three_d_charts: bool,
    pub technical_indicators: bool,

    // Quality preset
    pub quality_preset: String,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            max_fps: 60,
            msaa_samples: 2,
            enable_bloom: false,
            enable_fxaa: true,
            texture_filtering: "Bilinear".to_string(),
            chunk_size: 65536,
            prefetch_distance: 2,
            cache_size_mb: 256,
            scatter_plots: true,
            heatmaps: true,
            three_d_charts: false,
            technical_indicators: true,
            quality_preset: "medium".to_string(),
        }
    }
}

/// Minimal chart system demonstrating Phase 3 configuration
#[wasm_bindgen]
pub struct ChartSystemMinimal {
    config: ChartConfig,
    canvas_id: String,
}

#[wasm_bindgen]
impl ChartSystemMinimal {
    /// Create a new chart system
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: String) -> Self {
        console_error_panic_hook::set_once();
        log!("Initializing ChartSystemMinimal for canvas: {}", canvas_id);

        Self {
            config: ChartConfig::default(),
            canvas_id,
        }
    }

    /// Update configuration from JSON
    #[wasm_bindgen]
    pub fn update_config(&mut self, config_json: &str) -> Result<(), JsValue> {
        match serde_json::from_str::<ChartConfig>(config_json) {
            Ok(new_config) => {
                self.config = new_config;
                log!("Configuration updated successfully");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to parse config: {}", e);
                log!("Error: {}", error_msg);
                Err(JsValue::from_str(&error_msg))
            }
        }
    }

    /// Get current configuration as JSON
    #[wasm_bindgen]
    pub fn get_config(&self) -> String {
        serde_json::to_string(&self.config).unwrap_or_else(|_| "{}".to_string())
    }

    /// Set quality preset
    #[wasm_bindgen]
    pub fn set_quality_preset(&mut self, preset: &str) -> Result<(), JsValue> {
        match preset {
            "ultra" => {
                self.config.max_fps = 144;
                self.config.msaa_samples = 8;
                self.config.enable_bloom = true;
                self.config.enable_fxaa = true;
                self.config.texture_filtering = "Anisotropic16x".to_string();
                self.config.quality_preset = "ultra".to_string();
            }
            "high" => {
                self.config.max_fps = 120;
                self.config.msaa_samples = 4;
                self.config.enable_bloom = true;
                self.config.enable_fxaa = true;
                self.config.texture_filtering = "Anisotropic8x".to_string();
                self.config.quality_preset = "high".to_string();
            }
            "medium" => {
                self.config.max_fps = 60;
                self.config.msaa_samples = 2;
                self.config.enable_bloom = false;
                self.config.enable_fxaa = true;
                self.config.texture_filtering = "Bilinear".to_string();
                self.config.quality_preset = "medium".to_string();
            }
            "low" => {
                self.config.max_fps = 30;
                self.config.msaa_samples = 1;
                self.config.enable_bloom = false;
                self.config.enable_fxaa = false;
                self.config.texture_filtering = "Nearest".to_string();
                self.config.quality_preset = "low".to_string();
            }
            _ => {
                return Err(JsValue::from_str(&format!("Unknown preset: {}", preset)));
            }
        }

        log!("Quality preset changed to: {}", preset);
        Ok(())
    }

    /// Check if a feature is enabled
    #[wasm_bindgen]
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "scatter_plots" => self.config.scatter_plots,
            "heatmaps" => self.config.heatmaps,
            "three_d_charts" => self.config.three_d_charts,
            "technical_indicators" => self.config.technical_indicators,
            _ => false,
        }
    }

    /// Get FPS limit
    #[wasm_bindgen]
    pub fn get_max_fps(&self) -> u32 {
        self.config.max_fps
    }

    /// Set FPS limit
    #[wasm_bindgen]
    pub fn set_max_fps(&mut self, fps: u32) {
        self.config.max_fps = fps;
        log!("Max FPS set to: {}", fps);
    }

    /// Get canvas ID
    #[wasm_bindgen]
    pub fn get_canvas_id(&self) -> String {
        self.canvas_id.clone()
    }

    /// Simulate configuration hot-reload (for demonstration)
    #[wasm_bindgen]
    pub fn simulate_hot_reload(&mut self) {
        log!("Simulating configuration hot-reload...");
        // In the full implementation, this would watch for file changes
        // For now, we just log the action
        log!("Hot-reload simulation complete. Use update_config() to apply changes.");
    }

    /// Get performance metrics (simulated)
    #[wasm_bindgen]
    pub fn get_performance_metrics(&self) -> String {
        let metrics = serde_json::json!({
            "fps": self.config.max_fps,
            "memory_usage_mb": 42,
            "draw_calls": 156,
            "vertices": 1000000,
            "gpu_time_ms": 8.5,
            "cpu_time_ms": 2.3
        });

        metrics.to_string()
    }
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    log!("GPU Charts Phase 3 WASM Bridge initialized");
}
