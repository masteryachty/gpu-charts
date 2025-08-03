//! WASM Bridge crate for GPU Charts
//! Central orchestration layer that bridges JavaScript and Rust/WebGPU worlds

use config_system::PresetManager;
use data_manager::DataStore;
use wasm_bindgen::prelude::*;

// Core modules
pub mod chart_engine;
pub mod controls;
pub mod instance_manager;
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
        // Initialize logger on first construction
        static LOGGER_INIT: std::sync::Once = std::sync::Once::new();
        LOGGER_INIT.call_once(|| {
            if console_log::init_with_level(log::Level::Debug).is_err() {
                // Logger might already be initialized, that's ok
            }
        });

        log::debug!("[BRIDGE] new");

        Chart {
            instance_id: Uuid::new_v4(),
        }
    }

    #[wasm_bindgen]
    pub fn get_all_preset_names(&self) -> Result<js_sys::Array, JsValue> {
        log::debug!("[BRIDGE] get_all_preset_names");

        let preset_manager = PresetManager::new();
        let names = js_sys::Array::new();

        for preset in preset_manager.get_all_presets() {
            names.push(&JsValue::from_str(&preset.name));
        }

        Ok(names)
    }

    #[wasm_bindgen]
    pub fn get_metrics_for_preset(&self) -> Result<js_sys::Array, JsValue> {
        log::debug!("[BRIDGE] get_metrics_for_preset");
        let metrics = js_sys::Array::new();

        // Get the preset from the instance, return empty array if not available
        if let Some(Some(chart_preset)) =
            InstanceManager::with_instance(&self.instance_id, |instance| {
                instance.chart_engine.data_store().preset.clone()
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
        log::debug!("[BRIDGE] toggle_metric_visibility");

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Get mutable access to the data store
            let data_store = instance.chart_engine.data_store_mut();

            // Check if preset exists
            if let Some(preset) = &mut data_store.preset {
                // Find and toggle the metric's visibility
                if let Some(metric) = preset
                    .chart_types
                    .iter_mut()
                    .find(|m| m.label == metric_label)
                {
                    metric.visible = !metric.visible;
                }
            }

            // Trigger metric visibility changed - rebuild renderer
            instance.chart_engine.on_metric_visibility_changed();

            // Trigger a render to update the chart
            let _ = instance.chart_engine.render();
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn apply_preset_and_symbol(&mut self, preset: &str, symbol: &str) -> js_sys::Promise {
        log::debug!("[BRIDGE] apply_preset_and_symbol");
        let instance_id = self.instance_id;
        let preset_str = preset.to_string();
        let symbol_str = symbol.to_string();

        // Create a promise that resolves when the preset is fully applied
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let resolve = resolve.clone();
            let reject = reject.clone();
            let preset_str = preset_str.clone();
            let symbol_str = symbol_str.clone();

            // Spawn async task to handle everything
            wasm_bindgen_futures::spawn_local(async move {
                // First set the preset and symbol
                match InstanceManager::with_instance_mut(&instance_id, |instance| {
                    instance
                        .chart_engine
                        .set_preset_and_symbol(Some(preset_str), Some(symbol_str));
                }) {
                    Some(_) => {}
                    None => {
                        reject
                            .call1(
                                &JsValue::undefined(),
                                &JsValue::from_str("Chart instance not found"),
                            )
                            .unwrap();
                        return;
                    }
                }

                // Create a dedicated async function to handle the data fetching
                async fn fetch_and_process_data(
                    instance_id: Uuid,
                ) -> Result<(), shared_types::GpuChartsError> {
                    // Use InstanceManager to get a temporary mutable reference for the async operation
                    let instance_opt = InstanceManager::take_instance(&instance_id);

                    if let Some(mut instance) = instance_opt {
                        // Get a raw pointer to avoid borrow checker issues
                        let data_store_ptr =
                            instance.chart_engine.data_store_mut() as *mut DataStore;
                        let data_store = unsafe { &mut *data_store_ptr };

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
                        // Trigger render loop state change to preprocess
                        InstanceManager::with_instance_mut(&instance_id, |_instance| {
                            // let _ = instance.chart_engine.start_render_loop();
                            // instance.chart_engine.on_data_received();
                        });

                        // Start a render loop to ensure GPU bounds are calculated
                        wasm_bindgen_futures::spawn_local(async move {
                            InstanceManager::with_instance_mut(&instance_id, |instance| {
                                log::debug!("[bridge] 1");
                                let _ = instance.chart_engine.render();
                            });
                        });
                        // Resolve the promise after renders are complete
                        resolve
                            .call1(&JsValue::undefined(), &JsValue::from_bool(true))
                            .unwrap();
                    }
                    Err(e) => {
                        reject
                            .call1(
                                &JsValue::undefined(),
                                &JsValue::from_str(&format!("Failed to fetch data: {e:?}")),
                            )
                            .unwrap();
                    }
                }
            });
        });

        promise
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
        log::debug!("[BRIDGE] init");

        // Store instance using the instance manager
        self.instance_id =
            InstanceManager::create_instance(canvas_id, width, height, start_x, end_x)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn render(&self) -> Result<(), JsValue> {
        log::debug!("[BRIDGE] render");

        // For web rendering, we typically want to render asynchronously
        // without blocking. We'll spawn a local task to handle the render.
        let instance_id = self.instance_id;

        // Spawn the render task
        wasm_bindgen_futures::spawn_local(async move {
            // We need to perform the render in chunks to avoid holding the lock too long
            // First, check if the instance exists
            let exists = InstanceManager::instance_exists(&instance_id);
            if !exists {
                return;
            }

            // Now perform the actual render by temporarily taking ownership
            // This is a workaround for the async/borrow checker issues
            let render_result = {
                // Take the instance temporarily
                let instance_opt = InstanceManager::take_instance(&instance_id);
                match instance_opt {
                    Some(mut instance) => {
                        log::debug!("[bridge] 2");

                        // Perform the render
                        let result = instance.chart_engine.render();

                        // Put the instance back
                        InstanceManager::put_instance(instance_id, instance);

                        result
                    }
                    None => {
                        return;
                    }
                }
            };

            match render_result {
                Ok(()) => {
                    log::debug!("[BRIDGE] Render completed successfully");
                }
                Err(e) => {
                    log::error!("[BRIDGE] Render failed: {e:?}");
                }
            }
        });

        // Return immediately - the render will happen asynchronously
        Ok(())
    }

    #[wasm_bindgen]
    pub fn needs_render(&self) -> bool {
        log::debug!("[BRIDGE] needs_render");

        InstanceManager::with_instance(&self.instance_id, |instance| {
            instance.chart_engine.needs_render()
        })
        .unwrap_or(false)
    }

    #[wasm_bindgen]
    pub fn resize(&self, _width: u32, _height: u32) -> Result<(), JsValue> {
        log::debug!("[BRIDGE] resize");

        // InstanceManager::with_instance_mut(&self.instance_id, |instance| {
        //     instance.chart_engine.resized(width, height);
        // })
        // .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
        log::debug!("[BRIDGE] handle_mouse_wheel");

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // First update the mouse position
            let cursor_event = WindowEvent::CursorMoved {
                position: PhysicalPosition::new(x, y),
            };
            instance.chart_engine.handle_cursor_event(cursor_event);

            // Then send the wheel event
            let window_event = WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, delta_y)),
                phase: TouchPhase::Moved,
            };

            instance.chart_engine.handle_cursor_event(window_event);

            // After zoom, ensure bounds are recalculated
            let data_store = instance.chart_engine.data_store_mut();

            if data_store.is_dirty() {
                // Force recalculation of Y bounds by clearing them
                data_store.gpu_min_y = None;
                data_store.gpu_max_y = None;
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
            instance.chart_engine.handle_cursor_event(window_event);

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
            instance.chart_engine.handle_cursor_event(window_event);

            // After drag zoom (on release), ensure bounds are recalculated
            if !pressed {
                let chart_engine = &mut instance.chart_engine;
                let data_store = chart_engine.data_store_mut();
                if data_store.is_dirty() {
                    // Force recalculation of Y bounds by clearing them
                    data_store.gpu_min_y = None;
                    data_store.gpu_max_y = None;

                    // Trigger view changed for drag zoom
                    // instance.chart_engine.on_view_changed();
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
