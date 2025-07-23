//! WASM Bridge crate for GPU Charts
//! Central orchestration layer that bridges JavaScript and Rust/WebGPU worlds

// Allow clippy warnings for this crate
#![allow(clippy::all)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Core modules
pub mod line_graph;
pub mod controls;
pub mod wrappers;

// React integration modules
#[cfg(target_arch = "wasm32")]
pub mod lib_react;

#[cfg(target_arch = "wasm32")]
pub mod react_bridge;

// Re-export the Chart class for React integration
#[cfg(target_arch = "wasm32")]
pub use lib_react::Chart;

use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;
use shared_types::{ChartType, GpuChartsConfig};
use data_manager::DataManager;
use renderer::Renderer;
use config_system::ConfigManager;

/// Main ChartSystem implementation as per architect.md
pub struct ChartSystem {
    // WebGPU resources
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    // Core components
    data_manager: RefCell<DataManager>,
    renderer: Option<Renderer>,

    // Configuration
    config: GpuChartsConfig,
    config_manager: ConfigManager,
}

// Implementation methods
impl ChartSystem {
    pub async fn new(canvas_id: String, base_url: String) -> Result<ChartSystem, JsValue> {
        // Initialize logging
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Debug);

        // Get canvas element
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id(&canvas_id)
            .ok_or_else(|| JsValue::from_str(&format!("Canvas with id {} not found", canvas_id)))?;
        
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into()
            .map_err(|_| JsValue::from_str("Element is not a canvas"))?;

        // Initialize WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {:?}", e)))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| JsValue::from_str("Failed to find suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("GPU Charts Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create device: {:?}", e)))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Create components
        let config_manager = ConfigManager::new();
        let config = config_manager.get_config().clone();
        
        let data_manager = DataManager::new(device.clone(), queue.clone(), base_url);

        Ok(ChartSystem {
            device,
            queue,
            data_manager: RefCell::new(data_manager),
            renderer: None,
            config,
            config_manager,
        })
    }

    pub async fn update_chart(
        &mut self,
        chart_type: &str,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<(), JsValue> {
        // Parse chart type
        let chart_type = match chart_type {
            "line" => ChartType::Line,
            "candlestick" => ChartType::Candlestick,
            "bar" => ChartType::Bar,
            "area" => ChartType::Area,
            _ => return Err(JsValue::from_str(&format!("Unknown chart type: {}", chart_type))),
        };

        // Fetch data
        let columns = match chart_type {
            ChartType::Line => vec!["time", "best_bid", "best_ask"],
            ChartType::Candlestick => vec!["time", "open", "high", "low", "close"],
            ChartType::Bar => vec!["time", "volume"],
            ChartType::Area => vec!["time", "price"],
        };

        let _data_handle = self.data_manager
            .borrow_mut()
            .fetch_data(symbol, start_time, end_time, &columns)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data: {:?}", e)))?;

        // TODO: Update renderer with new data

        Ok(())
    }

    pub fn render(&mut self) {
        // TODO: Implement rendering
        log::debug!("Render called");
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(renderer) = &mut self.renderer {
            // TODO: Implement resize in renderer
            // renderer.resize(width, height);
            log::debug!("Resize called: {}x{}", width, height);
        }
    }

    pub fn handle_mouse_wheel(&mut self, delta_y: f32, x: f32, y: f32) {
        log::debug!("Mouse wheel: delta_y={}, x={}, y={}", delta_y, x, y);
        // TODO: Implement zoom handling
    }

    pub fn handle_mouse_click(&mut self, x: f32, y: f32, pressed: bool) {
        log::debug!("Mouse click: x={}, y={}, pressed={}", x, y, pressed);
        // TODO: Implement click handling
    }

    pub fn get_stats(&self) -> String {
        // TODO: Return actual stats
        format!("{{\"fps\": 60, \"dataPoints\": 0}}")
    }
}

// Also export manual_run for backward compatibility if needed
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn manual_run() {
    // This could be used for standalone mode if needed in the future
    // For now, just initialize logging
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            let _ = console_log::init_with_level(log::Level::Debug);
        }
    }
}