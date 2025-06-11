use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, console};

mod calcables;
mod controls;
mod drawables;
mod renderer;
mod wrappers;
mod line_graph;
pub mod store_state;

use crate::line_graph::LineGraph;
use crate::controls::canvas_controller::CanvasController;
use crate::store_state::{StoreState, StoreValidationResult};
use winit::{
    dpi::PhysicalSize,
    event::{WindowEvent, MouseScrollDelta, ElementState},
    window::{Window, WindowId},
    platform::web::WindowExtWebSys,
};

extern crate nalgebra_glm as glm;

// Global state for the chart instance
static mut CHART_INSTANCE: Option<ChartInstance> = None;

struct ChartInstance {
    line_graph: Rc<RefCell<LineGraph>>,
    canvas_controller: CanvasController,
    window: Rc<Window>,
    current_store_state: Option<StoreState>,
}

#[wasm_bindgen]
pub struct Chart {
    instance_id: u32,
}

#[wasm_bindgen]
impl Chart {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Chart {
        Chart { instance_id: 0 }
    }

    #[wasm_bindgen]
    pub async fn init(&self, canvas_id: &str, width: u32, height: u32) -> Result<(), JsValue> {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
            }
        }

        log::info!("Initializing chart with canvas: {}, size: {}x{}", canvas_id, width, height);

        // Get the canvas element
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

        // Create a mock event loop and window for winit compatibility
        let event_loop = winit::event_loop::EventLoop::new()
            .map_err(|e| format!("Failed to create event loop: {:?}", e))?;
        
        let window_attrs = Window::default_attributes()
            .with_inner_size(PhysicalSize::new(width, height));

        // For WASM, we need to associate with the canvas
        #[cfg(target_arch = "wasm32")]
        let window_attrs = {
            use winit::platform::web::WindowAttributesExtWebSys;
            window_attrs.with_canvas(Some(canvas))
        };

        let window = Rc::new(event_loop.create_window(window_attrs)
            .map_err(|e| format!("Failed to create window: {:?}", e))?);

        // Initialize the line graph
        let line_graph = LineGraph::new(width, height, window.clone())
            .await
            .map_err(|e| format!("Failed to create LineGraph: {:?}", e))?;

        let line_graph = Rc::new(RefCell::new(line_graph));

        // Create canvas controller
        let data_store = line_graph.borrow().data_store.clone();
        let engine = line_graph.borrow().engine.clone();
        let canvas_controller = CanvasController::new(window.clone(), data_store, engine);

        // Store globally (in a real app, you'd want better state management)
        let instance = ChartInstance {
            line_graph,
            canvas_controller,
            window,
            current_store_state: None,
        };

        unsafe {
            CHART_INSTANCE = Some(instance);
        }

        // Initial render
        self.render().await?;

        log::info!("Chart initialized successfully");
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn render(&self) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &CHART_INSTANCE {
                instance.line_graph.borrow().render()
                    .await
                    .map_err(|e| format!("Render failed: {:?}", e))?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn resize(&self, width: u32, height: u32) -> Result<(), JsValue> {
        log::info!("Resizing chart to: {}x{}", width, height);
        
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                instance.line_graph.borrow_mut().resized(width, height);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let window_event = WindowEvent::MouseWheel {
                    device_id: unsafe { std::mem::transmute(0u32) },
                    delta: MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, delta_y)),
                    phase: winit::event::TouchPhase::Moved,
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_move(&self, x: f64, y: f64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let window_event = WindowEvent::CursorMoved {
                    device_id: unsafe { std::mem::transmute(0u32) },
                    position: winit::dpi::PhysicalPosition::new(x, y),
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_click(&self, x: f64, y: f64, pressed: bool) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let window_event = WindowEvent::MouseInput {
                    device_id: unsafe { std::mem::transmute(0u32) },
                    state: if pressed { ElementState::Pressed } else { ElementState::Released },
                    button: winit::event::MouseButton::Left,
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_data_range(&self, start: u32, end: u32) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &CHART_INSTANCE {
                instance.line_graph.borrow().data_store.borrow_mut().set_x_range(start, end);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn request_redraw(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let closure = Closure::once_into_js(move || {
            wasm_bindgen_futures::spawn_local(async move {
                unsafe {
                    if let Some(instance) = &CHART_INSTANCE {
                        let _ = instance.line_graph.borrow().render().await;
                    }
                }
            });
        });
        
        window.request_animation_frame(closure.as_ref().unchecked_ref())?;
        closure.forget();
        Ok(())
    }

    /// Core bridge method: Update chart state from React store
    /// This is the main integration point between React and Rust
    #[wasm_bindgen]
    pub fn update_chart_state(&self, store_state_json: &str) -> Result<String, JsValue> {
        log::info!("update_chart_state called with: {}", store_state_json);
        
        // Step 1: Deserialize and validate the store state
        let store_state = match self.deserialize_and_validate_store_state(store_state_json) {
            Ok(state) => state,
            Err(validation_result) => {
                // Return validation errors as JSON
                let error_response = serde_json::json!({
                    "success": false,
                    "errors": validation_result.errors,
                    "warnings": validation_result.warnings
                });
                return Ok(error_response.to_string());
            }
        };

        // Step 2: Check if state has changed (to avoid unnecessary updates)
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                if let Some(ref current_state) = instance.current_store_state {
                    if self.states_are_equivalent(current_state, &store_state) {
                        log::info!("Store state unchanged, skipping update");
                        let response = serde_json::json!({
                            "success": true,
                            "message": "No changes detected",
                            "updated": false
                        });
                        return Ok(response.to_string());
                    }
                }

                // Step 3: Apply the state changes
                match self.apply_store_state_changes(&store_state, instance) {
                    Ok(changes_applied) => {
                        // Step 4: Update stored state
                        instance.current_store_state = Some(store_state);
                        
                        // Step 5: Return success response
                        let response = serde_json::json!({
                            "success": true,
                            "message": "Chart state updated successfully",
                            "updated": true,
                            "changes": changes_applied
                        });
                        Ok(response.to_string())
                    }
                    Err(e) => {
                        let error_response = serde_json::json!({
                            "success": false,
                            "errors": [format!("Failed to apply state changes: {}", e)],
                            "warnings": []
                        });
                        Ok(error_response.to_string())
                    }
                }
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
    }

    /// Check if the chart is initialized and has an active instance
    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        unsafe { CHART_INSTANCE.is_some() }
    }

    /// Get current store state as JSON (for debugging/sync purposes)
    #[wasm_bindgen]
    pub fn get_current_store_state(&self) -> Result<String, JsValue> {
        unsafe {
            if let Some(instance) = &CHART_INSTANCE {
                if let Some(ref state) = instance.current_store_state {
                    match serde_json::to_string(state) {
                        Ok(json) => Ok(json),
                        Err(e) => Err(JsValue::from_str(&format!("Serialization failed: {}", e)))
                    }
                } else {
                    Ok("null".to_string())
                }
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
    }

    /// Force a state update even if no changes detected (for debugging)
    #[wasm_bindgen]
    pub fn force_update_chart_state(&self, store_state_json: &str) -> Result<String, JsValue> {
        log::info!("force_update_chart_state called");
        
        let store_state = match self.deserialize_and_validate_store_state(store_state_json) {
            Ok(state) => state,
            Err(validation_result) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "errors": validation_result.errors,
                    "warnings": validation_result.warnings
                });
                return Ok(error_response.to_string());
            }
        };

        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                match self.apply_store_state_changes(&store_state, instance) {
                    Ok(changes_applied) => {
                        instance.current_store_state = Some(store_state);
                        let response = serde_json::json!({
                            "success": true,
                            "message": "Chart state force-updated successfully",
                            "updated": true,
                            "changes": changes_applied
                        });
                        Ok(response.to_string())
                    }
                    Err(e) => {
                        let error_response = serde_json::json!({
                            "success": false,
                            "errors": [format!("Failed to apply state changes: {}", e)],
                            "warnings": []
                        });
                        Ok(error_response.to_string())
                    }
                }
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
    }
}

// Private implementation methods for Chart
impl Chart {
    /// Deserialize and validate store state from JSON
    fn deserialize_and_validate_store_state(&self, json: &str) -> Result<StoreState, StoreValidationResult> {
        // First, try to deserialize the JSON
        let store_state: StoreState = match serde_json::from_str(json) {
            Ok(state) => state,
            Err(e) => {
                return Err(StoreValidationResult {
                    is_valid: false,
                    errors: vec![format!("JSON deserialization failed: {}", e)],
                    warnings: vec![],
                });
            }
        };

        // Then validate the deserialized state
        let validation_result = store_state.validate();
        if validation_result.is_valid {
            Ok(store_state)
        } else {
            Err(validation_result)
        }
    }

    /// Check if two store states are equivalent (for change detection)
    fn states_are_equivalent(&self, current: &StoreState, new: &StoreState) -> bool {
        // Compare relevant fields that would trigger chart updates
        current.current_symbol == new.current_symbol
            && current.chart_config.symbol == new.chart_config.symbol
            && current.chart_config.timeframe == new.chart_config.timeframe
            && current.chart_config.start_time == new.chart_config.start_time
            && current.chart_config.end_time == new.chart_config.end_time
            && current.chart_config.indicators == new.chart_config.indicators
            && current.is_connected == new.is_connected
    }

    /// Apply store state changes to the chart
    fn apply_store_state_changes(&self, store_state: &StoreState, instance: &mut ChartInstance) -> Result<Vec<String>, String> {
        let mut changes_applied = Vec::new();

        // Check if we need to update data (symbol or time range changed)
        let needs_data_update = if let Some(ref current_state) = instance.current_store_state {
            store_state.chart_config.symbol != current_state.chart_config.symbol
                || store_state.chart_config.start_time != current_state.chart_config.start_time
                || store_state.chart_config.end_time != current_state.chart_config.end_time
        } else {
            true // First time, always need data
        };

        if needs_data_update {
            // Update the data range in the data store
            let data_store = instance.line_graph.borrow().data_store.clone();
            data_store.borrow_mut().set_x_range(
                store_state.chart_config.start_time as u32,
                store_state.chart_config.end_time as u32,
            );
            
            // Note: In a full implementation, we would trigger data fetching here
            // For now, we just update the range
            changes_applied.push(format!(
                "Updated time range: {} to {}",
                store_state.chart_config.start_time,
                store_state.chart_config.end_time
            ));

            if let Some(ref current_state) = instance.current_store_state {
                if store_state.chart_config.symbol != current_state.chart_config.symbol {
                    changes_applied.push(format!(
                        "Changed symbol: {} -> {}",
                        current_state.chart_config.symbol,
                        store_state.chart_config.symbol
                    ));
                }
            } else {
                changes_applied.push(format!(
                    "Set symbol: {}",
                    store_state.chart_config.symbol
                ));
            }
        }

        // Check for timeframe changes
        if let Some(ref current_state) = instance.current_store_state {
            if store_state.chart_config.timeframe != current_state.chart_config.timeframe {
                changes_applied.push(format!(
                    "Changed timeframe: {} -> {}",
                    current_state.chart_config.timeframe,
                    store_state.chart_config.timeframe
                ));
            }

            // Check for indicator changes
            if store_state.chart_config.indicators != current_state.chart_config.indicators {
                changes_applied.push(format!(
                    "Updated indicators: {:?} -> {:?}",
                    current_state.chart_config.indicators,
                    store_state.chart_config.indicators
                ));
            }

            // Check for connection status changes
            if store_state.is_connected != current_state.is_connected {
                changes_applied.push(format!(
                    "Connection status: {} -> {}",
                    current_state.is_connected,
                    store_state.is_connected
                ));
            }
        } else {
            // First time setup
            changes_applied.push(format!("Set timeframe: {}", store_state.chart_config.timeframe));
            changes_applied.push(format!("Set indicators: {:?}", store_state.chart_config.indicators));
            changes_applied.push(format!("Set connection status: {}", store_state.is_connected));
        }

        // If any changes were applied, request a redraw
        if !changes_applied.is_empty() {
            log::info!("Requesting redraw due to state changes");
            // Spawn async render task
            let line_graph = instance.line_graph.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(graph) = line_graph.try_borrow() {
                    let _ = graph.render().await;
                }
            });
        }

        Ok(changes_applied)
    }
}

// Remove the auto-start function
// #[wasm_bindgen(start)]
// pub fn run() { ... }

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}