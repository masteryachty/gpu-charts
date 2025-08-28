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
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Get mutable access to the data store
            let data_store = instance.chart_engine.data_store_mut();

            // Check if preset exists
            if let Some(preset) = &mut data_store.preset {
                // Find and toggle the metric's visibility
                if let Some(chart_type) = preset
                    .chart_types
                    .iter_mut()
                    .find(|m| m.label == metric_label)
                {
                    chart_type.visible = !chart_type.visible;

                    // Also update the visibility in the data store's metrics
                    // Find the corresponding metric in data store by matching the column name
                    for (_data_type, column_name) in &chart_type.data_columns {
                        for data_group in &mut data_store.data_groups {
                            for metric in &mut data_group.metrics {
                                if metric.name == *column_name {
                                    metric.visible = chart_type.visible;
                                }
                            }
                        }
                    }
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

    // Note: Removed due to wasm-bindgen serialization issues with js_sys::Array parameter
    // This method was causing build failures and is not used by the React frontend
    
    #[wasm_bindgen]
    pub fn apply_preset_and_symbol(&mut self, preset: &str, symbol: &str) -> js_sys::Promise {
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
        // Store instance using the instance manager
        self.instance_id =
            InstanceManager::create_instance(canvas_id, width, height, start_x, end_x)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
        Ok(())
    }

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
                Ok(()) => {}
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
        InstanceManager::with_instance(&self.instance_id, |instance| {
            instance.chart_engine.needs_render()
        })
        .unwrap_or(false)
    }

    #[wasm_bindgen]
    pub fn update_time_range(&mut self, start_time: u32, end_time: u32) -> js_sys::Promise {
        let instance_id = self.instance_id;

        // Create a promise that resolves when data is fetched with new time range
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let resolve = resolve.clone();
            let reject = reject.clone();

            // Spawn async task to handle everything
            wasm_bindgen_futures::spawn_local(async move {
                // First check if preset and symbol are set
                let has_preset_and_symbol = InstanceManager::with_instance(&instance_id, |instance| {
                    instance.chart_engine.data_store().preset.is_some() && 
                    instance.chart_engine.data_store().symbol.is_some()
                }).unwrap_or(false);

                // Update the time range in the data store
                match InstanceManager::with_instance_mut(&instance_id, |instance| {
                    instance.chart_engine.data_store_mut().set_x_range(start_time, end_time);
                    instance.chart_engine.data_store_mut().mark_dirty();
                    
                    // Clear GPU bounds to force recalculation
                    instance.chart_engine.data_store_mut().gpu_min_y = None;
                    instance.chart_engine.data_store_mut().gpu_max_y = None;
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

                // Only fetch data if preset and symbol are set
                if !has_preset_and_symbol {
                    resolve
                        .call1(&JsValue::undefined(), &JsValue::from_bool(true))
                        .unwrap();
                    return;
                }

                // Now fetch data for the new time range
                async fn fetch_data_with_new_range(
                    instance_id: Uuid,
                ) -> Result<(), shared_types::GpuChartsError> {
                    let instance_opt = InstanceManager::take_instance(&instance_id);

                    if let Some(mut instance) = instance_opt {
                        let data_store_ptr =
                            instance.chart_engine.data_store_mut() as *mut DataStore;
                        let data_store = unsafe { &mut *data_store_ptr };

                        let result = instance
                            .chart_engine
                            .data_manager
                            .fetch_data_for_preset(data_store)
                            .await;

                        InstanceManager::put_instance(instance_id, instance);
                        result
                    } else {
                        Err(shared_types::GpuChartsError::DataNotFound {
                            resource: "Chart instance".to_string(),
                        })
                    }
                }

                // Execute the fetch
                match fetch_data_with_new_range(instance_id).await {
                    Ok(_) => {
                        // Trigger render to update the chart
                        wasm_bindgen_futures::spawn_local(async move {
                            InstanceManager::with_instance_mut(&instance_id, |instance| {
                                let _ = instance.chart_engine.render();
                            });
                        });
                        
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
    pub fn resize(&self, _width: u32, _height: u32) -> Result<(), JsValue> {
        // InstanceManager::with_instance_mut(&self.instance_id, |instance| {
        //     instance.chart_engine.resized(width, height);
        // })
        // .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    /// Get current start time from the chart
    #[wasm_bindgen]
    pub fn get_start_time(&self) -> Result<u32, JsValue> {
        let start_time = InstanceManager::with_instance(&self.instance_id, |instance| {
            instance.chart_engine.data_store().start_x
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;
        
        Ok(start_time)
    }

    /// Get current end time from the chart
    #[wasm_bindgen]
    pub fn get_end_time(&self) -> Result<u32, JsValue> {
        let end_time = InstanceManager::with_instance(&self.instance_id, |instance| {
            instance.chart_engine.data_store().end_x
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;
        
        Ok(end_time)
    }

    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, _y: f64) -> Result<(), JsValue> {
        // Process the mouse wheel event - separate from rendering to avoid RefCell conflicts
        let needs_render = InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let window_event = WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(x, delta_y)),
                phase: TouchPhase::Moved,
            };

            instance.chart_engine.handle_cursor_event(window_event);

            // After zoom, ensure bounds are recalculated
            let data_store = instance.chart_engine.data_store_mut();
            let is_dirty = data_store.is_dirty();

            if is_dirty {
                // Force recalculation of Y bounds by clearing them
                data_store.gpu_min_y = None;
                data_store.gpu_max_y = None;
            }

            is_dirty
        })
        .unwrap_or(false);

        // Render in a separate instance access to avoid borrow conflicts
        if needs_render {
            InstanceManager::with_instance_mut(&self.instance_id, |instance| {
                let _ = instance.chart_engine.render();
            });
        }

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
    
    #[wasm_bindgen]
    pub fn handle_mouse_right_click(&self, _x: f64, _y: f64, pressed: bool) -> Result<(), JsValue> {
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let window_event = WindowEvent::MouseInput {
                state: if pressed {
                    ElementState::Pressed
                } else {
                    ElementState::Released
                },
                button: MouseButton::Right,
            };
            instance.chart_engine.handle_cursor_event(window_event);
            
            // Trigger render to show/hide tooltip
            if instance.chart_engine.data_store().is_dirty() {
                let _ = instance.chart_engine.render();
            }
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn get_tooltip_data(&self, x: f64, _y: f64) -> Result<JsValue, JsValue> {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            // Get tooltip data from the data store
            let data_store = instance.chart_engine.data_store();
            
            // Find closest data point at this x position
            if let Some((timestamp, values)) = data_store.find_closest_data_point(x) {
                // Create JavaScript object with tooltip data
                let obj = js_sys::Object::new();
                
                // Format timestamp as readable date string in UTC
                // The timestamp is Unix seconds, convert to milliseconds for JavaScript Date
                let date = js_sys::Date::new(&JsValue::from_f64((timestamp as f64) * 1000.0));
                
                // Create a more readable format in UTC: "YYYY-MM-DD HH:MM:SS UTC"
                let year = date.get_utc_full_year();
                let month = format!("{:02}", date.get_utc_month() + 1); // getUTCMonth is 0-indexed
                let day = format!("{:02}", date.get_utc_date());
                let hours = format!("{:02}", date.get_utc_hours());
                let minutes = format!("{:02}", date.get_utc_minutes());
                let seconds = format!("{:02}", date.get_utc_seconds());
                
                let time_str = format!("{}-{}-{} {}:{}:{} UTC", year, month, day, hours, minutes, seconds);
                
                js_sys::Reflect::set(&obj, &JsValue::from_str("time"), &JsValue::from_str(&time_str))?;
                
                // Add best bid and best ask if available
                for (name, value, _) in &values {
                    if name.to_lowercase().contains("bid") {
                        js_sys::Reflect::set(&obj, &JsValue::from_str("best_bid"), &JsValue::from_f64(*value as f64))?;
                    } else if name.to_lowercase().contains("ask") {
                        js_sys::Reflect::set(&obj, &JsValue::from_str("best_ask"), &JsValue::from_f64(*value as f64))?;
                    }
                    
                    // Also set individual metric values by name
                    let safe_name = name.replace(" ", "_").to_lowercase();
                    js_sys::Reflect::set(&obj, &JsValue::from_str(&safe_name), &JsValue::from_f64(*value as f64))?;
                }
                
                Ok(JsValue::from(obj))
            } else {
                Ok(JsValue::NULL)
            }
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?
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
