//! Clean WASM bridge implementation following NEW_ARCHITECTURE.md
//!
//! This is a simplified version that orchestrates data-manager and renderer
//! without the complex system-integration layer.

use gpu_charts_config::GpuChartsConfig;
use gpu_charts_shared::{DataHandle, DataRequest, Error, Result};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::console;

/// Log a message to the browser console
macro_rules! log {
    ($($t:tt)*) => {
        console::log_1(&format!($($t)*).into());
    };
}

/// Convert Result to JsValue for wasm_bindgen
fn to_js_result<T>(result: Result<T>) -> std::result::Result<T, JsValue> {
    result.map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Main chart system that orchestrates data and rendering
#[wasm_bindgen]
pub struct ChartSystem {
    #[allow(dead_code)]
    device: Arc<wgpu::Device>,
    #[allow(dead_code)]
    queue: Arc<wgpu::Queue>,
    #[allow(dead_code)]
    surface: Option<wgpu::Surface<'static>>,
    data_manager: gpu_charts_data::DataManager,
    renderer: Option<gpu_charts_renderer::Renderer>,
    #[allow(dead_code)]
    canvas_id: String,
    config: GpuChartsConfig,
}

#[wasm_bindgen]
impl ChartSystem {
    /// Initialize the chart system
    #[wasm_bindgen(constructor)]
    pub async fn new(
        canvas_id: String,
        base_url: String,
    ) -> std::result::Result<ChartSystem, JsValue> {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();

        log!("Initializing ChartSystem for canvas: {}", canvas_id);

        // Initialize WebGPU with surface
        let (device, queue, surface) = to_js_result(Self::init_webgpu(&canvas_id).await)?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Get canvas dimensions
        let (width, height) = to_js_result(Self::get_canvas_size(&canvas_id))?;

        // Create data manager
        let data_manager =
            gpu_charts_data::DataManager::new_with_device(device.clone(), queue.clone(), base_url);

        // Create renderer with surface
        let renderer = to_js_result(gpu_charts_renderer::Renderer::new_with_device(
            device.clone(),
            queue.clone(),
            surface,
            width,
            height,
        ))?;

        // Initialize with default config
        let config = GpuChartsConfig::default();

        Ok(Self {
            device,
            queue,
            surface: None, // Surface is now owned by renderer
            data_manager,
            renderer: Some(renderer),
            canvas_id,
            config,
        })
    }

    /// Initialize WebGPU device and queue
    async fn init_webgpu(
        canvas_id: &str,
    ) -> Result<(wgpu::Device, wgpu::Queue, wgpu::Surface<'static>)> {
        // Use the webgpu_init module that's already in the crate
        let (device, queue, surface) = crate::webgpu_init::initialize_webgpu(canvas_id)
            .await
            .map_err(|e| Error::GpuError(format!("WebGPU init failed: {:?}", e)))?;

        Ok((device, queue, surface))
    }

    /// Get canvas dimensions
    fn get_canvas_size(canvas_id: &str) -> Result<(u32, u32)> {
        use web_sys::window;

        let window = window().ok_or_else(|| Error::GpuError("No window".to_string()))?;
        let document = window
            .document()
            .ok_or_else(|| Error::GpuError("No document".to_string()))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| Error::GpuError(format!("Canvas {} not found", canvas_id)))?;

        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into()
            .map_err(|_| Error::GpuError("Not a canvas element".to_string()))?;

        let width = canvas.client_width() as u32;
        let height = canvas.client_height() as u32;

        Ok((width, height))
    }

    /// Update chart with new data
    #[wasm_bindgen]
    pub async fn update_chart(
        &mut self,
        chart_type: &str,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> std::result::Result<(), JsValue> {
        log!(
            "Updating chart: {} for {} ({} - {})",
            chart_type,
            symbol,
            start_time,
            end_time
        );

        // Build data request
        let data_request = DataRequest {
            symbol: symbol.to_string(),
            time_range: gpu_charts_shared::TimeRange::new(start_time, end_time),
            columns: vec!["time".to_string(), "price".to_string()],
            aggregation: None,
            max_points: None,
        };

        // Fetch data
        let request_json = serde_json::to_string(&data_request).unwrap();
        let handle_json = to_js_result(self.data_manager.fetch_data(&request_json).await)?;
        let handle: DataHandle = serde_json::from_str(&handle_json).unwrap();

        log!("Data fetched, handle: {:?}", handle.id);

        // Update renderer with data
        if let Some(renderer) = &mut self.renderer {
            // Configure chart based on type
            let chart_config = match chart_type {
                "line" => gpu_charts_shared::ChartConfiguration {
                    chart_type: gpu_charts_shared::ChartType::Line,
                    data_handles: vec![handle.clone()],
                    visual_config: gpu_charts_shared::VisualConfig {
                        background_color: [0.05, 0.05, 0.05, 1.0],
                        grid_color: [0.2, 0.2, 0.2, 0.3],
                        text_color: [0.9, 0.9, 0.9, 1.0],
                        margin_percent: 0.1,
                        show_grid: true,
                        show_axes: true,
                    },
                    overlays: vec![],
                },
                "candlestick" => gpu_charts_shared::ChartConfiguration {
                    chart_type: gpu_charts_shared::ChartType::Candlestick,
                    data_handles: vec![handle.clone()],
                    visual_config: gpu_charts_shared::VisualConfig {
                        background_color: [0.05, 0.05, 0.05, 1.0],
                        grid_color: [0.2, 0.2, 0.2, 0.3],
                        text_color: [0.9, 0.9, 0.9, 1.0],
                        margin_percent: 0.1,
                        show_grid: true,
                        show_axes: true,
                    },
                    overlays: vec![],
                },
                _ => {
                    return Err(JsValue::from_str(&format!(
                        "Unknown chart type: {}",
                        chart_type
                    )))
                }
            };

            // Update renderer configuration
            to_js_result(renderer.update_config(chart_config))?;

            // Get GPU buffers from data manager and register with renderer
            if let Some(gpu_buffer_set) = self.data_manager.get_buffer_set(&handle.id) {
                let renderer_buffer_set = gpu_charts_renderer::GpuBufferSet {
                    buffers: gpu_buffer_set.buffers,
                    metadata: handle.metadata.clone(),
                };

                renderer.register_buffer_set(handle.clone(), Arc::new(renderer_buffer_set));
                log!(
                    "Registered GPU buffers with renderer for handle: {:?}",
                    handle.id
                );
            } else {
                log!(
                    "Warning: Could not get GPU buffers for handle: {:?}",
                    handle.id
                );
            }

            log!("Chart configuration updated successfully");
        }

        Ok(())
    }

    /// Render a frame
    #[wasm_bindgen]
    pub fn render(&mut self) -> std::result::Result<(), JsValue> {
        log!("ChartSystem.render() called");
        if let Some(renderer) = &mut self.renderer {
            log!("Calling renderer.render()");
            to_js_result(renderer.render())?;
            log!("renderer.render() completed successfully");
        } else {
            log!("Warning: No renderer available");
        }
        Ok(())
    }

    /// Update configuration
    #[wasm_bindgen]
    pub fn update_config(&mut self, config_json: &str) -> std::result::Result<(), JsValue> {
        let new_config: GpuChartsConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;

        self.config = new_config;

        // TODO: Apply config to renderer
        // if let Some(renderer) = &mut self.renderer {
        //     renderer.update_config(&self.config)?;
        // }

        Ok(())
    }

    /// Get current configuration as JSON
    #[wasm_bindgen]
    pub fn get_config(&self) -> String {
        serde_json::to_string(&self.config).unwrap_or_else(|_| "{}".to_string())
    }

    /// Resize the canvas
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
        let renderer_stats = self
            .renderer
            .as_ref()
            .map(|r| r.get_stats())
            .unwrap_or_else(|| serde_json::json!({}));

        let stats = serde_json::json!({
            "data_manager": serde_json::from_str::<serde_json::Value>(&data_stats).unwrap_or(serde_json::json!({})),
            "renderer": renderer_stats,
        });

        stats.to_string()
    }

    /// Clean up resources
    #[wasm_bindgen]
    pub fn destroy(&mut self) {
        log!("Destroying ChartSystem");
        self.renderer = None;
    }

    /// Handle mouse wheel event for zoom
    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&mut self, delta_y: f32, x: f32, y: f32) {
        log!("Mouse wheel: delta_y={}, x={}, y={}", delta_y, x, y);

        // TODO: Implement zoom logic
        // For now, just update viewport scale
        if let Some(_renderer) = &mut self.renderer {
            // Basic zoom implementation - scale factor based on wheel delta
            let _zoom_factor = if delta_y < 0.0 { 1.1 } else { 0.9 };
            // TODO: Apply zoom to viewport at (x, y) position
        }
    }

    /// Handle mouse move event
    #[wasm_bindgen]
    pub fn handle_mouse_move(&mut self, _x: f32, _y: f32) {
        // TODO: Implement crosshair/tooltip logic
    }

    /// Handle mouse click event  
    #[wasm_bindgen]
    pub fn handle_mouse_click(&mut self, x: f32, y: f32, pressed: bool) {
        if pressed {
            log!("Mouse down at: {}, {}", x, y);
            // TODO: Start pan operation
        } else {
            log!("Mouse up at: {}, {}", x, y);
            // TODO: End pan operation
        }
    }

    /// Check if chart needs re-rendering
    #[wasm_bindgen]
    pub fn needs_render(&self) -> bool {
        // TODO: Implement dirty flag logic
        true // For now, always render
    }
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Export version info
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
