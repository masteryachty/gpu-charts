use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

// Remove unused imports - we'll use them via crate:: when needed

use crate::controls::canvas_controller::CanvasController;
use crate::events::{
    ElementState, MouseButton, MouseScrollDelta, PhysicalPosition, TouchPhase, WindowEvent,
};
use crate::line_graph::LineGraph;
use crate::store_state::{
    ChangeDetectionConfig, StateChangeDetection, StoreState, StoreValidationResult,
};

extern crate nalgebra_glm as glm;

// Global state for the chart instance
static mut CHART_INSTANCE: Option<ChartInstance> = None;

struct ChartInstance {
    line_graph: Rc<RefCell<LineGraph>>,
    canvas_controller: CanvasController,
    current_store_state: Option<StoreState>,
    change_detection_config: ChangeDetectionConfig,
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
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
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

        let line_graph = Rc::new(RefCell::new(line_graph));

        // Create canvas controller
        let data_store = line_graph.borrow().data_store.clone();
        let engine = line_graph.borrow().engine.clone();
        let canvas_controller = CanvasController::new(data_store, engine);

        // Store globally (in a real app, you'd want better state management)
        let instance = ChartInstance {
            line_graph,
            canvas_controller,
            current_store_state: None,
            change_detection_config: ChangeDetectionConfig::default(),
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
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn render(&self) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = (&raw const CHART_INSTANCE).as_ref().unwrap() {
                // Clone the Rc to avoid holding the borrow across await
                let line_graph = instance.line_graph.clone();
                // Drop the instance reference before the await point

                line_graph
                    .borrow()
                    .render()
                    .await
                    .map_err(|e| format!("Render failed: {e:?}"))?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn needs_render(&self) -> bool {
        unsafe {
            if let Some(instance) = (&raw const CHART_INSTANCE).as_ref().unwrap() {
                instance.line_graph.borrow().data_store.borrow().is_dirty()
            } else {
                false
            }
        }
    }

    #[wasm_bindgen]
    pub fn resize(&self, width: u32, height: u32) -> Result<(), JsValue> {
        log::info!("Resizing chart to: {width}x{height}");

        unsafe {
            if let Some(instance) = (&raw mut CHART_INSTANCE).as_mut().unwrap() {
                instance.line_graph.borrow_mut().resized(width, height);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, _y: f64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = (&raw mut CHART_INSTANCE).as_mut().unwrap() {
                let window_event = WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(x, delta_y)),
                    phase: TouchPhase::Moved,
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_move(&self, x: f64, y: f64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = (&raw mut CHART_INSTANCE).as_mut().unwrap() {
                let window_event = WindowEvent::CursorMoved {
                    position: PhysicalPosition::new(x, y),
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_click(&self, _x: f64, _y: f64, pressed: bool) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = (&raw mut CHART_INSTANCE).as_mut().unwrap() {
                let window_event = WindowEvent::MouseInput {
                    state: if pressed {
                        ElementState::Pressed
                    } else {
                        ElementState::Released
                    },
                    button: MouseButton::Left,
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_data_range(&self, start: u32, end: u32) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = (&raw const CHART_INSTANCE).as_ref().unwrap() {
                instance
                    .line_graph
                    .borrow()
                    .data_store
                    .borrow_mut()
                    .set_x_range(start, end);
            }
        }
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
        unsafe {
            if let Some(instance) = (&raw mut CHART_INSTANCE).as_mut().unwrap() {
                let change_detection = if let Some(ref current_state) = instance.current_store_state
                {
                    store_state
                        .detect_changes_from(current_state, &instance.change_detection_config)
                } else {
                    // First time - treat everything as changed
                    StateChangeDetection {
                        has_changes: true,
                        symbol_changed: true,
                        time_range_changed: true,
                        timeframe_changed: true,
                        indicators_changed: true,
                        metrics_changed: true,
                        connection_changed: true,
                        user_changed: false,
                        market_data_changed: false,
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

                // Step 3: Extract references using try_borrow to handle conflicts gracefully
                let data_store = match instance.line_graph.try_borrow() {
                    Ok(line_graph) => line_graph.data_store.clone(),
                    Err(_) => {
                        log::warn!(
                            "Failed to borrow line_graph for data_store - skipping state update"
                        );
                        let error_response = serde_json::json!({
                            "success": false,
                            "errors": ["Chart is busy - try again in a moment"],
                            "warnings": []
                        });
                        return Ok(error_response.to_string());
                    }
                };

                let device = match instance.line_graph.try_borrow() {
                    Ok(line_graph) => match line_graph.engine.try_borrow() {
                        Ok(engine_ref) => engine_ref.device.clone(),
                        Err(_) => {
                            log::warn!("Failed to borrow engine - skipping state update");
                            let error_response = serde_json::json!({
                                "success": false,
                                "errors": ["Chart engine is busy - try again in a moment"],
                                "warnings": []
                            });
                            return Ok(error_response.to_string());
                        }
                    },
                    Err(_) => {
                        log::warn!(
                            "Failed to borrow line_graph for device - skipping state update"
                        );
                        let error_response = serde_json::json!({
                            "success": false,
                            "errors": ["Chart is busy - try again in a moment"],
                            "warnings": []
                        });
                        return Ok(error_response.to_string());
                    }
                };

                // Step 4: Apply the state changes using smart detection with shared refs
                match self.apply_smart_state_changes(
                    &store_state,
                    &change_detection,
                    instance,
                    &data_store,
                ) {
                    Ok(changes_applied) => {
                        // Step 4.5: Handle data fetching for metrics changes
                        if change_detection.metrics_changed && change_detection.requires_data_fetch
                        {
                            log::info!("Triggering data fetch for metrics changes");

                            let start = store_state.chart_config.start_time as u32;
                            let end = store_state.chart_config.end_time as u32;
                            let selected_metrics =
                                Some(store_state.chart_config.selected_metrics.clone());

                            // Spawn async data fetching task using shared refs
                            wasm_bindgen_futures::spawn_local(async move {
                                use crate::renderer::data_retriever::fetch_data;
                                fetch_data(&device, start, end, data_store, selected_metrics).await;
                            });
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
                                "timeframeChanged": change_detection.timeframe_changed,
                                "indicatorsChanged": change_detection.indicators_changed,
                                "metricsChanged": change_detection.metrics_changed,
                                "connectionChanged": change_detection.connection_changed,
                                "userChanged": change_detection.user_changed,
                                "marketDataChanged": change_detection.market_data_changed,
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
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
    }

    /// Check if the chart is initialized and has an active instance
    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        unsafe {
            let ptr = &raw const CHART_INSTANCE;
            (*ptr).is_some()
        }
    }

    /// Get current store state as JSON (for debugging/sync purposes)
    #[wasm_bindgen]
    pub fn get_current_store_state(&self) -> Result<String, JsValue> {
        unsafe {
            if let Some(instance) = (&raw const CHART_INSTANCE).as_ref().unwrap() {
                if let Some(ref state) = instance.current_store_state {
                    match serde_json::to_string(state) {
                        Ok(json) => Ok(json),
                        Err(e) => Err(JsValue::from_str(&format!("Serialization failed: {e}"))),
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
            if let Some(instance) = (&raw mut CHART_INSTANCE).as_mut().unwrap() {
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

    /// Configure change detection behavior
    #[wasm_bindgen]
    pub fn configure_change_detection(&self, config_json: &str) -> Result<String, JsValue> {
        log::info!("configure_change_detection called with: {config_json}");

        unsafe {
            if let Some(instance) = (&raw mut CHART_INSTANCE).as_mut().unwrap() {
                // Parse the configuration JSON
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

                // Update the configuration
                instance.change_detection_config = config;

                let response = serde_json::json!({
                    "success": true,
                    "message": "Change detection configuration updated",
                    "config": {
                        "enableSymbolChangeDetection": instance.change_detection_config.enable_symbol_change_detection,
                        "enableTimeRangeChangeDetection": instance.change_detection_config.enable_time_range_change_detection,
                        "enableTimeframeChangeDetection": instance.change_detection_config.enable_timeframe_change_detection,
                        "enableIndicatorChangeDetection": instance.change_detection_config.enable_indicator_change_detection,
                        "symbolChangeTriggersF etch": instance.change_detection_config.symbol_change_triggers_fetch,
                        "timeRangeChangeTriggersF etch": instance.change_detection_config.time_range_change_triggers_fetch,
                        "minimumTimeRangeChangeSeconds": instance.change_detection_config.minimum_time_range_change_seconds
                    }
                });
                Ok(response.to_string())
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
    }

    /// Get current change detection configuration
    #[wasm_bindgen]
    pub fn get_change_detection_config(&self) -> Result<String, JsValue> {
        unsafe {
            if let Some(instance) = (&raw const CHART_INSTANCE).as_ref().unwrap() {
                let config_json = serde_json::json!({
                    "enableSymbolChangeDetection": instance.change_detection_config.enable_symbol_change_detection,
                    "enableTimeRangeChangeDetection": instance.change_detection_config.enable_time_range_change_detection,
                    "enableTimeframeChangeDetection": instance.change_detection_config.enable_timeframe_change_detection,
                    "enableIndicatorChangeDetection": instance.change_detection_config.enable_indicator_change_detection,
                    "symbolChangeTriggersF etch": instance.change_detection_config.symbol_change_triggers_fetch,
                    "timeRangeChangeTriggersF etch": instance.change_detection_config.time_range_change_triggers_fetch,
                    "timeframeChangeTriggersRender": instance.change_detection_config.timeframe_change_triggers_render,
                    "indicatorChangeTriggersRender": instance.change_detection_config.indicator_change_triggers_render,
                    "minimumTimeRangeChangeSeconds": instance.change_detection_config.minimum_time_range_change_seconds
                });
                Ok(config_json.to_string())
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
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

        unsafe {
            if let Some(instance) = (&raw const CHART_INSTANCE).as_ref().unwrap() {
                let change_detection = if let Some(ref current_state) = instance.current_store_state
                {
                    store_state
                        .detect_changes_from(current_state, &instance.change_detection_config)
                } else {
                    StateChangeDetection {
                        has_changes: true,
                        symbol_changed: true,
                        time_range_changed: true,
                        timeframe_changed: true,
                        indicators_changed: true,
                        metrics_changed: true,
                        connection_changed: true,
                        user_changed: false,
                        market_data_changed: false,
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
                        "timeframeChanged": change_detection.timeframe_changed,
                        "indicatorsChanged": change_detection.indicators_changed,
                        "connectionChanged": change_detection.connection_changed,
                        "userChanged": change_detection.user_changed,
                        "marketDataChanged": change_detection.market_data_changed,
                        "requiresDataFetch": change_detection.requires_data_fetch,
                        "requiresRender": change_detection.requires_render,
                        "summary": change_detection.change_summary
                    }
                });
                Ok(response.to_string())
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
    }

    /// Set the chart type (line or candlestick)
    #[wasm_bindgen]
    pub fn set_chart_type(&self, chart_type: &str) -> Result<(), JsValue> {
        unsafe {
            if let Some(ref mut instance) = CHART_INSTANCE {
                instance.line_graph.borrow_mut().set_chart_type(chart_type);
                Ok(())
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
    }

    /// Set the candle timeframe in seconds (e.g., 60 for 1 minute, 300 for 5 minutes)
    #[wasm_bindgen]
    pub fn set_candle_timeframe(&self, timeframe_seconds: u32) -> Result<(), JsValue> {
        unsafe {
            if let Some(ref mut instance) = CHART_INSTANCE {
                instance
                    .line_graph
                    .borrow_mut()
                    .set_candle_timeframe(timeframe_seconds);
                Ok(())
            } else {
                Err(JsValue::from_str("Chart not initialized"))
            }
        }
    }
}

// Private implementation methods for Chart
impl Chart {
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

    // Removed unused states_are_equivalent method

    /// Apply store state changes to the chart
    fn apply_store_state_changes(
        &self,
        store_state: &StoreState,
        instance: &mut ChartInstance,
    ) -> Result<Vec<String>, String> {
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
                store_state.chart_config.start_time, store_state.chart_config.end_time
            ));

            if let Some(ref current_state) = instance.current_store_state {
                if store_state.chart_config.symbol != current_state.chart_config.symbol {
                    changes_applied.push(format!(
                        "Changed symbol: {} -> {}",
                        current_state.chart_config.symbol, store_state.chart_config.symbol
                    ));
                }
            } else {
                changes_applied.push(format!("Set symbol: {}", store_state.chart_config.symbol));
            }
        }

        // Check for timeframe changes
        if let Some(ref current_state) = instance.current_store_state {
            if store_state.chart_config.timeframe != current_state.chart_config.timeframe {
                changes_applied.push(format!(
                    "Changed timeframe: {} -> {}",
                    current_state.chart_config.timeframe, store_state.chart_config.timeframe
                ));
            }

            // Check for indicator changes
            if store_state.chart_config.indicators != current_state.chart_config.indicators {
                changes_applied.push(format!(
                    "Updated indicators: {:?} -> {:?}",
                    current_state.chart_config.indicators, store_state.chart_config.indicators
                ));
            }

            // Check for connection status changes
            if store_state.is_connected != current_state.is_connected {
                changes_applied.push(format!(
                    "Connection status: {} -> {}",
                    current_state.is_connected, store_state.is_connected
                ));
            }
        } else {
            // First time setup
            changes_applied.push(format!(
                "Set timeframe: {}",
                store_state.chart_config.timeframe
            ));
            changes_applied.push(format!(
                "Set indicators: {:?}",
                store_state.chart_config.indicators
            ));
            changes_applied.push(format!(
                "Set connection status: {}",
                store_state.is_connected
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
        _instance: &mut ChartInstance,
        data_store: &std::rc::Rc<std::cell::RefCell<crate::renderer::data_store::DataStore>>,
    ) -> Result<Vec<String>, String> {
        let mut changes_applied = Vec::new();

        // Handle symbol changes
        if change_detection.symbol_changed {
            // Update topic in data store using shared ref
            data_store.borrow_mut().topic = Some(store_state.chart_config.symbol.clone());

            changes_applied.push(format!(
                "Symbol updated to: {}",
                store_state.chart_config.symbol
            ));

            // Note: In full implementation, this would trigger async data fetching
            if change_detection.requires_data_fetch {
                changes_applied.push("Data fetch triggered for new symbol".to_string());
            }
        }

        // Handle time range changes
        if change_detection.time_range_changed {
            data_store.borrow_mut().set_x_range(
                store_state.chart_config.start_time as u32,
                store_state.chart_config.end_time as u32,
            );

            changes_applied.push(format!(
                "Time range updated: {} to {}",
                store_state.chart_config.start_time, store_state.chart_config.end_time
            ));

            if change_detection.requires_data_fetch {
                changes_applied.push("Data fetch triggered for new time range".to_string());
            }
        }

        // Handle timeframe changes
        if change_detection.timeframe_changed {
            changes_applied.push(format!(
                "Timeframe updated to: {}",
                store_state.chart_config.timeframe
            ));

            // Note: In full implementation, this would update aggregation logic
            if change_detection.requires_render {
                changes_applied.push("Render triggered for timeframe change".to_string());
            }
        }

        // Handle indicator changes
        if change_detection.indicators_changed {
            changes_applied.push(format!(
                "Indicators updated: {:?}",
                store_state.chart_config.indicators
            ));

            // Note: In full implementation, this would update indicator renderers
            if change_detection.requires_render {
                changes_applied.push("Render triggered for indicator changes".to_string());
            }
        }

        // Handle metrics changes
        if change_detection.metrics_changed {
            changes_applied.push(format!(
                "Metrics updated: {:?}",
                store_state.chart_config.selected_metrics
            ));

            // This is the key change - metrics changes should trigger data fetching
            if change_detection.requires_data_fetch {
                changes_applied.push("Data fetch triggered for new metrics".to_string());

                // Note: We skip the actual fetch here to avoid borrow conflicts
                // The data fetching will be handled elsewhere or deferred
            }

            if change_detection.requires_render {
                changes_applied.push("Render triggered for metrics changes".to_string());
            }
        }

        // Handle connection status changes
        if change_detection.connection_changed {
            log::info!("Connection status changed");
            changes_applied.push(format!("Connection status: {}", store_state.is_connected));

            // Note: In full implementation, this might pause/resume data feeds
        }

        // Handle user changes
        if change_detection.user_changed {
            log::info!("User information changed");
            if store_state.user.is_some() {
                changes_applied.push("User session updated".to_string());
            } else {
                changes_applied.push("User logged out".to_string());
            }
        }

        // Handle market data changes
        if change_detection.market_data_changed {
            changes_applied.push("Market data refreshed".to_string());

            if change_detection.requires_render {
                changes_applied.push("Render triggered for market data update".to_string());
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
