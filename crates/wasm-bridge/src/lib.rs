//! WASM Bridge crate for GPU Charts
//! Central orchestration layer that bridges JavaScript and Rust/WebGPU worlds

use config_system::PresetManager;
use wasm_bindgen::prelude::*;

// Core modules
pub mod chart_engine;
pub mod controls;
pub mod instance_manager;
pub mod render_loop;
pub mod wrappers;

use instance_manager::InstanceManager;
use shared_types::events::{
    ElementState, MouseButton, MouseScrollDelta, PhysicalPosition, TouchPhase, WindowEvent,
};

use uuid::Uuid;

extern crate nalgebra_glm as glm;

#[wasm_bindgen]
pub struct Chart {
    instance_id: Uuid,
}

impl Default for Chart {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Chart {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Chart {
        Chart {
            instance_id: Uuid::new_v4(),
        }
    }

    #[wasm_bindgen]
    pub fn get_all_preset_names(&self) -> Result<js_sys::Array, JsValue> {
        let preset_manager = PresetManager::new();
        let names = js_sys::Array::new();

        for preset in preset_manager.get_all_presets() {
            names.push(&JsValue::from_str(&preset.name));
        }

        Ok(names)
    }

    #[wasm_bindgen]
    pub fn get_metrics_for_preset(&self) -> Result<js_sys::Array, JsValue> {
        let metrics = js_sys::Array::new();

        // Get the preset from the instance, return empty array if not available
        if let Some(Some(chart_preset)) =
            InstanceManager::with_instance(&self.instance_id, |instance| {
                instance.chart_engine.renderer.data_store().preset.clone()
            })
        {
            // Add each metric's label and visibility to the array
            for metric in &chart_preset.chart_types {
                metrics.push(&JsValue::from_str(&metric.label));
                metrics.push(&JsValue::from_bool(metric.visible));
            }
        }

        Ok(metrics)
    }

    #[wasm_bindgen]
    pub fn toggle_metric_visibility(&self, metric_label: &str) -> Result<(), JsValue> {
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Get mutable access to the data store
            let data_store = instance.chart_engine.renderer.data_store_mut();

            // Check if preset exists
            if let Some(preset) = &mut data_store.preset {
                // Find and toggle the metric's visibility
                if let Some(metric) = preset
                    .chart_types
                    .iter_mut()
                    .find(|m| m.label == metric_label)
                {
                    metric.visible = !metric.visible;
                    log::info!(
                        "Toggled metric '{}' visibility to: {}",
                        metric_label,
                        metric.visible
                    );
                }
            }

            // Trigger metric visibility changed - render only
            instance.chart_engine.on_metric_visibility_changed();
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn apply_preset_and_symbol(&mut self, preset: &str, symbol: &str) -> Result<(), JsValue> {
        log::info!("Active Preset set to : {preset}");

        let instance_id = self.instance_id;

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            instance
                .chart_engine
                .set_preset_and_symbol(Some(preset.to_string()), Some(symbol.to_string()));
            log::info!("Active Preset set to : {preset}");
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        // Spawn async task to fetch data and update render loop state
        wasm_bindgen_futures::spawn_local(async move {
            // Create a dedicated async function to handle the data fetching
            async fn fetch_and_process_data(
                instance_id: Uuid,
            ) -> Result<(), shared_types::GpuChartsError> {
                // Use InstanceManager to get a temporary mutable reference for the async operation
                let instance_opt = InstanceManager::take_instance(&instance_id);

                if let Some(mut instance) = instance_opt {
                    let data_store = instance.chart_engine.renderer.data_store_mut();
                    let result = instance
                        .chart_engine
                        .data_manager
                        .fetch_data_for_preset(data_store)
                        .await;

                    // Put the instance back
                    InstanceManager::put_instance(instance_id, instance);

                    result
                } else {
                    Err(shared_types::GpuChartsError::DataNotFound {
                        resource: "Chart instance".to_string(),
                    })
                }
            }

            // Execute the fetch
            match fetch_and_process_data(instance_id).await {
                Ok(_) => {
                    log::info!("Data fetched successfully for preset");

                    // Trigger render loop state change to preprocess
                    InstanceManager::with_instance_mut(&instance_id, |instance| {
                        let _ = instance.chart_engine.start_render_loop();
                        instance.chart_engine.on_data_received();
                    });
                }
                Err(e) => {
                    log::error!("Failed to fetch data for preset: {e:?}");
                }
            }
        });

        Ok(())
    }

    #[wasm_bindgen]
    pub async fn init(
        &mut self,
        canvas_id: &str,
        width: u32,
        height: u32,
        start_x: u32,
        end_x: u32,
    ) -> Result<(), JsValue> {
        // Only set panic hook if not already set
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            // Try to initialize logger, but don't panic if it fails (already initialized)
            let _ = console_log::init_with_level(log::Level::Debug);
        });

        // Store instance using the instance manager
        self.instance_id =
            InstanceManager::create_instance(canvas_id, width, height, start_x, end_x)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
        Ok(())
    }

    // #[wasm_bindgen]
    // pub fn start_render_loop(&mut self) -> Result<(), JsValue> {
    //     log::info!("Starting render loop");
    //     InstanceManager::with_instance_mut(&self.instance_id, |instance| {
    //         instance.chart_engine.start_render_loop()
    //             .map_err(|e| JsValue::from_str(&format!("Failed to start render loop: {:?}", e)))
    //     })
    //     .ok_or_else(|| JsValue::from_str("Chart instance not found"))?
    // }

