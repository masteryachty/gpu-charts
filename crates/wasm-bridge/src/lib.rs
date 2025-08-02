//! WASM Bridge crate for GPU Charts
//! Central orchestration layer that bridges JavaScript and Rust/WebGPU worlds

use config_system::PresetManager;
use wasm_bindgen::prelude::*;

// Core modules
pub mod chart_engine;
pub mod controls;
pub mod immediate_update;
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

        let instance_id = self.instance_id;

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            instance
                .chart_engine
                .set_preset_and_symbol(Some(preset.to_string()), Some(symbol.to_string()));
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

                    // Trigger render loop state change to preprocess
                    InstanceManager::with_instance_mut(&instance_id, |instance| {
                        let _ = instance.chart_engine.start_render_loop();
                        instance.chart_engine.on_data_received();
                    });
                    
                    // Start a render loop to ensure GPU bounds are calculated
                    wasm_bindgen_futures::spawn_local(async move {
                        // Render a few times to ensure GPU bounds calculation completes
                        for i in 0..5 {
                            // Wait a bit between renders
                            if i > 0 {
                                let promise = js_sys::Promise::new(&mut |resolve, _| {
                                    web_sys::window()
                                        .unwrap()
                                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                                            &resolve,
                                            100,
                                        )
                                        .unwrap();
                                });
                                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                            }
                            
                            InstanceManager::with_instance_mut(&instance_id, |instance| {
                                let _ = instance.chart_engine.render();
                            });
                        }
                    });
                }
                Err(e) => {
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
                }
                Err(e) => {
                }
            }
        });

        // Return immediately - the render will happen asynchronously
        Ok(())
    }

    #[wasm_bindgen]
    pub fn needs_render(&self) -> bool {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            instance.chart_engine.needs_render()
        }).unwrap_or(false)
    }

    #[wasm_bindgen]
    pub fn resize(&self, width: u32, height: u32) -> Result<(), JsValue> {

        // InstanceManager::with_instance_mut(&self.instance_id, |instance| {
        //     instance.chart_engine.resized(width, height);
        // })
        // .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    /// Set the time range for the chart
    #[wasm_bindgen(js_name = setTimeRange)]
    pub fn set_time_range(&mut self, start_time: u32, end_time: u32) -> Result<(), JsValue> {
        
        // Update the DataStore with the new time range
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let data_store = instance.chart_engine.renderer.data_store_mut();
            data_store.start_x = start_time;
            data_store.end_x = end_time;
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;
        
        // If we have a preset and symbol, re-fetch data with the new time range
        let (preset_name, symbol) = InstanceManager::with_instance(&self.instance_id, |instance| {
            let data_store = instance.chart_engine.renderer.data_store();
            (
                data_store.preset.as_ref().map(|p| p.name.clone()),
                data_store.symbol.clone()
            )
        })
        .unwrap_or((None, None));
        
        if let (Some(preset), Some(symbol)) = (preset_name, symbol) {
            self.apply_preset_and_symbol(&preset, &symbol)?;
        }
        
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, _y: f64) -> Result<(), JsValue> {

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let window_event = WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(x, delta_y)),
                phase: TouchPhase::Moved,
            };


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

            if data_store.is_dirty() {
                // Force recalculation of Y bounds by clearing them
                data_store.gpu_min_y = None;
                data_store.gpu_max_y = None;

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
    pub fn update_unified_state(&self, store_state_json: &str) -> Result<String, JsValue> {

        // Parse the JSON state
        let store_state: serde_json::Value = serde_json::from_str(store_state_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse state JSON: {}", e)))?;

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Update unified state and get diff
            let diff = instance.chart_engine.update_from_react_state(&store_state);

            // Get required actions
            let actions = diff.get_required_actions();

            // Trigger appropriate render loop updates based on actions
            if actions.needs_data_fetch {
                instance.chart_engine.on_data_config_changed();
            } else if actions.needs_pipeline_rebuild {
                instance.chart_engine.on_data_config_changed();
            } else if actions.needs_render {
                instance.chart_engine.on_view_changed();
            }

            // Return state diff as JSON
            serde_json::to_string(&diff)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize diff: {}", e)))
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?
    }

    #[wasm_bindgen]
    pub fn get_unified_state(&self) -> Result<String, JsValue> {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            let state = instance.chart_engine.get_unified_state();
            serde_json::to_string(state)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize state: {}", e)))
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?
    }

    #[wasm_bindgen]
    pub fn get_state_generation(&self) -> Result<u64, JsValue> {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            instance.chart_engine.get_unified_state().generation
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))
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

// Re-export simplified API for easy access
// pub use simple_api::{
//     create_chart, ChartBatch, ChartConfig, ChartFactory, ChartRegistry, SimpleChart,
// };
