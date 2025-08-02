use chrono::DateTime;
use config_system::PresetManager;
use data_manager::{DataManager, DataStore};
use renderer::{MultiRenderer, Renderer};
use shared_types::{StateData, StateDiff, StateSection, UnifiedState};
use std::rc::Rc;
use uuid::Uuid;

use crate::{
    controls::canvas_controller::CanvasController,
    immediate_update::{ImmediateUpdater, UpdateAction, UpdateEvent},
};

use js_sys::Error;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

pub struct ChartEngine {
    pub renderer: Renderer,
    pub canvas_controller: CanvasController,
    pub data_manager: DataManager,
    pub preset_manager: PresetManager,
    pub multi_renderer: Option<MultiRenderer>,
    updater: ImmediateUpdater,
    instance_id: Uuid,
    unified_state: UnifiedState,
}

impl ChartEngine {
    // Get list of column names that should be excluded from Y bounds calculation
    // based on the active preset's additional_data_columns
    // pub fn get_excluded_columns_from_preset(&self, preset_name: &str) -> Vec<String> {
    //     let mut excluded_columns = Vec::new();

    //     log::info!(
    //         "[LineGraph] Getting excluded columns for preset '{}'",
    //         preset_name
    //     );

    //     if let Some(preset) = self.preset_manager.get_preset(preset_name) {
    //         log::info!(
    //             "[LineGraph] Found preset with {} chart types",
    //             preset.chart_types.len()
    //         );

    //         for (idx, chart_type) in preset.chart_types.iter().enumerate() {
    //             log::info!(
    //                 "[LineGraph]   Chart type[{}]: '{}' - visible={}",
    //                 idx,
    //                 chart_type.label,
    //                 chart_type.visible
    //             );

    //             if let Some(additional_cols) = &chart_type.additional_data_columns {
    //                 log::info!(
    //                     "[LineGraph]     Has {} additional columns",
    //                     additional_cols.len()
    //                 );

    //                 for (_data_type, column_name) in additional_cols {
    //                     log::info!("[LineGraph]     Adding excluded column: '{}'", column_name);
    //                     if !excluded_columns.contains(column_name) {
    //                         excluded_columns.push(column_name.clone());
    //                     }
    //                 }
    //             } else {
    //                 log::info!("[LineGraph]     No additional columns");
    //             }
    //         }
    //     } else {
    //         log::warn!("[LineGraph] Preset '{}' not found!", preset_name);
    //     }

    //     // Always exclude "side" and "volume" as defaults
    //     for default_exclude in ["side", "volume"] {
    //         if !excluded_columns.contains(&default_exclude.to_string()) {
    //             log::info!(
    //                 "[LineGraph] Adding default excluded column: '{}'",
    //                 default_exclude
    //             );
    //             excluded_columns.push(default_exclude.to_string());
    //         }
    //     }

    //     log::info!(
    //         "[LineGraph] Final excluded columns from preset '{}': {:?}",
    //         preset_name,
    //         excluded_columns
    //     );
    //     excluded_columns
    // }