    // #[wasm_bindgen]
    // pub fn stop_render_loop(&mut self) -> Result<(), JsValue> {
    //     log::info!("Stopping render loop");
    //     InstanceManager::with_instance_mut(&self.instance_id, |instance| {
    //         instance.chart_engine.stop_render_loop()
    //             .map_err(|e| JsValue::from_str(&format!("Failed to stop render loop: {:?}", e)))
    //     })
    //     .ok_or_else(|| JsValue::from_str("Chart instance not found"))?
    // }

    // #[wasm_bindgen]
    // pub fn get_render_state(&self) -> String {
    //     InstanceManager::with_instance(&self.instance_id, |instance| {
    //         format!("{:?}", instance.chart_engine.get_render_state())
    //     })
    //     .unwrap_or_else(|| "Unknown".to_string())
    // }

    #[wasm_bindgen]
    pub async fn render(&self) -> Result<(), JsValue> {
        // For web rendering, we typically want to render asynchronously
        // without blocking. We'll spawn a local task to handle the render.
        let instance_id = self.instance_id;

        // Spawn the render task
        wasm_bindgen_futures::spawn_local(async move {
            // We need to perform the render in chunks to avoid holding the lock too long
            // First, check if the instance exists
            let exists = InstanceManager::instance_exists(&instance_id);
            if !exists {
                log::error!("Chart instance not found for rendering");
                return;
            }

            // Now perform the actual render by temporarily taking ownership
            // This is a workaround for the async/borrow checker issues
            let render_result = {
                // Take the instance temporarily
                let instance_opt = InstanceManager::take_instance(&instance_id);
                match instance_opt {
                    Some(mut instance) => {
                        // Perform the render
                        let result = instance.chart_engine.render().await;

                        // Put the instance back
                        InstanceManager::put_instance(instance_id, instance);

                        result
                    }
                    None => {
                        log::error!("Failed to take instance for rendering");
                        return;
                    }
                }
            };

            match render_result {
                Ok(()) => {
                    log::trace!("Render completed successfully");
                }
                Err(e) => {
                    log::error!("Render failed: {e:?}");
                }
            }
        });

        // Return immediately - the render will happen asynchronously
        Ok(())
    }

    #[wasm_bindgen]
    pub fn resize(&self, width: u32, height: u32) -> Result<(), JsValue> {
        log::info!("Resizing chart to: {width}x{height}");

        // InstanceManager::with_instance_mut(&self.instance_id, |instance| {
        //     instance.chart_engine.resized(width, height);
        // })
        // .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, _y: f64) -> Result<(), JsValue> {
        log::info!("[WASM] handle_mouse_wheel called with delta_y={delta_y}, x={x}");

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let window_event = WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(x, delta_y)),
                phase: TouchPhase::Moved,
            };

            log::info!("[WASM] Created MouseWheel event, passing to canvas_controller");

            instance
                .chart_engine
                .canvas_controller
                .handle_cursor_event(window_event, &mut instance.chart_engine.renderer);

            // After zoom, ensure bounds are recalculated
            let renderer = &mut instance.chart_engine.renderer;
            let data_store = renderer.data_store_mut();

            let is_dirty = data_store.is_dirty();
            let gpu_min_y = data_store.gpu_min_y;
            let gpu_max_y = data_store.gpu_max_y;
            log::info!("[WASM] After handle_cursor_event - data_store is_dirty: {is_dirty}, min_y: {gpu_min_y:?}, max_y: {gpu_max_y:?}");

            if data_store.is_dirty() {
                // Force recalculation of Y bounds by clearing them
                data_store.gpu_min_y = None;
                data_store.gpu_max_y = None;
                log::info!("[WASM] Cleared Y bounds for recalculation");

                // Trigger view changed in render loop
                instance.chart_engine.on_view_changed();
            }
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_move(&self, x: f64, y: f64) -> Result<(), JsValue> {
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let window_event = WindowEvent::CursorMoved {
                position: PhysicalPosition::new(x, y),
            };
            instance
                .chart_engine
                .canvas_controller
                .handle_cursor_event(window_event, &mut instance.chart_engine.renderer);

            // Mouse movement during drag should trigger view change
            // instance.chart_engine.on_view_changed();
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_click(&self, _x: f64, _y: f64, pressed: bool) -> Result<(), JsValue> {
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let window_event = WindowEvent::MouseInput {
                state: if pressed {
                    ElementState::Pressed
                } else {
                    ElementState::Released
                },
                button: MouseButton::Left,
            };
            instance
                .chart_engine
                .canvas_controller
                .handle_cursor_event(window_event, &mut instance.chart_engine.renderer);

            // After drag zoom (on release), ensure bounds are recalculated
            if !pressed {
                let renderer = &mut instance.chart_engine.renderer;
                let data_store = renderer.data_store_mut();
                if data_store.is_dirty() {
                    // Force recalculation of Y bounds by clearing them
                    data_store.gpu_min_y = None;
                    data_store.gpu_max_y = None;

                    // Trigger view changed for drag zoom
                    instance.chart_engine.on_view_changed();
                }
            }
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}
