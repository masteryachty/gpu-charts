use chrono::DateTime;
use config_system::PresetManager;
use data_manager::{DataManager, DataStore};
use renderer::{MultiRenderer, Renderer};
use std::rc::Rc;
use uuid::Uuid;

use crate::{
    controls::canvas_controller::CanvasController,
    render_loop::{RenderLoopController, RenderLoopState, StateTransitionTrigger},
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
    render_loop: RenderLoopController,
    instance_id: Uuid,
}

impl ChartEngine {
    /// Get list of column names that should be excluded from Y bounds calculation
    /// based on the active preset's additional_data_columns
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
        log::info!("Initializing chart with canvas: {canvas_id}, size: {width}x{height}");
        let window = web_sys::window().ok_or_else(|| Error::new(&format!("No Window")))?;
        let document = window
            .document()
            .ok_or_else(|| Error::new(&format!("No document")))?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or_else(|| Error::new(&format!("Canvas not found")))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| Error::new(&format!("Element is not a canvas")))?;

        // Set canvas size
        canvas.set_width(width);
        canvas.set_height(height);

        // let _params = get_query_params();

        // // // Handle missing query parameters gracefully (for React integration)
        // let topic = params
        //     .get("topic")
        //     .unwrap_or(&"default_topic".to_string())
        //     .clone();
        // let start = params
        //     .get("start")
        //     .and_then(|s| s.parse().ok())
        //     .unwrap_or_else(|| {
        //         // Default to last hour if no start time provided
        //         chrono::Utc::now().timestamp() - 3600
        //     });
        // let end = params
        //     .get("end")
        //     .and_then(|s| s.parse().ok())
        //     .unwrap_or_else(|| {
        //         // Default to current time if no end time provided
        //         chrono::Utc::now().timestamp()
        //     });

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

        // Create DataManager with modular approach
        let data_manager = DataManager::new(
            device.clone(),
            queue.clone(),
            "https://api.rednax.io".to_string(),
        );

        // Log initial state before moving data_store
        log::info!("Initial DataStore state:");
        log::info!(
            "  - X range: {} to {}",
            data_store.start_x,
            data_store.end_x
        );
        log::info!(
            "  - Y bounds: min={:?}, max={:?}",
            data_store.gpu_min_y,
            data_store.gpu_max_y
        );

        // Create Renderer with modular approach
        let renderer = Renderer::new(canvas, device.clone(), queue.clone(), data_store)
            .await
            .map_err(|e| Error::new(&format!("Failed to create renderer: {e:?}")))?;

        // Skip initial data fetch - wait for user to select a preset
        log::info!("Skipping initial data fetch - waiting for preset selection");
        // Data will be fetched when user selects a preset via fetch_preset_data

        log::info!("LineGraph initialization completed - no data loaded yet");

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

        log::info!("ChartEngine initialized with minimal renderer configuration - waiting for preset selection");

        // Create render loop controller
        let render_loop = RenderLoopController::new();
        let instance_id = Uuid::new_v4();

        // Add state change listener for debugging
        render_loop.add_state_listener(Rc::new(|old, new| {
            log::info!("ChartEngine state changed: {:?} -> {:?}", old, new);
        }));

        // Create the ChartEngine instance
        Ok(Self {
            renderer,
            data_manager,
            canvas_controller,
            preset_manager: PresetManager::new(),
            multi_renderer: Some(multi_renderer),
            render_loop,
            instance_id,
        })
    }

    pub async fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        log::info!("RENDER !!!!!!!!");

        // Always use multi-renderer (we ensure it exists in new())
        if let Some(ref mut multi_renderer) = self.multi_renderer {
            self.renderer
                .render(multi_renderer)
                .await
                .map_err(|e| match e {
                    shared_types::GpuChartsError::Surface { .. } => wgpu::SurfaceError::Outdated,
                    _ => wgpu::SurfaceError::Outdated,
                })
        } else {
            // This should never happen since we create a default multi-renderer in new()
            log::error!("No multi-renderer available!");
            Err(wgpu::SurfaceError::Outdated)
        }
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);

        // Also resize the multi-renderer if present
        if let Some(ref mut multi_renderer) = self.multi_renderer {
            multi_renderer.resize(width, height);
        }

        // Trigger resize state transition
        // Resize only needs re-render, no preprocessing needed because:
        // - Data doesn't change
        // - GPU buffers don't change
        // - Only the viewport/surface size changes
        self.on_resized(false);
    }

    pub fn set_preset_and_symbol(
        &mut self,
        preset_name: Option<String>,
        symbol_name: Option<String>,
    ) {
        if let Some(name) = preset_name {
            log::info!("[ChartEngine] Looking for preset: {}", name);
            let preset = self.preset_manager.find_preset(&name).cloned();
            if let Some(preset) = preset {
                log::info!("[ChartEngine] Found preset: {}", name);

                // Update renderer with preset
                self.renderer
                    .set_preset_and_symbol(Some(&preset), symbol_name);

                // Rebuild multi-renderer based on preset configuration
                self.rebuild_multi_renderer_for_preset(&preset);

                // Trigger data config changed when preset is set
                self.on_data_config_changed();
            } else {
                log::warn!("[ChartEngine] Preset not found: {}", name);
                self.renderer.set_preset_and_symbol(None, symbol_name);
            }
        } else {
            log::warn!("[ChartEngine] No preset name provided");
            self.renderer.set_preset_and_symbol(None, symbol_name);
        }
    }

    // async fn startRenderLoop(&mut self) {
    //     let result self.render().await;
    // }

    /// Start the render loop
    pub fn start_render_loop(&mut self) -> Result<(), Error> {
        self.render_loop
            .trigger_transition(StateTransitionTrigger::Start, self.instance_id);
        Ok(())
    }

    /// Stop the render loop
    pub fn stop_render_loop(&mut self) -> Result<(), Error> {
        self.render_loop
            .trigger_transition(StateTransitionTrigger::Stop, self.instance_id);
        Ok(())
    }

    /// Called when new data is received
    pub fn on_data_received(&mut self) {
        self.render_loop
            .trigger_transition(StateTransitionTrigger::DataReceived, self.instance_id);
    }

    /// Called when view changes (pan/zoom) - render only
    pub fn on_view_changed(&mut self) {
        self.render_loop
            .trigger_transition(StateTransitionTrigger::ViewChanged, self.instance_id);
    }

    /// Called when visual settings change - render only
    pub fn on_visual_settings_changed(&mut self) {
        self.render_loop.trigger_transition(
            StateTransitionTrigger::VisualSettingsChanged,
            self.instance_id,
        );
    }

    /// Called when metric visibility changes - render only
    pub fn on_metric_visibility_changed(&mut self) {
        self.render_loop.trigger_transition(
            StateTransitionTrigger::MetricVisibilityChanged,
            self.instance_id,
        );
    }

    /// Called when data configuration changes - requires preprocessing
    pub fn on_data_config_changed(&mut self) {
        self.render_loop
            .trigger_transition(StateTransitionTrigger::DataConfigChanged, self.instance_id);
    }

    /// Called when resized
    pub fn on_resized(&mut self, requires_preprocessing: bool) {
        self.render_loop.trigger_transition(
            StateTransitionTrigger::Resized {
                requires_preprocessing,
            },
            self.instance_id,
        );
    }

    /// Get current render loop state
    pub fn get_render_state(&self) -> RenderLoopState {
        self.render_loop.get_state()
    }

    /// Set instance ID (used by instance manager)
    pub fn set_instance_id(&mut self, id: Uuid) {
        self.instance_id = id;
    }

    /// Rebuild the multi-renderer based on preset configuration
    fn rebuild_multi_renderer_for_preset(&mut self, preset: &config_system::ChartPreset) {
        log::info!(
            "[ChartEngine] Rebuilding multi-renderer for preset: {}",
            preset.name
        );

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
                    log::debug!(
                        "[ChartEngine] Adding PlotRenderer for: {}",
                        chart_type.label
                    );

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
                    log::debug!(
                        "[ChartEngine] Adding TriangleRenderer for: {}",
                        chart_type.label
                    );

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
                    log::debug!(
                        "[ChartEngine] Adding CandlestickRenderer for: {}",
                        chart_type.label
                    );
                    builder = builder.add_candlestick_renderer();
                }
                config_system::RenderType::Bar => {
                    log::warn!(
                        "[ChartEngine] Bar renderer not yet implemented for: {}",
                        chart_type.label
                    );
                    // TODO: Implement bar renderer
                }
                config_system::RenderType::Area => {
                    log::warn!(
                        "[ChartEngine] Area renderer not yet implemented for: {}",
                        chart_type.label
                    );
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

        log::info!("[ChartEngine] Multi-renderer rebuilt with renderers based on preset");
    }
}

pub fn unix_timestamp_to_string(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    datetime.unwrap().to_rfc3339()
}
