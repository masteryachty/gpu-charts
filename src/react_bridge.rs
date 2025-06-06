use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

// Simple bridge that allows React to start the existing chart system
static mut CHART_INITIALIZED: bool = false;

#[wasm_bindgen]
pub struct SimpleChart;

#[wasm_bindgen]
impl SimpleChart {
    #[wasm_bindgen(constructor)]
    pub fn new() -> SimpleChart {
        SimpleChart
    }

    /// Initialize the chart system - this will start the existing chart with winit
    #[wasm_bindgen]
    pub fn init(&self, canvas_id: &str) -> Result<(), JsValue> {
        unsafe {
            if CHART_INITIALIZED {
                log::warn!("Chart already initialized, skipping");
                return Ok(());
            }
            CHART_INITIALIZED = true;
        }

        // Initialize logging
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                let _ = console_log::init_with_level(log::Level::Debug);
            }
        }

        log::info!("SimpleChart.init called with canvas_id: {}", canvas_id);

        // Verify the canvas exists
        let window = web_sys::window().ok_or("No window object available")?;
        let document = window.document().ok_or("No document object available")?;

        log::info!("Looking for canvas element with id: {}", canvas_id);

        let canvas_element = document.get_element_by_id(canvas_id).ok_or_else(|| {
            log::error!("Canvas element with id '{}' not found in DOM", canvas_id);
            format!("Canvas with id '{}' not found in DOM", canvas_id)
        })?;

        log::info!("Found canvas element: {:?}", canvas_element.tag_name());

        let html_canvas: HtmlCanvasElement = canvas_element.dyn_into().map_err(|_| {
            log::error!("Element with id '{}' is not a canvas element", canvas_id);
            "Element is not a canvas"
        })?;

        log::info!(
            "Canvas verification successful. Canvas dimensions: {}x{}",
            html_canvas.width(),
            html_canvas.height()
        );

        // Start the existing chart system
        // This will use the existing winit event loop
        log::info!("Starting chart system via manual_run()");
        crate::manual_run();

        log::info!("Chart initialization completed successfully");
        Ok(())
    }

    /// Check if chart is initialized
    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        unsafe { CHART_INITIALIZED }
    }
}
