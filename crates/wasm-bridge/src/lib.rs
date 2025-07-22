//! JavaScript/WASM bridge for GPU Charts
//!
//! This crate provides the main entry point for the web application,
//! orchestrating the data manager and renderer modules.

// Use simplified version for initial integration
#[path = "lib_simple.rs"]
mod lib_simple;
pub use lib_simple::*;

// Full version commented out until dependency issues are resolved
/*

use gpu_charts_shared::{ChartConfiguration, DataHandle, DataRequest, Error, Result};
use gpu_charts_config::{GpuChartsConfig, HotReloadManager};
use gpu_charts_integration::{SystemIntegration, UnifiedApi};
use wasm_bindgen::prelude::*;
use web_sys::console;
use std::sync::Arc;

/// Log a message to the browser console
macro_rules! log {
    ($($t:tt)*) => {
        console::log_1(&format!($($t)*).into());
    };
}

/// Main chart system that orchestrates data and rendering
#[wasm_bindgen]
pub struct ChartSystem {
    data_manager: gpu_charts_data::DataManager,
    renderer: Option<gpu_charts_renderer::Renderer>,
    canvas_id: String,
    config_manager: Arc<HotReloadManager>,
    system_integration: Arc<SystemIntegration>,
    unified_api: Arc<UnifiedApi>,
}

#[wasm_bindgen]
impl ChartSystem {
    /// Initialize the chart system
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_id: String, base_url: String) -> Result<ChartSystem> {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        // Initialize console logging
        console_log::init_with_level(log::Level::Debug).expect("Failed to initialize logger");

        log!("Initializing ChartSystem for canvas: {}", canvas_id);

        // Create renderer
        let renderer = gpu_charts_renderer::Renderer::new(&canvas_id).await?;

        // Note: In a real implementation, we'd need to get device/queue from renderer
        // For now, this is a placeholder
        let device = unsafe { std::mem::zeroed() };
        let queue = unsafe { std::mem::zeroed() };

        // Create data manager
        let data_manager = gpu_charts_data::DataManager::new(&device, &queue, base_url);

        // Initialize configuration system
        let default_config = GpuChartsConfig::default();
        let config_manager = Arc::new(HotReloadManager::new(default_config, |_| Ok(())));

        // Initialize system integration
        let system_integration = Arc::new(SystemIntegration::new(config_manager.clone())?);

        // Create unified API
        let unified_api = Arc::new(UnifiedApi::new(system_integration.clone()));

        Ok(Self {
            data_manager,
            renderer: Some(renderer),
            canvas_id,
            config_manager,
            system_integration,
            unified_api,
        })
    }

    /// Update chart configuration and fetch necessary data
    #[wasm_bindgen]
    pub async fn update_chart(
        &mut self,
        chart_type: &str,
        symbol: &str,
        start_time: u64,
        end_time: u64,
        config_json: &str,
    ) -> Result<()> {
        log!(
            "Updating chart: {} for {} ({} - {})",
            chart_type,
            symbol,
            start_time,
            end_time
        );

        // Parse configuration
        let config: ChartConfiguration = serde_json::from_str(config_json)
            .map_err(|e| Error::InvalidConfiguration(e.to_string()))?;

        // Build data request
        let data_request = DataRequest {
            symbol: symbol.to_string(),
            time_range: gpu_charts_shared::TimeRange::new(start_time, end_time),
            columns: self.determine_columns(&config),
            aggregation: self.determine_aggregation(&config),
            max_points: None,
        };

        // Fetch data
        let request_json = serde_json::to_string(&data_request).unwrap();
        let handle_json = self.data_manager.fetch_data(&request_json).await?;
        let handle: DataHandle = serde_json::from_str(&handle_json).unwrap();

        // Update renderer configuration
        if let Some(renderer) = &mut self.renderer {
            renderer.update_config(config_json)?;
            renderer.set_data_handles(&serde_json::to_string(&vec![handle]).unwrap())?;
        }

        Ok(())
    }

    /// Render a frame
    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<()> {
        if let Some(renderer) = &mut self.renderer {
            renderer.render()?;
        }
        Ok(())
    }

    /// Handle window resize
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        log!("Resizing to {}x{}", width, height);
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(width, height);
        }
    }

    /// Get performance statistics
    #[wasm_bindgen]
    pub fn get_stats(&self) -> String {
        let data_stats = self.data_manager.get_stats();
        let render_stats = self
            .renderer
            .as_ref()
            .map(|r| r.get_stats())
            .unwrap_or_else(|| "{}".to_string());

        // Combine stats
        format!(r#"{{"data": {}, "render": {}}}"#, data_stats, render_stats)
    }

    /// Update configuration from JSON
    #[wasm_bindgen]
    pub fn update_config(&self, config_json: &str) -> Result<()> {
        let new_config: GpuChartsConfig = serde_json::from_str(config_json)
            .map_err(|e| Error::InvalidConfiguration(e.to_string()))?;

        self.config_manager.update_config(new_config);
        Ok(())
    }

    /// Get current configuration as JSON
    #[wasm_bindgen]
    pub fn get_config(&self) -> String {
        let config = self.config_manager.get_current();
        serde_json::to_string(&*config).unwrap_or_else(|_| "{}".to_string())
    }

    /// Enable hot-reload for configuration file
    #[wasm_bindgen]
    pub async fn enable_config_hot_reload(&self, _file_path: &str) -> Result<()> {
        // File watching not supported in WASM
        // Configuration updates must be done through update_config()
        Ok(())
    }

    /// Get system performance metrics
    #[wasm_bindgen]
    pub fn get_performance_metrics(&self) -> String {
        self.unified_api.get_performance_metrics()
    }

    /// Clean up resources
    #[wasm_bindgen]
    pub fn destroy(&mut self) {
        log!("Destroying ChartSystem");
        self.renderer = None;
        // Data manager cleanup happens automatically
    }
}

impl ChartSystem {
    fn determine_columns(&self, config: &ChartConfiguration) -> Vec<String> {
        // Determine required columns based on chart type
        match config.chart_type {
            gpu_charts_shared::ChartType::Candlestick => {
                vec!["time".to_string(), "price".to_string()]
            }
            _ => {
                vec![
                    "time".to_string(),
                    "price".to_string(),
                    "volume".to_string(),
                ]
            }
        }
    }

    fn determine_aggregation(
        &self,
        config: &ChartConfiguration,
    ) -> Option<gpu_charts_shared::AggregationConfig> {
        match config.chart_type {
            gpu_charts_shared::ChartType::Candlestick => {
                Some(gpu_charts_shared::AggregationConfig {
                    aggregation_type: gpu_charts_shared::AggregationType::Ohlc,
                    timeframe: 60, // Default 1 minute
                })
            }
            _ => None,
        }
    }
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Export shared types for TypeScript
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
// Re-export shared types
export * from './gpu_charts_shared';
"#;
*/
