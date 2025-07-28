//! WASM Bridge crate for GPU Charts
//! Central orchestration layer that bridges JavaScript and Rust/WebGPU worlds

#![allow(clippy::uninlined_format_args)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Core modules
pub mod controls;
pub mod instance_manager;
pub mod line_graph;
pub mod wrappers;

use uuid::Uuid;
use web_sys::HtmlCanvasElement;

use controls::canvas_controller::CanvasController;
use instance_manager::InstanceManager;
use line_graph::LineGraph;
use shared_types::events::{
    ElementState, MouseButton, MouseScrollDelta, PhysicalPosition, TouchPhase, WindowEvent,
};
use shared_types::store_state::{
    ChangeDetectionConfig, StateChangeDetection, StoreState, StoreValidationResult,
};
use shared_types::{GpuChartsResult, TradeData};

extern crate nalgebra_glm as glm;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct Chart {
    instance_id: Uuid,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Chart {
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Chart {
        // Create a new instance ID but don't create the actual instance yet
        // That happens in init()
        Chart {
            instance_id: Uuid::new_v4(),
        }
    }

    #[wasm_bindgen]
    pub async fn init(&mut self, canvas_id: &str, width: u32, height: u32) -> Result<(), JsValue> {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                // Only set panic hook if not already set
                use std::sync::Once;
                static INIT: Once = Once::new();
                INIT.call_once(|| {
                    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                    // Try to initialize logger, but don't panic if it fails (already initialized)
                    let _ = console_log::init_with_level(log::Level::Debug);
                });
            }
        }

        log::info!("Initializing chart with canvas: {canvas_id}, size: {width}x{height}");

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

        // Initialize the line graph directly with canvas
        let line_graph = LineGraph::new(width, height, canvas)
            .await
            .map_err(|e| format!("Failed to create LineGraph: {e:?}"))?;

        // Create canvas controller
        let canvas_controller = CanvasController::new();

        // Store instance using the instance manager
        self.instance_id = InstanceManager::create_instance(line_graph, canvas_controller);

        // Initial render
        self.render().await?;

        log::info!("Chart initialized successfully");
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
                        let result = instance.line_graph.render().await;

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
    pub fn needs_render(&self) -> bool {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            // Check if renderer needs a render
            instance.line_graph.renderer.needs_render()
        })
        .unwrap_or(false)
    }

    /// Poll GPU operations and check if a render is needed
    /// This should be called periodically from JavaScript to ensure GPU bounds are read
    #[wasm_bindgen]
    pub fn poll_gpu_operations(&self) -> bool {
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Poll the device to progress any pending GPU operations
            instance
                .line_graph
                .renderer
                .device
                .poll(wgpu::Maintain::Poll);

            // Check if we need to render (e.g., GPU bounds are pending)
            instance.line_graph.renderer.needs_render()
        })
        .unwrap_or(false)
    }

    #[wasm_bindgen]
    pub fn resize(&self, width: u32, height: u32) -> Result<(), JsValue> {
        log::info!("Resizing chart to: {width}x{height}");

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            instance.line_graph.resized(width, height);
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, _y: f64) -> Result<(), JsValue> {
        log::info!(
            "[WASM] handle_mouse_wheel called with delta_y={}, x={}",
            delta_y,
            x
        );

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let window_event = WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(x, delta_y)),
                phase: TouchPhase::Moved,
            };

            log::info!("[WASM] Created MouseWheel event, passing to canvas_controller");

            instance
                .canvas_controller
                .handle_cursor_event(window_event, &mut instance.line_graph.renderer);

            // After zoom, ensure bounds are recalculated
            let renderer = &mut instance.line_graph.renderer;
            let data_store = renderer.data_store_mut();

            log::info!("[WASM] After handle_cursor_event - data_store is_dirty: {}, min_y: {:?}, max_y: {:?}", 
                data_store.is_dirty(), data_store.min_y, data_store.max_y);

            if data_store.is_dirty() {
                // Force recalculation of Y bounds by clearing them
                data_store.min_y = None;
                data_store.max_y = None;
                log::info!("[WASM] Cleared Y bounds for recalculation");
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
                .canvas_controller
                .handle_cursor_event(window_event, &mut instance.line_graph.renderer);
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
                .canvas_controller
                .handle_cursor_event(window_event, &mut instance.line_graph.renderer);

            // After drag zoom (on release), ensure bounds are recalculated
            if !pressed {
                let renderer = &mut instance.line_graph.renderer;
                let data_store = renderer.data_store_mut();
                if data_store.is_dirty() {
                    // Force recalculation of Y bounds by clearing them
                    data_store.min_y = None;
                    data_store.max_y = None;
                }
            }
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_data_range(&self, start: u32, end: u32) -> Result<(), JsValue> {
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            instance
                .line_graph
                .renderer
                .data_store_mut()
                .set_x_range(start, end);
        })
        .ok_or_else(|| JsValue::from_str("Chart instance not found"))?;

        Ok(())
    }

    #[wasm_bindgen]
    pub fn request_redraw(&self) -> Result<(), JsValue> {
        // Simply mark that a redraw is needed - actual rendering will happen on next frame
        log::debug!("Redraw requested");
        // In a real implementation, this would set a flag that the render loop checks
        // For now, we'll just log the request since the RefCell borrow checker is being problematic
        Ok(())
    }

    /// Core bridge method: Update chart state from React store
    /// This is the main integration point between React and Rust
    #[wasm_bindgen]
    pub fn update_chart_state(&self, store_state_json: &str) -> Result<String, JsValue> {
        log::info!("update_chart_state called with: {store_state_json}");

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

        // Step 2: Smart change detection
        let result = InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            let change_detection = if let Some(ref current_state) = instance.current_store_state {
                store_state.detect_changes_from(current_state, &instance.change_detection_config)
            } else {
                // First time - treat everything as changed
                StateChangeDetection {
                    has_changes: true,
                    symbol_changed: true,
                    time_range_changed: true,
                    preset_changed: true,
                    requires_data_fetch: true,
                    requires_render: true,
                    change_summary: vec!["Initial state setup".to_string()],
                }
            };

            if !change_detection.has_changes {
                log::info!("Store state unchanged, skipping update");
                let response = serde_json::json!({
                    "success": true,
                    "message": "No changes detected",
                    "updated": false,
                    "changeDetection": {
                        "hasChanges": false,
                        "summary": []
                    }
                });
                return Ok(response.to_string());
            }

            // Step 3: Apply the state changes

            // Step 4: Apply the state changes using smart detection
            match self.apply_smart_state_changes(&store_state, &change_detection, instance) {
                Ok(changes_applied) => {
                    // Step 4.5: Handle data fetching for preset changes
                    if change_detection.preset_changed && change_detection.requires_data_fetch {
                        log::info!("Triggering data fetch for preset changes - should be handled by parent component");
                        // Note: Data fetching should be triggered by the parent component
                        // that has access to the DataManager instance
                    }

                    // Step 4: Update stored state
                    instance.current_store_state = Some(store_state);

                    // Step 5: Return detailed success response
                    let response = serde_json::json!({
                        "success": true,
                        "message": "Chart state updated successfully",
                        "updated": true,
                        "changes": changes_applied,
                        "changeDetection": {
                            "hasChanges": change_detection.has_changes,
                            "symbolChanged": change_detection.symbol_changed,
                            "timeRangeChanged": change_detection.time_range_changed,
                            "presetChanged": change_detection.preset_changed,
                            "requiresDataFetch": change_detection.requires_data_fetch,
                            "requiresRender": change_detection.requires_render,
                            "summary": change_detection.change_summary
                        }
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
        });

        result.unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Check if the chart is initialized and has an active instance
    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        InstanceManager::instance_exists(&self.instance_id)
    }

    /// Get current store state as JSON (for debugging/sync purposes)
    #[wasm_bindgen]
    pub fn get_current_store_state(&self) -> Result<String, JsValue> {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            if let Some(ref state) = instance.current_store_state {
                match serde_json::to_string(state) {
                    Ok(json) => Ok(json),
                    Err(e) => Err(JsValue::from_str(&format!("Serialization failed: {e}"))),
                }
            } else {
                Ok("null".to_string())
            }
        })
        .unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
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

        let result = InstanceManager::with_instance_mut(&self.instance_id, |instance| {
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
        });

        result.unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Configure change detection behavior
    #[wasm_bindgen]
    pub fn configure_change_detection(&self, config_json: &str) -> Result<String, JsValue> {
        log::info!("configure_change_detection called with: {config_json}");

        // Parse the configuration JSON first
        let config: ChangeDetectionConfig = match serde_json::from_str(config_json) {
            Ok(config) => config,
            Err(e) => {
                let error_response = serde_json::json!({
                    "success": false,
                    "errors": [format!("Invalid configuration JSON: {}", e)],
                    "warnings": []
                });
                return Ok(error_response.to_string());
            }
        };

        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Update the configuration
            instance.change_detection_config = config;

            let response = serde_json::json!({
                "success": true,
                "message": "Change detection configuration updated",
                "config": {
                    "enableSymbolChangeDetection": instance.change_detection_config.enable_symbol_change_detection,
                    "enableTimeRangeChangeDetection": instance.change_detection_config.enable_time_range_change_detection,
                    "enablePresetChangeDetection": instance.change_detection_config.enable_preset_change_detection,
                    "symbolChangeTriggersFetch": instance.change_detection_config.symbol_change_triggers_fetch,
                    "timeRangeChangeTriggersFetch": instance.change_detection_config.time_range_change_triggers_fetch,
                    "presetChangeTriggersFetch": instance.change_detection_config.preset_change_triggers_fetch,
                    "minimumTimeRangeChangeSeconds": instance.change_detection_config.minimum_time_range_change_seconds
                }
            });
            Ok(response.to_string())
        })
        .unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Get current change detection configuration
    #[wasm_bindgen]
    pub fn get_change_detection_config(&self) -> Result<String, JsValue> {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            let config_json = serde_json::json!({
                "enableSymbolChangeDetection": instance.change_detection_config.enable_symbol_change_detection,
                "enableTimeRangeChangeDetection": instance.change_detection_config.enable_time_range_change_detection,
                "enablePresetChangeDetection": instance.change_detection_config.enable_preset_change_detection,
                "symbolChangeTriggersFetch": instance.change_detection_config.symbol_change_triggers_fetch,
                "timeRangeChangeTriggersFetch": instance.change_detection_config.time_range_change_triggers_fetch,
                "presetChangeTriggersFetch": instance.change_detection_config.preset_change_triggers_fetch,
                "minimumTimeRangeChangeSeconds": instance.change_detection_config.minimum_time_range_change_seconds
            });
            Ok(config_json.to_string())
        })
        .unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Detect changes between current state and provided state without applying them
    #[wasm_bindgen]
    pub fn detect_state_changes(&self, store_state_json: &str) -> Result<String, JsValue> {
        log::info!("detect_state_changes called");

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

        InstanceManager::with_instance(&self.instance_id, |instance| {
            let change_detection = if let Some(ref current_state) = instance.current_store_state {
                store_state.detect_changes_from(current_state, &instance.change_detection_config)
            } else {
                StateChangeDetection {
                    has_changes: true,
                    symbol_changed: true,
                    time_range_changed: true,
                    preset_changed: true,
                    requires_data_fetch: true,
                    requires_render: true,
                    change_summary: vec!["No previous state for comparison".to_string()],
                }
            };

            let response = serde_json::json!({
                "success": true,
                "changeDetection": {
                    "hasChanges": change_detection.has_changes,
                    "symbolChanged": change_detection.symbol_changed,
                    "timeRangeChanged": change_detection.time_range_changed,
                    "presetChanged": change_detection.preset_changed,
                    "requiresDataFetch": change_detection.requires_data_fetch,
                    "requiresRender": change_detection.requires_render,
                    "summary": change_detection.change_summary
                }
            });
            Ok(response.to_string())
        })
        .unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Set the chart type (line or candlestick)
    #[wasm_bindgen]
    pub fn set_chart_type(&self, chart_type: &str) -> Result<(), JsValue> {
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            instance.line_graph.set_chart_type(chart_type);
        })
        .ok_or_else(|| JsValue::from_str("Chart not initialized"))?;

        Ok(())
    }

    /// Set the candle timeframe in seconds (e.g., 60 for 1 minute, 300 for 5 minutes)
    #[wasm_bindgen]
    pub fn set_candle_timeframe(&self, timeframe_seconds: u32) -> Result<(), JsValue> {
        InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            instance.line_graph.set_candle_timeframe(timeframe_seconds);
        })
        .ok_or_else(|| JsValue::from_str("Chart not initialized"))?;

        Ok(())
    }

    /// Apply a rendering preset by name and fetch required data
    #[wasm_bindgen]
    pub async fn apply_preset(&self, preset_name: &str, symbol: &str, start_time: u64, end_time: u64) -> Result<String, JsValue> {
        log::info!(
            "🎨 [apply_preset] Starting preset application: '{}'",
            preset_name
        );

        let result = InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Get the preset from the preset manager
            let preset = match instance.line_graph.preset_manager.get_preset(preset_name) {
                Some(p) => {
                    log::info!(
                        "✅ [apply_preset] Found preset '{}' with {} chart types",
                        preset_name,
                        p.chart_types.len()
                    );
                    for (idx, ct) in p.chart_types.iter().enumerate() {
                        log::info!("  📊 Chart type[{}]: label='{}', visible={}, render_type={:?}, data_columns={:?}",
                            idx, ct.label, ct.visible, ct.render_type, ct.data_columns);
                    }
                    p
                }
                None => {
                    let available = instance.line_graph.preset_manager.list_presets();
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": format!("Preset '{}' not found", preset_name),
                        "available_presets": available
                    });
                    return Ok(error_response.to_string());
                }
            };

            // Create a multi-renderer based on the preset
            let mut multi_renderer = instance
                .line_graph
                .renderer
                .create_multi_renderer()
                .with_render_order(renderer::RenderOrder::BackgroundToForeground)
                .build();

            // Add renderers based on the preset chart types
            let mut data_requirements = Vec::new();
            log::info!("🏗️ [apply_preset] Building multi-renderer from preset chart types:");

            for (idx, chart_preset) in preset.chart_types.iter().enumerate() {
                // Only add renderer if the chart type is visible
                if chart_preset.visible {
                    log::info!(
                        "  ✅ Adding renderer[{}] for '{}' (render_type={:?})",
                        idx,
                        chart_preset.label,
                        chart_preset.render_type
                    );
                    use config_system::RenderType;

                    match chart_preset.render_type {
                        RenderType::Line => {
                            // Check if this is a computed line (e.g., mid price)
                            if let Some(compute_op) = &chart_preset.compute_op {
                                use config_system::ComputeOp;
                                match compute_op {
                                    ComputeOp::Average if chart_preset.data_columns.len() == 2 => {
                                        // Check if this is the mid price calculation (bid/ask average)
                                        let is_mid_price = chart_preset
                                            .data_columns
                                            .iter()
                                            .any(|(_, col)| col == "best_bid")
                                            && chart_preset
                                                .data_columns
                                                .iter()
                                                .any(|(_, col)| col == "best_ask");

                                        if is_mid_price {
                                            // Mid price is computed by the GPU compute engine during rendering
                                            log::debug!("  Creating ConfigurablePlotRenderer for '{}' (GPU-computed metric)", chart_preset.label);
                                            let plot_renderer =
                                                renderer::ConfigurablePlotRenderer::new(
                                                    instance.line_graph.renderer.device.clone(),
                                                    instance.line_graph.renderer.queue.clone(),
                                                    instance.line_graph.renderer.config.format,
                                                    format!("PlotRenderer_{}", chart_preset.label),
                                                    vec![(
                                                        "MD".to_string(),
                                                        "mid_price".to_string(),
                                                    )], // GPU-computed metric in MD group
                                                );
                                            multi_renderer.add_renderer(Box::new(plot_renderer));
                                        } else {
                                            // For other averages, use regular plot renderer for now
                                            log::info!("  Creating ConfigurablePlotRenderer for '{}' with source data", chart_preset.label);
                                            let plot_renderer =
                                                renderer::ConfigurablePlotRenderer::new(
                                                    instance.line_graph.renderer.device.clone(),
                                                    instance.line_graph.renderer.queue.clone(),
                                                    instance.line_graph.renderer.config.format,
                                                    format!("PlotRenderer_{}", chart_preset.label),
                                                    chart_preset.data_columns.clone(),
                                                );
                                            multi_renderer.add_renderer(Box::new(plot_renderer));
                                        }
                                    }
                                    _ => {
                                        // For other compute operations, use regular plot renderer for now
                                        log::info!("  Creating ConfigurablePlotRenderer for '{}' (compute op not GPU-accelerated yet)", chart_preset.label);
                                        let plot_renderer = renderer::ConfigurablePlotRenderer::new(
                                            instance.line_graph.renderer.device.clone(),
                                            instance.line_graph.renderer.queue.clone(),
                                            instance.line_graph.renderer.config.format,
                                            format!("PlotRenderer_{}", chart_preset.label),
                                            chart_preset.data_columns.clone(),
                                        );
                                        multi_renderer.add_renderer(Box::new(plot_renderer));
                                    }
                                }
                            } else {
                                // No compute operation, regular line rendering
                                let plot_renderer = renderer::ConfigurablePlotRenderer::new(
                                    instance.line_graph.renderer.device.clone(),
                                    instance.line_graph.renderer.queue.clone(),
                                    instance.line_graph.renderer.config.format,
                                    format!("PlotRenderer_{}", chart_preset.label),
                                    chart_preset.data_columns.clone(),
                                );
                                log::info!("  Created ConfigurablePlotRenderer for '{}' with columns: {:?}", 
                                chart_preset.label, chart_preset.data_columns);
                                multi_renderer.add_renderer(Box::new(plot_renderer));
                            }
                        }
                        RenderType::Triangle => {
                            // Create a TriangleRenderer for trade markers
                            let mut triangle_renderer = renderer::TriangleRenderer::new(
                                instance.line_graph.renderer.device.clone(),
                                instance.line_graph.renderer.queue.clone(),
                                instance.line_graph.renderer.config.format,
                            );

                            // Set triangle size from style
                            triangle_renderer.set_triangle_size(chart_preset.style.size);

                            // Set the data group name - extract from data_columns
                            if let Some((data_type, _)) = chart_preset.data_columns.first() {
                                log::info!(
                                    "  Setting triangle renderer data group to '{}'",
                                    data_type
                                );
                                triangle_renderer.set_data_group(data_type.clone());
                            }

                            multi_renderer.add_renderer(Box::new(triangle_renderer));
                        }
                        RenderType::Candlestick => {
                            // Create a CandlestickRenderer
                            let candlestick_renderer = renderer::CandlestickRenderer::new(
                                instance.line_graph.renderer.device.clone(),
                                instance.line_graph.renderer.queue.clone(),
                                instance.line_graph.renderer.config.format,
                            );
                            multi_renderer.add_renderer(Box::new(candlestick_renderer));
                        }
                        RenderType::Bar => {
                            // Bar renderer not yet implemented, log warning
                            log::warn!("Bar renderer not yet implemented for preset");
                        }
                        RenderType::Area => {
                            // Area renderer not yet implemented, log warning
                            log::warn!("Area renderer not yet implemented for preset");
                        }
                    }
                } else {
                    log::info!(
                        "  ❌ Skipping renderer[{}] for '{}' (visible=false)",
                        idx,
                        chart_preset.label
                    );
                }

                // Collect data requirements for all chart types (visible or not)
                for (data_type, column) in &chart_preset.data_columns {
                    data_requirements.push((data_type.clone(), column.clone()));
                }
            }

            // Always add axis renderers
            let width = instance.line_graph.renderer.data_store().screen_size.width;
            let height = instance.line_graph.renderer.data_store().screen_size.height;

            let x_axis = renderer::XAxisRenderer::new(
                instance.line_graph.renderer.device.clone(),
                instance.line_graph.renderer.queue.clone(),
                instance.line_graph.renderer.config.format,
                width,
                height,
            );
            multi_renderer.add_renderer(Box::new(x_axis));

            let y_axis = renderer::YAxisRenderer::new(
                instance.line_graph.renderer.device.clone(),
                instance.line_graph.renderer.queue.clone(),
                instance.line_graph.renderer.config.format,
                width,
                height,
            );
            multi_renderer.add_renderer(Box::new(y_axis));

            // Store the multi-renderer
            instance.line_graph.multi_renderer = Some(multi_renderer);

            // Store the active preset name
            instance.active_preset = Some(preset_name.to_string());

            log::info!(
                "🎯 [apply_preset] Updating excluded columns for preset '{}'",
                preset_name
            );

            // Update excluded columns for Y bounds calculation
            let excluded_columns = instance
                .line_graph
                .get_excluded_columns_from_preset(preset_name);
            log::info!(
                "🎯 [apply_preset] Excluded columns to set: {:?}",
                excluded_columns
            );

            instance
                .line_graph
                .renderer
                .data_store_mut()
                .set_excluded_columns(excluded_columns);

            // Mark renderer as dirty to trigger re-render
            instance.line_graph.renderer.data_store_mut().mark_dirty();

            log::info!(
                "🎯 [apply_preset] Preset '{}' applied successfully",
                preset_name
            );

            let response = serde_json::json!({
                "success": true,
                "preset_applied": preset_name,
                "description": preset.description,
                "data_requirements": data_requirements,
                "renderer_count": instance.line_graph.multi_renderer.as_ref().map(|mr| mr.renderer_count()).unwrap_or(0),
                "needs_data_fetch": true
            });

            Ok(response.to_string())
        });

        let initial_result = result.unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))?;
        
        // Parse the initial result to check if preset was applied successfully
        let initial_response: serde_json::Value = serde_json::from_str(&initial_result)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse response: {}", e)))?;
        
        if initial_response["success"].as_bool().unwrap_or(false) {
            // Now fetch the data for the preset
            log::info!("📡 [apply_preset] Fetching data for preset");
            let fetch_result = self.fetch_preset_data(symbol, start_time, end_time).await?;
            
            // Merge the results
            let fetch_response: serde_json::Value = serde_json::from_str(&fetch_result)
                .map_err(|e| JsValue::from_str(&format!("Failed to parse fetch response: {}", e)))?;
            
            let final_response = serde_json::json!({
                "success": initial_response["success"],
                "preset_applied": initial_response["preset_applied"],
                "description": initial_response["description"],
                "data_requirements": initial_response["data_requirements"],
                "renderer_count": initial_response["renderer_count"],
                "data_fetch": fetch_response
            });
            
            Ok(final_response.to_string())
        } else {
            Ok(initial_result)
        }
    }

    /// Get the currently active preset
    #[wasm_bindgen]
    pub fn get_active_preset(&self) -> Result<String, JsValue> {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            let response = serde_json::json!({
                "active_preset": instance.active_preset.as_ref()
            });
            Ok(response.to_string())
        })
        .unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Get the current visibility states of chart types in the active preset
    #[wasm_bindgen]
    pub fn get_preset_chart_states(&self) -> Result<String, JsValue> {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            // Check if a preset is active
            let preset_name = match &instance.active_preset {
                Some(name) => name.clone(),
                None => {
                    let response = serde_json::json!({
                        "success": false,
                        "error": "No preset is currently active",
                        "chart_states": []
                    });
                    return Ok(response.to_string());
                }
            };

            // Get the preset
            let preset = match instance.line_graph.preset_manager.get_preset(&preset_name) {
                Some(p) => p,
                None => {
                    let response = serde_json::json!({
                        "success": false,
                        "error": format!("Active preset '{}' not found", preset_name),
                        "chart_states": []
                    });
                    return Ok(response.to_string());
                }
            };

            // Get the visibility states
            let chart_states: Vec<_> = preset
                .chart_types
                .iter()
                .map(|cp| {
                    serde_json::json!({
                        "label": cp.label,
                        "visible": cp.visible,
                        "render_type": format!("{:?}", cp.render_type),
                        "data_columns": cp.data_columns
                    })
                })
                .collect();

            let response = serde_json::json!({
                "success": true,
                "preset_name": preset_name,
                "chart_states": chart_states
            });

            Ok(response.to_string())
        })
        .unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// List all available presets
    #[wasm_bindgen]
    pub fn list_presets(&self) -> Result<String, JsValue> {
        InstanceManager::with_instance(&self.instance_id, |instance| {
            let presets = instance.line_graph.preset_manager.get_all_presets();

            // Convert presets to JSON-serializable format
            let presets_json: Vec<serde_json::Value> = presets
                .iter()
                .map(|preset| {
                    serde_json::json!({
                        "name": preset.name,
                        "description": preset.description,
                        "chart_types": preset.chart_types.iter().map(|ct| {
                            serde_json::json!({
                                "render_type": format!("{:?}", ct.render_type),
                                "data_columns": ct.data_columns,
                                "visible": ct.visible,
                                "label": ct.label,
                                "style": ct.style,
                                "compute_op": ct.compute_op
                            })
                        }).collect::<Vec<_>>()
                    })
                })
                .collect();

            let response = serde_json::json!({
                "presets": presets_json
            });
            Ok(response.to_string())
        })
        .unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Clear the active preset and return to normal rendering
    #[wasm_bindgen]
    pub fn clear_preset(&self) -> Result<String, JsValue> {
        log::info!("Clearing active preset");

        let result = InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Clear the multi-renderer
            instance.line_graph.multi_renderer = None;

            // Clear the active preset name
            let previous_preset = instance.active_preset.take();

            // Mark renderer as dirty to trigger re-render
            instance.line_graph.renderer.data_store_mut().mark_dirty();

            let response = serde_json::json!({
                "success": true,
                "previous_preset": previous_preset,
                "message": "Preset cleared, returning to standard rendering"
            });

            Ok(response.to_string())
        });

        result.unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Toggle visibility of a specific metric in the active preset
    #[wasm_bindgen]
    pub fn toggle_metric_visibility(&self, metric_id: &str) -> Result<String, JsValue> {
        log::info!(
            "🔄 [toggle_metric_visibility] Starting toggle for metric: '{}'",
            metric_id
        );

        let result = InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Check if a preset is active
            let preset_name = match &instance.active_preset {
                Some(name) => {
                    log::info!("📌 [toggle_metric_visibility] Active preset: '{}'", name);
                    name.clone()
                }
                None => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": "No preset is currently active"
                    });
                    return Ok(error_response.to_string());
                }
            };

            // Get the preset
            let preset = match instance.line_graph.preset_manager.get_preset(&preset_name) {
                Some(p) => {
                    log::info!(
                        "✅ [toggle_metric_visibility] Found preset with {} chart types",
                        p.chart_types.len()
                    );
                    p
                }
                None => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": format!("Active preset '{}' not found", preset_name)
                    });
                    return Ok(error_response.to_string());
                }
            };

            // Find the chart type index and toggle its visibility
            let mut preset_clone = preset.clone();
            let mut found = false;
            log::info!(
                "🔍 [toggle_metric_visibility] Searching for metric '{}' in preset",
                metric_id
            );

            for (idx, chart_preset) in preset_clone.chart_types.iter_mut().enumerate() {
                log::info!(
                    "  - Chart type[{}]: label='{}', visible={}, render_type={:?}",
                    idx,
                    chart_preset.label,
                    chart_preset.visible,
                    chart_preset.render_type
                );

                if chart_preset.label == metric_id {
                    let old_visibility = chart_preset.visible;
                    chart_preset.visible = !chart_preset.visible;
                    log::info!(
                        "🎯 [toggle_metric_visibility] Found match! Toggling '{}' from {} to {}",
                        metric_id,
                        old_visibility,
                        chart_preset.visible
                    );
                    found = true;
                    break;
                }
            }

            if !found {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": format!("Metric '{}' not found in preset", metric_id)
                });
                return Ok(error_response.to_string());
            }

            // Save the updated preset state back to the PresetManager
            log::info!("💾 [toggle_metric_visibility] Saving updated preset state");
            instance
                .line_graph
                .preset_manager
                .update_preset(&preset_name, preset_clone.clone());

            // Rebuild the multi-renderer with updated visibility
            let mut multi_renderer = instance
                .line_graph
                .renderer
                .create_multi_renderer()
                .with_render_order(renderer::RenderOrder::BackgroundToForeground)
                .build();

            // Add only visible renderers
            let mut visible_count = 0;
            log::info!(
                "🏗️ [toggle_metric_visibility] Rebuilding multi-renderer with visible components:"
            );

            for (idx, chart_preset) in preset_clone.chart_types.iter().enumerate() {
                if !chart_preset.visible {
                    log::info!(
                        "  ❌ Skipping chart_type[{}]: '{}' (visible=false)",
                        idx,
                        chart_preset.label
                    );
                    continue;
                }

                visible_count += 1;
                log::info!(
                    "  ✅ Adding chart_type[{}]: '{}' (render_type={:?})",
                    idx,
                    chart_preset.label,
                    chart_preset.render_type
                );

                use config_system::RenderType;

                match chart_preset.render_type {
                    RenderType::Line => {
                        // Check if this is a computed line (e.g., mid price)
                        if let Some(compute_op) = &chart_preset.compute_op {
                            use config_system::ComputeOp;
                            match compute_op {
                                ComputeOp::Average if chart_preset.data_columns.len() == 2 => {
                                    // Check if this is the mid price calculation (bid/ask average)
                                    let is_mid_price = chart_preset
                                        .data_columns
                                        .iter()
                                        .any(|(_, col)| col == "best_bid")
                                        && chart_preset
                                            .data_columns
                                            .iter()
                                            .any(|(_, col)| col == "best_ask");

                                    if is_mid_price {
                                        // Mid price is computed by the GPU compute engine during rendering
                                        log::debug!("  Creating ConfigurablePlotRenderer for '{}' (GPU-computed metric)", chart_preset.label);
                                        let plot_renderer = renderer::ConfigurablePlotRenderer::new(
                                            instance.line_graph.renderer.device.clone(),
                                            instance.line_graph.renderer.queue.clone(),
                                            instance.line_graph.renderer.config.format,
                                            format!("PlotRenderer_{}", chart_preset.label),
                                            vec![("MD".to_string(), "mid_price".to_string())], // GPU-computed metric in MD group
                                        );
                                        multi_renderer.add_renderer(Box::new(plot_renderer));
                                    } else {
                                        // For other averages, use regular plot renderer for now
                                        log::info!("  Creating ConfigurablePlotRenderer for '{}' with source data", chart_preset.label);
                                        let plot_renderer = renderer::ConfigurablePlotRenderer::new(
                                            instance.line_graph.renderer.device.clone(),
                                            instance.line_graph.renderer.queue.clone(),
                                            instance.line_graph.renderer.config.format,
                                            format!("PlotRenderer_{}", chart_preset.label),
                                            chart_preset.data_columns.clone(),
                                        );
                                        multi_renderer.add_renderer(Box::new(plot_renderer));
                                    }
                                }
                                _ => {
                                    // For other compute operations, use regular plot renderer for now
                                    log::info!("  Creating ConfigurablePlotRenderer for '{}' (compute op not GPU-accelerated yet)", chart_preset.label);
                                    let plot_renderer = renderer::ConfigurablePlotRenderer::new(
                                        instance.line_graph.renderer.device.clone(),
                                        instance.line_graph.renderer.queue.clone(),
                                        instance.line_graph.renderer.config.format,
                                        format!("PlotRenderer_{}", chart_preset.label),
                                        chart_preset.data_columns.clone(),
                                    );
                                    multi_renderer.add_renderer(Box::new(plot_renderer));
                                }
                            }
                        } else {
                            // No compute operation, regular line rendering
                            let plot_renderer = renderer::ConfigurablePlotRenderer::new(
                                instance.line_graph.renderer.device.clone(),
                                instance.line_graph.renderer.queue.clone(),
                                instance.line_graph.renderer.config.format,
                                format!("PlotRenderer_{}", chart_preset.label),
                                chart_preset.data_columns.clone(),
                            );
                            log::info!(
                                "  Created ConfigurablePlotRenderer for '{}' with columns: {:?}",
                                chart_preset.label,
                                chart_preset.data_columns
                            );
                            multi_renderer.add_renderer(Box::new(plot_renderer));
                        }
                    }
                    RenderType::Triangle => {
                        let mut triangle_renderer = renderer::TriangleRenderer::new(
                            instance.line_graph.renderer.device.clone(),
                            instance.line_graph.renderer.queue.clone(),
                            instance.line_graph.renderer.config.format,
                        );
                        triangle_renderer.set_triangle_size(chart_preset.style.size);

                        // Set the data group name - extract from data_columns
                        if let Some((data_type, _)) = chart_preset.data_columns.first() {
                            log::info!("  Setting triangle renderer data group to '{}'", data_type);
                            triangle_renderer.set_data_group(data_type.clone());
                        }

                        multi_renderer.add_renderer(Box::new(triangle_renderer));
                    }
                    RenderType::Candlestick => {
                        let candlestick_renderer = renderer::CandlestickRenderer::new(
                            instance.line_graph.renderer.device.clone(),
                            instance.line_graph.renderer.queue.clone(),
                            instance.line_graph.renderer.config.format,
                        );
                        multi_renderer.add_renderer(Box::new(candlestick_renderer));
                    }
                    RenderType::Bar | RenderType::Area => {
                        log::warn!(
                            "{:?} renderer not yet implemented",
                            chart_preset.render_type
                        );
                    }
                }
            }

            // Always add axis renderers
            let width = instance.line_graph.renderer.data_store().screen_size.width;
            let height = instance.line_graph.renderer.data_store().screen_size.height;

            let x_axis = renderer::XAxisRenderer::new(
                instance.line_graph.renderer.device.clone(),
                instance.line_graph.renderer.queue.clone(),
                instance.line_graph.renderer.config.format,
                width,
                height,
            );
            multi_renderer.add_renderer(Box::new(x_axis));

            let y_axis = renderer::YAxisRenderer::new(
                instance.line_graph.renderer.device.clone(),
                instance.line_graph.renderer.queue.clone(),
                instance.line_graph.renderer.config.format,
                width,
                height,
            );
            multi_renderer.add_renderer(Box::new(y_axis));

            // Update the multi-renderer
            log::info!("📦 [toggle_metric_visibility] Setting new multi-renderer with {} visible components", visible_count);
            instance.line_graph.multi_renderer = Some(multi_renderer);

            // Mark renderer as dirty
            instance.line_graph.renderer.data_store_mut().mark_dirty();
            log::info!(
                "🚀 [toggle_metric_visibility] Renderer marked as dirty, ready for re-render"
            );

            // Get the updated visibility state
            let visibility_states: Vec<_> = preset_clone
                .chart_types
                .iter()
                .map(|cp| {
                    serde_json::json!({
                        "label": cp.label,
                        "visible": cp.visible,
                        "render_type": format!("{:?}", cp.render_type)
                    })
                })
                .collect();

            let response = serde_json::json!({
                "success": true,
                "metric_id": metric_id,
                "visible": preset_clone.chart_types.iter().find(|cp| cp.label == metric_id).map(|cp| cp.visible).unwrap_or(false),
                "visible_count": visible_count,
                "all_chart_states": visibility_states
            });

            Ok(response.to_string())
        });

        result.unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Update triangle renderer with trade data
    #[wasm_bindgen]
    pub fn update_trade_data(&self, trades_json: &str) -> Result<String, JsValue> {
        log::info!("Updating trade data for triangle renderer");

        let result = InstanceManager::with_instance_mut(&self.instance_id, |instance| {
            // Parse the trades JSON
            let trades: Vec<TradeData> = match serde_json::from_str(trades_json) {
                Ok(trades) => trades,
                Err(e) => {
                    let error_response = serde_json::json!({
                        "success": false,
                        "error": format!("Failed to parse trades JSON: {}", e)
                    });
                    return Ok(error_response.to_string());
                }
            };

            // Find triangle renderer in multi-renderer and update it
            if let Some(ref mut _multi_renderer) = instance.line_graph.multi_renderer {
                // This is a bit hacky but necessary due to the trait object limitation
                // In a real implementation, we'd need a more elegant solution
                log::info!("Found multi-renderer, but cannot directly update triangle renderer due to trait limitations");

                // For now, we'll store the trades and recreate the renderer
                // This is not ideal but works for the prototype
                let response = serde_json::json!({
                    "success": true,
                    "trades_count": trades.len(),
                    "message": "Trade data received but renderer update not yet implemented"
                });
                Ok(response.to_string())
            } else {
                let error_response = serde_json::json!({
                    "success": false,
                    "error": "No active preset with multi-renderer"
                });
                Ok(error_response.to_string())
            }
        });

        result.unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Check if the required data for the current preset is already loaded
    #[wasm_bindgen]
    pub fn is_preset_data_loaded(&self) -> Result<bool, JsValue> {
        let result = InstanceManager::with_instance(&self.instance_id, |instance| {
            // Check if we have an active preset
            if let Some(preset_name) = &instance.active_preset {
                if let Some(_preset) = instance.line_graph.preset_manager.get_preset(preset_name) {
                    // Check if all required data columns are loaded in the data store
                    let data_store = instance.line_graph.renderer.data_store();

                    // For now, we'll check if we have any data loaded
                    // In a more sophisticated implementation, we'd check specific columns
                    let has_data = data_store.get_data_len() > 0;

                    log::info!(
                        "Checking if preset '{}' data is loaded: {}",
                        preset_name,
                        has_data
                    );
                    return Ok(has_data);
                }
            }
            Ok(false)
        });

        result.unwrap_or_else(|| Err(JsValue::from_str("Chart not initialized")))
    }

    /// Fetch data for the current preset
    #[wasm_bindgen]
    pub async fn fetch_preset_data(
        &self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<String, JsValue> {
        log::info!(
            "🔄 [fetch_preset_data] Starting data fetch: symbol={}, start={}, end={}",
            symbol,
            start_time,
            end_time
        );

        let instance_id = self.instance_id;

        // Get the preset data requirements
        let preset_info = InstanceManager::with_instance(&instance_id, |instance| {
            if let Some(preset_name) = &instance.active_preset {
                log::info!("📌 [fetch_preset_data] Active preset: '{}'", preset_name);
                if let Some(preset) = instance.line_graph.preset_manager.get_preset(preset_name) {
                    // Collect unique data requirements using HashSet to avoid duplicates
                    let mut data_requirements: std::collections::HashMap<String, std::collections::HashSet<String>> = std::collections::HashMap::new();
                    log::info!("📊 [fetch_preset_data] Analyzing {} chart types in preset", preset.chart_types.len());

                    for (idx, chart_preset) in preset.chart_types.iter().enumerate() {
                        log::info!("  Chart type[{}]: label='{}', visible={}, render_type={:?}, data_columns={:?}",
                            idx, chart_preset.label, chart_preset.visible, chart_preset.render_type, chart_preset.data_columns);

                        // Only collect data for visible chart types
                        if chart_preset.visible {
                            // Add main data columns
                            for (data_type, column) in &chart_preset.data_columns {
                                log::info!("    Adding data requirement: type='{}', column='{}'", data_type, column);
                                data_requirements.entry(data_type.clone())
                                    .or_default()
                                    .insert(column.clone());
                            }

                            // Also add additional data columns if present
                            if let Some(additional_cols) = &chart_preset.additional_data_columns {
                                for (data_type, column) in additional_cols {
                                    log::info!("    Adding additional data requirement: type='{}', column='{}'", data_type, column);
                                    data_requirements.entry(data_type.clone())
                                        .or_default()
                                        .insert(column.clone());
                                }
                            }
                        } else {
                            log::info!("    Skipping data fetch for '{}' (not visible)", chart_preset.label);
                        }
                    }

                    log::info!("📦 [fetch_preset_data] Total data requirements: {:?}", data_requirements);
                    Some((preset_name.clone(), data_requirements))
                } else {
                    log::error!("❌ [fetch_preset_data] Preset '{}' not found in manager", preset_name);
                    None
                }
            } else {
                log::warn!("⚠️ [fetch_preset_data] No active preset");
                None
            }
        }).unwrap_or(None);

        if let Some((preset_name, data_requirements)) = preset_info {
            // Fetch data for each data type
            let mut fetch_results: Vec<(String, Result<Vec<String>, String>)> = Vec::new();

            for (data_type, columns) in data_requirements {
                log::info!(
                    "🌐 [fetch_preset_data] Fetching '{}' data with columns: {:?}",
                    data_type,
                    columns
                );

                // Take the instance temporarily for async operation
                let instance_opt = InstanceManager::take_instance(&instance_id);
                match instance_opt {
                    Some(mut instance) => {
                        // Prepare column list with "time" always included
                        let mut all_columns = vec!["time"];
                        let columns_vec: Vec<String> = columns.into_iter().collect();
                        all_columns.extend(columns_vec.iter().map(|s| s.as_str()));

                        log::info!("📡 [fetch_preset_data] Requesting data: symbol='{}', type='{}', columns={:?}", 
                            symbol, data_type, all_columns);

                        // Perform the fetch with proper data_type parameter
                        let result = instance
                            .line_graph
                            .data_manager
                            .fetch_data(symbol, &data_type, start_time, end_time, &all_columns)
                            .await;

                        match result {
                            Ok(data_handle) => {
                                log::info!(
                                    "✅ [fetch_preset_data] Successfully fetched '{}' data",
                                    data_type
                                );
                                // Get device reference before mutable borrow
                                let device = instance.line_graph.renderer.device.clone();
                                // Process the data and add it to the DataStore
                                if let Err(e) = Self::process_data_handle(
                                    &data_handle,
                                    &mut instance.line_graph.data_manager,
                                    instance.line_graph.renderer.data_store_mut(),
                                    &device,
                                ) {
                                    log::error!(
                                        "❌ [fetch_preset_data] Failed to process '{}' data: {:?}",
                                        data_type,
                                        e
                                    );
                                    fetch_results.push((
                                        data_type.to_string(),
                                        Err(format!("Processing failed: {e:?}")),
                                    ));
                                } else {
                                    log::info!(
                                        "✅ [fetch_preset_data] Successfully processed '{}' data",
                                        data_type
                                    );
                                    fetch_results.push((data_type.to_string(), Ok(columns_vec)));
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "❌ [fetch_preset_data] Failed to fetch '{}' data: {:?}",
                                    data_type,
                                    e
                                );
                                fetch_results.push((
                                    data_type.to_string(),
                                    Err(format!("Fetch failed: {e:?}")),
                                ));
                            }
                        }

                        // Put the instance back
                        InstanceManager::put_instance(instance_id, instance);
                    }
                    None => {
                        return Err(JsValue::from_str(
                            "Failed to take instance for data fetching",
                        ));
                    }
                }
            }

            // After fetching all data, computed fields will be handled by the compute engine
            log::debug!("[fetch_preset_data] Computed fields will be processed by compute engine during rendering");
            // Build response
            let mut successes = Vec::new();
            let mut failures = Vec::new();

            for (data_type, result) in fetch_results {
                match result {
                    Ok(columns) => successes.push(serde_json::json!({
                        "data_type": data_type,
                        "columns": columns
                    })),
                    Err(error) => failures.push(serde_json::json!({
                        "data_type": data_type,
                        "error": error
                    })),
                }
            }

            let response = serde_json::json!({
                "success": failures.is_empty(),
                "preset": preset_name,
                "symbol": symbol,
                "time_range": {
                    "start": start_time,
                    "end": end_time
                },
                "data_fetched": successes,
                "failures": failures
            });

            Ok(response.to_string())
        } else {
            let response = serde_json::json!({
                "success": false,
                "error": "No active preset or preset not found"
            });
            Ok(response.to_string())
        }
    }
}

// Private implementation methods for Chart
#[cfg(target_arch = "wasm32")]
impl Chart {
    /// Process data handle and update data store
    fn process_data_handle(
        data_handle: &shared_types::DataHandle,
        data_manager: &mut data_manager::DataManager,
        data_store: &mut data_manager::DataStore,
        _device: &wgpu::Device,
    ) -> Result<(), shared_types::GpuChartsError> {
        // Get the GPU buffer set from the data manager
        let gpu_buffer_set = data_manager.get_buffers(data_handle).ok_or_else(|| {
            shared_types::GpuChartsError::DataNotFound {
                resource: "GPU buffers for data handle".to_string(),
            }
        })?;

        // Extract the time column (shared x-axis for all metrics)
        let time_buffer = gpu_buffer_set.raw_buffers.get("time").ok_or_else(|| {
            shared_types::GpuChartsError::DataNotFound {
                resource: "Time column in data".to_string(),
            }
        })?;

        let time_gpu_buffers = gpu_buffer_set.buffers.get("time").ok_or_else(|| {
            shared_types::GpuChartsError::DataNotFound {
                resource: "Time GPU buffers".to_string(),
            }
        })?;

        // Add a new data group for this data type
        data_store.add_data_group((time_buffer.clone(), time_gpu_buffers.clone()), true);
        let data_group_index = data_store.data_groups.len() - 1;

        // Add each metric column
        let mut has_bid = false;
        let mut has_ask = false;

        for column_name in &gpu_buffer_set.metadata.columns {
            if column_name == "time" {
                continue; // Skip time column as it's already the x-axis
            }

            // Track if we have bid/ask data
            if column_name == "best_bid" {
                has_bid = true;
            } else if column_name == "best_ask" {
                has_ask = true;
            }

            if let (Some(raw_buffer), Some(gpu_buffers)) = (
                gpu_buffer_set.raw_buffers.get(column_name),
                gpu_buffer_set.buffers.get(column_name),
            ) {
                // Assign colors based on column name
                let color = match column_name.as_str() {
                    "best_bid" => [0.0, 0.5, 1.0], // Blue
                    "best_ask" => [1.0, 0.2, 0.2], // Red
                    "price" => [0.0, 1.0, 0.0],    // Green
                    "volume" => [1.0, 1.0, 0.0],   // Yellow
                    "side" => [0.5, 0.5, 0.5],     // Gray (not displayed as a line)
                    _ => {
                        // Generate a color based on hash
                        let hash = column_name
                            .chars()
                            .fold(0u32, |acc, c| acc.wrapping_add(c as u32));
                        let hue = (hash % 360) as f32;
                        let (r, g, b) = Self::hsv_to_rgb(hue, 0.8, 0.9);
                        [r, g, b]
                    }
                };

                data_store.add_metric_to_group(
                    data_group_index,
                    (raw_buffer.clone(), gpu_buffers.clone()),
                    color,
                    column_name.clone(),
                );
            }
        }

        // If we have both bid and ask, add a computed mid_price metric
        if has_bid && has_ask {
            log::debug!("[process_data_handle] Adding computed mid_price metric");

            // Find the bid and ask metric refs
            let bid_ref = data_store.find_metric("best_bid");
            let ask_ref = data_store.find_metric("best_ask");

            if let (Some(bid_ref), Some(ask_ref)) = (bid_ref, ask_ref) {
                data_store.add_computed_metric_to_group(
                    data_group_index,
                    "mid_price".to_string(),
                    [0.7, 0.7, 1.0], // Light blue
                    data_manager::ComputeType::Average,
                    vec![bid_ref, ask_ref],
                );
                log::debug!("[process_data_handle] Successfully added computed mid_price metric");
            } else {
                log::error!("[process_data_handle] Failed to find bid/ask metrics for mid_price computation");
            }
        }

        log::info!(
            "Successfully added {} columns to DataStore for data type",
            gpu_buffer_set.metadata.columns.len()
        );

        // Calculate initial CPU bounds for immediate rendering
        // This provides reasonable bounds while GPU calculation happens asynchronously
        #[allow(deprecated)]
        data_store.recalculate_y_bounds();

        Ok(())
    }

    /// Convert HSV to RGB color
    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match (h / 60.0) as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            5 => (c, 0.0, x),
            _ => (0.0, 0.0, 0.0),
        };

        (r + m, g + m, b + m)
    }

    /// Compute derived metrics based on ComputeOp
    #[allow(dead_code)]
    fn compute_derived_metrics(
        &self,
        columns: &[String],
        compute_op: &config_system::ComputeOp,
        buffers: &std::collections::HashMap<String, Vec<f32>>,
    ) -> Result<Vec<f32>, String> {
        use config_system::ComputeOp;

        // Validate we have the required columns
        for col in columns {
            if !buffers.contains_key(col) {
                return Err(format!("Missing required column: {col}"));
            }
        }

        // Get the first column's length as reference
        let length = buffers.get(&columns[0]).ok_or("No columns provided")?.len();

        // Validate all columns have the same length
        for col in columns {
            if buffers[col].len() != length {
                return Err(format!("Column {col} has different length"));
            }
        }

        let mut result = Vec::with_capacity(length);

        match compute_op {
            ComputeOp::Average => {
                // Calculate average of all columns at each time point
                for i in 0..length {
                    let sum: f32 = columns.iter().map(|col| buffers[col][i]).sum();
                    result.push(sum / columns.len() as f32);
                }
            }
            ComputeOp::Sum => {
                for i in 0..length {
                    let sum: f32 = columns.iter().map(|col| buffers[col][i]).sum();
                    result.push(sum);
                }
            }
            ComputeOp::Min => {
                for i in 0..length {
                    let min = columns
                        .iter()
                        .map(|col| buffers[col][i])
                        .fold(f32::INFINITY, f32::min);
                    result.push(min);
                }
            }
            ComputeOp::Max => {
                for i in 0..length {
                    let max = columns
                        .iter()
                        .map(|col| buffers[col][i])
                        .fold(f32::NEG_INFINITY, f32::max);
                    result.push(max);
                }
            }
            ComputeOp::Difference => {
                if columns.len() != 2 {
                    return Err("Difference operation requires exactly 2 columns".to_string());
                }
                for i in 0..length {
                    result.push(buffers[&columns[0]][i] - buffers[&columns[1]][i]);
                }
            }
            ComputeOp::Product => {
                for i in 0..length {
                    let product: f32 = columns.iter().map(|col| buffers[col][i]).product();
                    result.push(product);
                }
            }
            ComputeOp::Ratio => {
                if columns.len() != 2 {
                    return Err("Ratio operation requires exactly 2 columns".to_string());
                }
                for i in 0..length {
                    let denominator = buffers[&columns[1]][i];
                    if denominator == 0.0 {
                        result.push(f32::NAN);
                    } else {
                        result.push(buffers[&columns[0]][i] / denominator);
                    }
                }
            }
            ComputeOp::WeightedAverage { weights } => {
                if weights.len() != columns.len() {
                    return Err("Number of weights must match number of columns".to_string());
                }
                let total_weight: f32 = weights.iter().sum();
                for i in 0..length {
                    let weighted_sum: f32 = columns
                        .iter()
                        .zip(weights.iter())
                        .map(|(col, weight)| buffers[col][i] * weight)
                        .sum();
                    result.push(weighted_sum / total_weight);
                }
            }
        }

        Ok(result)
    }

    /// Deserialize and validate store state from JSON
    fn deserialize_and_validate_store_state(
        &self,
        json: &str,
    ) -> Result<StoreState, StoreValidationResult> {
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

    /// Apply store state changes to the chart
    fn apply_store_state_changes(
        &self,
        store_state: &StoreState,
        instance: &mut instance_manager::ChartInstance,
    ) -> GpuChartsResult<Vec<String>> {
        let mut changes_applied = Vec::new();

        // Check if we need to update data (symbol or time range changed)
        let needs_data_update = if let Some(ref current_state) = instance.current_store_state {
            store_state.current_symbol != current_state.current_symbol
                || store_state.start_time != current_state.start_time
                || store_state.end_time != current_state.end_time
        } else {
            true // First time, always need data
        };

        if needs_data_update {
            // Update the data range in the data store
            instance.line_graph.renderer.data_store_mut().set_x_range(
                store_state.start_time as u32,
                store_state.end_time as u32,
            );

            // Note: In a full implementation, we would trigger data fetching here
            // For now, we just update the range
            changes_applied.push(format!(
                "Updated time range: {} to {}",
                store_state.start_time, store_state.end_time
            ));

            if let Some(ref current_state) = instance.current_store_state {
                if store_state.current_symbol != current_state.current_symbol {
                    changes_applied.push(format!(
                        "Changed symbol: {} -> {}",
                        current_state.current_symbol, store_state.current_symbol
                    ));
                }
            } else {
                changes_applied.push(format!("Set symbol: {}", store_state.current_symbol));
            }
        }

        // Check for preset changes
        if let Some(ref current_state) = instance.current_store_state {
            if store_state.preset != current_state.preset {
                changes_applied.push(format!(
                    "Changed preset: {:?} -> {:?}",
                    current_state.preset, store_state.preset
                ));
            }
        } else {
            // First time setup
            changes_applied.push(format!(
                "Set preset: {:?}",
                store_state.preset
            ));
        }

        // If any changes were applied, request a redraw
        if !changes_applied.is_empty() {
            log::info!("Requesting redraw due to state changes");
            // Request a redraw instead of directly spawning render task
            // This avoids RefCell borrow issues across await points
            log::info!("Requesting redraw due to state changes");
        }

        Ok(changes_applied)
    }

    /// Smart state changes application using detailed change detection
    fn apply_smart_state_changes(
        &self,
        store_state: &StoreState,
        change_detection: &StateChangeDetection,
        instance: &mut instance_manager::ChartInstance,
    ) -> GpuChartsResult<Vec<String>> {
        let mut changes_applied = Vec::new();

        // Handle symbol changes
        if change_detection.symbol_changed {
            // Update topic in data store
            instance.line_graph.renderer.data_store_mut().topic =
                Some(store_state.current_symbol.clone());

            changes_applied.push(format!(
                "Symbol updated to: {}",
                store_state.current_symbol
            ));

            // Note: In full implementation, this would trigger async data fetching
            if change_detection.requires_data_fetch {
                changes_applied.push("Data fetch triggered for new symbol".to_string());
            }
        }

        // Handle time range changes
        if change_detection.time_range_changed {
            instance.line_graph.renderer.data_store_mut().set_x_range(
                store_state.start_time as u32,
                store_state.end_time as u32,
            );

            changes_applied.push(format!(
                "Time range updated: {} to {}",
                store_state.start_time, store_state.end_time
            ));

            if change_detection.requires_data_fetch {
                changes_applied.push("Data fetch triggered for new time range".to_string());
            }
        }

        // Handle preset changes
        if change_detection.preset_changed {
            changes_applied.push(format!(
                "Preset updated to: {:?}",
                store_state.preset
            ));

            // Note: In full implementation, this would apply the preset
            if change_detection.requires_render {
                changes_applied.push("Render triggered for preset change".to_string());
            }
        }

        // Note: Data fetching for metrics changes will be handled by the caller
        // to avoid borrow conflicts within this function

        // Trigger rendering if needed
        if change_detection.requires_render && !changes_applied.is_empty() {
            log::info!("Triggering render due to state changes");
            // Request a redraw instead of directly spawning render task
            // This avoids RefCell borrow issues across await points
        }

        // Add smart change detection summary
        changes_applied.extend(change_detection.change_summary.clone());

        Ok(changes_applied)
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

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}
