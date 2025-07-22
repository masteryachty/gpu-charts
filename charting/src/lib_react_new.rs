//! React bridge for the new modular renderer architecture

use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlCanvasElement;

use crate::renderer_bridge::{RendererBridge, create_default_config};
use gpu_charts_shared::{ChartType, TimeRange};
use gpu_charts_renderer::Viewport;

extern crate nalgebra_glm as glm;

// Global state for the chart instance
static mut CHART_INSTANCE: Option<ChartInstance> = None;

struct ChartInstance {
    renderer_bridge: RendererBridge,
    current_symbol: String,
    current_time_range: TimeRange,
    width: u32,
    height: u32,
}

#[wasm_bindgen]
#[derive(Default)]
pub struct Chart {
    #[allow(dead_code)]
    instance_id: u32,
}

#[wasm_bindgen]
impl Chart {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Chart {
        Chart::default()
    }

    #[wasm_bindgen]
    pub async fn init(&self, canvas_id: &str, width: u32, height: u32) -> Result<(), JsValue> {
        // Initialize panic hook and logging
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
            }
        }

        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Element is not a canvas")?;

        // Set canvas size
        canvas.set_width(width);
        canvas.set_height(height);

        // Create renderer bridge
        let mut renderer_bridge = RendererBridge::new(canvas, width, height)
            .await
            .map_err(|e| format!("Failed to create renderer bridge: {:?}", e))?;
            
        // Initialize with default configuration
        let config = create_default_config(ChartType::Line);
        renderer_bridge.init_with_config(config)
            .await
            .map_err(|e| format!("Failed to initialize config: {:?}", e))?;

        // Set default time range (last hour)
        let now = chrono::Utc::now().timestamp() as u64;
        let time_range = TimeRange::new(now - 3600, now);
        
        // Load initial data
        let symbol = "BTC-USD".to_string();
        renderer_bridge.load_data(symbol.clone(), time_range)
            .await
            .map_err(|e| format!("Failed to load data: {:?}", e))?;

        // Store globally
        let instance = ChartInstance {
            renderer_bridge,
            current_symbol: symbol,
            current_time_range: time_range,
            width,
            height,
        };

        unsafe {
            CHART_INSTANCE = Some(instance);
        }

        // Initial render
        self.render().await?;

        log::info!("Chart initialized successfully with new renderer");
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn render(&self) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                instance.renderer_bridge.render()?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn resize(&self, width: u32, height: u32) -> Result<(), JsValue> {
        log::info!("Resizing chart to: {}x{}", width, height);

        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                instance.renderer_bridge.resize(width, height);
                instance.width = width;
                instance.height = height;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn handle_mouse_wheel(&self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                instance.renderer_bridge.handle_mouse_wheel(delta_y, x, y).await?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn update_viewport(&self, x: f32, y: f32, zoom: f32) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let viewport = Viewport {
                    x,
                    y,
                    width: instance.width as f32,
                    height: instance.height as f32,
                    zoom_level: zoom,
                    time_range: instance.current_time_range,
                };
                instance.renderer_bridge.update_viewport(viewport);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn load_data(&self, symbol: String, start: u64, end: u64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let time_range = TimeRange::new(start, end);
                instance.renderer_bridge.load_data(symbol.clone(), time_range).await?;
                instance.current_symbol = symbol;
                instance.current_time_range = time_range;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_chart_type(&self, chart_type: String) -> Result<(), JsValue> {
        let chart_type = match chart_type.as_str() {
            "line" => ChartType::Line,
            "candlestick" => ChartType::Candlestick,
            "area" => ChartType::Area,
            "bar" => ChartType::Bar,
            _ => return Err(JsValue::from_str("Invalid chart type")),
        };
        
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let mut config = create_default_config(chart_type);
                // Preserve data handles if any
                spawn_local(async move {
                    if let Err(e) = instance.renderer_bridge.init_with_config(config).await {
                        log::error!("Failed to update chart type: {:?}", e);
                    }
                });
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn get_performance_metrics(&self) -> String {
        unsafe {
            if let Some(instance) = &CHART_INSTANCE {
                instance.renderer_bridge.get_performance_metrics()
            } else {
                "{}".to_string()
            }
        }
    }

    #[wasm_bindgen]
    pub fn get_stats(&self) -> String {
        unsafe {
            if let Some(instance) = &CHART_INSTANCE {
                instance.renderer_bridge.get_stats()
            } else {
                "{}".to_string()
            }
        }
    }

    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        unsafe {
            CHART_INSTANCE.is_some()
        }
    }
}

/// Export create_chart for convenience
#[wasm_bindgen]
pub fn create_chart() -> Chart {
    Chart::new()
}