    pub async fn new(
        width: u32,
        height: u32,
        canvas_id: &str,
        start_x: u32,
        end_x: u32,
    ) -> Result<ChartEngine, Error> {
        let window = web_sys::window().ok_or_else(|| Error::new("No Window"))?;
        let document = window.document().ok_or_else(|| Error::new("No document"))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| Error::new("Canvas not found"))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| Error::new("Element is not a canvas"))?;

        // Set canvas size
        canvas.set_width(width);
        canvas.set_height(height);

        // Create canvas controller
        let canvas_controller = CanvasController::new();

        // Create DataStore
        let data_store = DataStore::new(width, height, start_x, end_x);
        // data_store.topic = Some(topic.clone());

        // Create WebGPU instance and get device/queue
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| Error::new(&format!("Failed to create surface: {e}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or_else(|| Error::new("Failed to get adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("Device"),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| Error::new(&format!("Failed to request device: {e}")))?;

        let device = Rc::new(device);
        let queue = Rc::new(queue);

        let api_base_url = "https://api.rednax.io".to_string();

        // Create DataManager with modular approach
        let data_manager = DataManager::new(device.clone(), queue.clone(), api_base_url);

        // Log initial state before moving data_store

        // Create Renderer with modular approach
        let renderer = Renderer::new(canvas, device.clone(), queue.clone(), data_store)
            .await
            .map_err(|e| Error::new(&format!("Failed to create renderer: {e:?}")))?;

        // Skip initial data fetch - wait for user to select a preset
        // Data will be fetched when user selects a preset via fetch_preset_data

        // Create a minimal multi-renderer with just axes
        // Specific chart renderers will be added when a preset is selected
        let multi_renderer = renderer
            .create_multi_renderer()
            .with_render_order(renderer::RenderOrder::BackgroundToForeground)
            .add_x_axis_renderer(
                renderer.data_store().screen_size.width,
                renderer.data_store().screen_size.height,
            )
            .add_y_axis_renderer(
                renderer.data_store().screen_size.width,
                renderer.data_store().screen_size.height,
            )
            .build();

        // Create immediate updater
        let updater = ImmediateUpdater::new();
        let instance_id = Uuid::new_v4();

        // Initialize unified state with initial values
        let mut unified_state = UnifiedState::new();

        // Initialize with current viewport dimensions
        unified_state.update_section(
            StateSection::View,
            StateData::View {
                zoom_level: 1.0,
                pan_offset: 0.0,
                viewport_width: width,
                viewport_height: height,
            },
        );

        // Initialize with default data config
        unified_state.update_section(
            StateSection::Data,
            StateData::Data {
                symbol: "BTC-USD".to_string(),
                start_time: start_x as i64,
                end_time: end_x as i64,
                timeframe: 60,
                data_version: 0,
            },
        );

        // Create the ChartEngine instance
        Ok(Self {
            renderer,
            data_manager,
            canvas_controller,
            preset_manager: PresetManager::new(),
            multi_renderer: Some(multi_renderer),
            updater,
            instance_id,
            unified_state,
        })
    }

    pub fn needs_render(&self) -> bool {
        // Check if renderer needs to render
        self.renderer.needs_render()
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Always use multi-renderer (we ensure it exists in new())
        if let Some(ref mut multi_renderer) = self.multi_renderer {
            self.renderer.render(multi_renderer).map_err(|e| match e {
                shared_types::GpuChartsError::Surface { .. } => wgpu::SurfaceError::Outdated,
                _ => wgpu::SurfaceError::Outdated,
            })
        } else {
            // This should never happen since we create a default multi-renderer in new()
            Err(wgpu::SurfaceError::Outdated)
        }
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);

        // Also resize the multi-renderer if present
        if let Some(ref mut multi_renderer) = self.multi_renderer {
            multi_renderer.resize(width, height);
        }

        // Update unified state with new viewport size
        let diff = self.unified_state.update_section(
            StateSection::View,
            StateData::View {
                zoom_level: 1.0, // TODO: Get actual zoom level from renderer
                pan_offset: 0.0, // TODO: Get actual pan offset from renderer
                viewport_width: width,
                viewport_height: height,
            },
        );

        // Pass state diff to renderer for incremental updates
        self.renderer.process_state_change(&diff);

        // Use state diff to determine required actions
        let actions = diff.get_required_actions();

        if actions.needs_pipeline_rebuild {
            self.on_resized(width, height); // Resizing requires pipeline rebuild
        } else if actions.needs_render {
            self.on_view_changed();
        }
    }

    pub fn set_preset_and_symbol(
        &mut self,
        preset_name: Option<String>,
        symbol_name: Option<String>,
    ) {
        let mut state_updates = Vec::new();

        // Update config state if preset changed
        if let Some(name) = preset_name.clone() {
            let preset = self.preset_manager.find_preset(&name).cloned();
            if let Some(preset) = preset {
                // Update renderer with preset
                self.renderer
                    .set_preset_and_symbol(Some(&preset), symbol_name.clone());

                // Rebuild multi-renderer based on preset configuration
                self.rebuild_multi_renderer_for_preset(&preset);

                // Update config state
                state_updates.push((
                    StateSection::Config,
                    StateData::Config {
                        preset_name: name,
                        quality_level: shared_types::QualityLevel::Medium, // TODO: Get from preset
                        chart_type: preset
                            .chart_types
                            .first()
                            .map(|ct| ct.render_type.to_string())
                            .unwrap_or_else(|| "line".to_string()),
                        show_grid: true,
                    },
                ));
            } else {
                self.renderer
                    .set_preset_and_symbol(None, symbol_name.clone());
            }
        } else {
            self.renderer
                .set_preset_and_symbol(None, symbol_name.clone());
        }

        // Update data state if symbol changed
        if let Some(symbol) = symbol_name {
            // Get current data state and update symbol
            if let Some(section_state) = self.unified_state.get_section(StateSection::Data) {
                if let StateData::Data {
                    start_time,
                    end_time,
                    timeframe,
                    data_version,
                    ..
                } = &section_state.data
                {
                    state_updates.push((
                        StateSection::Data,
                        StateData::Data {
                            symbol,
                            start_time: *start_time,
                            end_time: *end_time,
                            timeframe: *timeframe,
                            data_version: data_version + 1,
                        },
                    ));
                }
            }
        }

        // Apply all state updates
        if !state_updates.is_empty() {
            let diff = self.unified_state.batch_update(state_updates);

            // Pass state diff to renderer for incremental updates
            self.renderer.process_state_change(&diff);

            let actions = diff.get_required_actions();

            // Trigger appropriate actions based on state changes
            if actions.needs_data_fetch || actions.needs_pipeline_rebuild {
                self.on_data_config_changed();
            } else if actions.needs_render {
                self.on_visual_settings_changed();
            }
        }
    }

    // async fn startRenderLoop(&mut self) {
    //     let result self.render().await;
    // }

    /// Start the render loop (now just marks as ready)
    pub fn start_render_loop(&mut self) -> Result<(), Error> {
        self.updater.set_ready();
        Ok(())
    }

    /// Called when new data is received
    pub fn on_data_received(&mut self) {
        let action = self.updater.process_update(UpdateEvent::DataChanged);
        self.handle_update_action(action);
    }

    /// Called when view changes (pan/zoom) - render only
    pub fn on_view_changed(&mut self) {
        let action = self.updater.process_update(UpdateEvent::ViewChanged {
            zoom: true,
            pan: true,
        });
        self.handle_update_action(action);
    }

    /// Called when visual settings change - render only
    pub fn on_visual_settings_changed(&mut self) {
        let action = self.updater.process_update(UpdateEvent::ViewChanged {
            zoom: false,
            pan: false,
        });
        self.handle_update_action(action);
    }

    /// Called when metric visibility changes - render only
    pub fn on_metric_visibility_changed(&mut self) {
        let action = self
            .updater
            .process_update(UpdateEvent::MetricVisibilityChanged);
        self.handle_update_action(action);
    }

    /// Called when data configuration changes - requires preprocessing
    pub fn on_data_config_changed(&mut self) {
        let action = self.updater.process_update(UpdateEvent::ConfigChanged);
        self.handle_update_action(action);
    }

    /// Called when resized
    pub fn on_resized(&mut self, width: u32, height: u32) {
        let action = self
            .updater
            .process_update(UpdateEvent::Resized(width, height));
        self.handle_update_action(action);
    }

    /// Get current render state
    pub fn get_render_state(&self) -> crate::immediate_update::RenderState {
        self.updater.get_state()
    }

    /// Handle update action from the immediate updater
    fn handle_update_action(&mut self, action: UpdateAction) {
        match action {
            UpdateAction::RenderOnly => {
                // Mark data as dirty to trigger render
                self.renderer.data_store_mut().mark_dirty();
            }
            UpdateAction::FetchAndRender => {
                // Clear bounds for recalculation
                {
                    let data_store = self.renderer.data_store_mut();
                    data_store.min_max_buffer = None;
                    data_store.gpu_min_y = None;
                    data_store.gpu_max_y = None;
                    data_store.mark_dirty();
                }

                // In immediate mode, we don't spawn async tasks here
                // The actual data fetch happens in apply_preset_and_symbol
            }
            UpdateAction::RebuildAndRender => {
                // Rebuild multi-renderer if we have a preset
                let preset_clone = self.renderer.data_store().preset.clone();
                if let Some(preset) = preset_clone {
                    self.rebuild_multi_renderer_for_preset(&preset);
                }
                self.renderer.data_store_mut().mark_dirty();
            }
        }
    }

    /// Set instance ID (used by instance manager)
    pub fn set_instance_id(&mut self, id: Uuid) {
        self.instance_id = id;
    }

    /// Update unified state from React store state
    pub fn update_from_react_state(&mut self, store_state: &serde_json::Value) -> StateDiff {
        let mut state_updates = Vec::new();

        // Extract data state from React
        if let Some(chart_config) = store_state.get("ChartStateConfig") {
            if let Ok(symbol) = chart_config
                .get("symbol")
                .and_then(|v| v.as_str())
                .ok_or("missing symbol")
            {
                let start_time = chart_config
                    .get("startTime")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let end_time = chart_config
                    .get("endTime")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let timeframe = chart_config
                    .get("timeframe")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(60) as u32;

                state_updates.push((
                    StateSection::Data,
                    StateData::Data {
                        symbol: symbol.to_string(),
                        start_time,
                        end_time,
                        timeframe,
                        data_version: self.unified_state.generation + 1,
                    },
                ));
            }
        }

        // Extract UI state from React
        if let Some(metrics) = store_state.get("selectedMetrics") {
            if let Some(metrics_array) = metrics.as_array() {
                let visible_metrics: Vec<String> = metrics_array
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();

                state_updates.push((
                    StateSection::UI,
                    StateData::UI {
                        visible_metrics,
                        theme: "dark".to_string(),
                        layout_mode: "default".to_string(),
                    },
                ));
            }
        }

        // Apply batch update and return diff
        if !state_updates.is_empty() {
            self.unified_state.batch_update(state_updates)
        } else {
            // Return empty diff if no updates
            StateDiff {
                changed_sections: Default::default(),
                generation_delta: 0,
                section_changes: Default::default(),
            }
        }
    }

    /// Get current unified state for React
    pub fn get_unified_state(&self) -> &UnifiedState {
        &self.unified_state
    }

    /// Get state changes since a given generation
    pub fn get_state_changes_since(&self, generation: u64) -> Vec<StateSection> {
        self.unified_state.get_changes_since(generation)
    }

    /// Rebuild the multi-renderer based on preset configuration
    fn rebuild_multi_renderer_for_preset(&mut self, preset: &config_system::ChartPreset) {
        // Get current screen dimensions
        let width = self.renderer.data_store().screen_size.width;
        let height = self.renderer.data_store().screen_size.height;

        // Create new multi-renderer
        let mut builder = self
            .renderer
            .create_multi_renderer()
            .with_render_order(renderer::RenderOrder::BackgroundToForeground);

        // Add renderers based on preset chart types
        for chart_type in &preset.chart_types {
            if !chart_type.visible {
                continue;
            }

            match chart_type.render_type {
                config_system::RenderType::Line => {
                    // Create a configurable plot renderer with specific data columns
                    let plot_renderer = renderer::ConfigurablePlotRenderer::new(
                        self.renderer.device.clone(),
                        self.renderer.queue.clone(),
                        self.renderer.config.format,
                        chart_type.label.clone(),
                        chart_type.data_columns.clone(),
                    );
                    builder = builder.add_renderer(Box::new(plot_renderer));
                }
                config_system::RenderType::Triangle => {
                    let triangle_renderer = renderer::charts::TriangleRenderer::new(
                        self.renderer.device.clone(),
                        self.renderer.queue.clone(),
                        self.renderer.config.format,
                    );

                    // The TriangleRenderer automatically finds data groups with "price" and "side" metrics
                    // So we don't need to set a specific data group name
                    builder = builder.add_renderer(Box::new(triangle_renderer));
                }
                config_system::RenderType::Candlestick => {
                    builder = builder.add_candlestick_renderer();
                }
                config_system::RenderType::Bar => {
                    // TODO: Implement bar renderer
                }
                config_system::RenderType::Area => {
                    // TODO: Implement area renderer
                }
            }
        }

        // Always add axes
        builder = builder
            .add_x_axis_renderer(width, height)
            .add_y_axis_renderer(width, height);

        // Build and replace the multi-renderer
        let new_multi_renderer = builder.build();
        self.multi_renderer = Some(new_multi_renderer);
    }
}

pub fn unix_timestamp_to_string(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    datetime.unwrap().to_rfc3339()
}
