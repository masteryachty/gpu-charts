//! React bridge using the modular renderer architecture (simplified version)

use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::renderer_bridge_simple::{ChartConfig, RendererBridge};

extern crate nalgebra_glm as glm;

// Global state for the chart instance
static mut CHART_INSTANCE: Option<ChartInstance> = None;

struct ChartInstance {
    renderer_bridge: RendererBridge,
    canvas_id: String,
}

#[wasm_bindgen]
#[derive(Default)]
pub struct ModularChart {
    #[allow(dead_code)]
    instance_id: u32,
}

#[wasm_bindgen]
impl ModularChart {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ModularChart {
        ModularChart::default()
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
        let renderer_bridge = RendererBridge::new(canvas, width, height)
            .await
            .map_err(|e| format!("Failed to create renderer bridge: {:?}", e))?;

        // Store globally
        let instance = ChartInstance {
            renderer_bridge,
            canvas_id: canvas_id.to_string(),
        };

        unsafe {
            CHART_INSTANCE = Some(instance);
        }

        // Initial render
        self.render().await?;

        log::info!("ModularChart initialized successfully");
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn render(&self) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                instance.renderer_bridge.render().await?;
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
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn handle_mouse_wheel(&self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                instance
                    .renderer_bridge
                    .handle_mouse_wheel(delta_y, x, y)
                    .await?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_chart_type(&self, chart_type: String) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let mut config = ChartConfig::default();
                config.chart_type = chart_type;
                instance.renderer_bridge.update_config(config)?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_background_color(&self, r: f32, g: f32, b: f32, a: f32) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let mut config = ChartConfig::default();
                config.background_color = [r, g, b, a];
                instance.renderer_bridge.update_config(config)?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_grid_visibility(&self, show_grid: bool, show_axes: bool) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let mut config = ChartConfig::default();
                config.show_grid = show_grid;
                config.show_axes = show_axes;
                instance.renderer_bridge.update_config(config)?;
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
        unsafe { CHART_INSTANCE.is_some() }
    }
}

/// Export create_modular_chart for convenience
#[wasm_bindgen]
pub fn create_modular_chart() -> ModularChart {
    ModularChart::new()
}
