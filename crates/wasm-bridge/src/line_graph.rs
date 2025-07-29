use chrono::DateTime;
use config_system::PresetManager;
use data_manager::{ChartType, DataManager, DataStore};
use renderer::{MultiRenderer, Renderer};
use shared_types::DataHandle;
use std::rc::Rc;

use crate::{controls::canvas_controller::CanvasController, wrappers::js::get_query_params};

use js_sys::Error;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

pub struct ChartEngine {
    pub renderer: Renderer,
    pub canvas_controller: CanvasController,
    pub data_manager: DataManager,
    pub preset_manager: PresetManager,
    pub multi_renderer: Option<MultiRenderer>,
}

impl ChartEngine {
    /// Get list of column names that should be excluded from Y bounds calculation
    /// based on the active preset's additional_data_columns
    pub fn get_excluded_columns_from_preset(&self, preset_name: &str) -> Vec<String> {
        let mut excluded_columns = Vec::new();

        log::info!(
            "[LineGraph] Getting excluded columns for preset '{}'",
            preset_name
        );

        if let Some(preset) = self.preset_manager.get_preset(preset_name) {
            log::info!(
                "[LineGraph] Found preset with {} chart types",
                preset.chart_types.len()
            );

            for (idx, chart_type) in preset.chart_types.iter().enumerate() {
                log::info!(
                    "[LineGraph]   Chart type[{}]: '{}' - visible={}",
                    idx,
                    chart_type.label,
                    chart_type.visible
                );

                if let Some(additional_cols) = &chart_type.additional_data_columns {
                    log::info!(
                        "[LineGraph]     Has {} additional columns",
                        additional_cols.len()
                    );

                    for (_data_type, column_name) in additional_cols {
                        log::info!("[LineGraph]     Adding excluded column: '{}'", column_name);
                        if !excluded_columns.contains(column_name) {
                            excluded_columns.push(column_name.clone());
                        }
                    }
                } else {
                    log::info!("[LineGraph]     No additional columns");
                }
            }
        } else {
            log::warn!("[LineGraph] Preset '{}' not found!", preset_name);
        }

        // Always exclude "side" and "volume" as defaults
        for default_exclude in ["side", "volume"] {
            if !excluded_columns.contains(&default_exclude.to_string()) {
                log::info!(
                    "[LineGraph] Adding default excluded column: '{}'",
                    default_exclude
                );
                excluded_columns.push(default_exclude.to_string());
            }
        }

        log::info!(
            "[LineGraph] Final excluded columns from preset '{}': {:?}",
            preset_name,
            excluded_columns
        );
        excluded_columns
    }

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

        let params = get_query_params();

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
        let mut data_store = DataStore::new(width, height, start_x, end_x);
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

        // Create a default multi-renderer with basic line plot and axes
        let multi_renderer = renderer
            .create_multi_renderer()
            .with_render_order(renderer::RenderOrder::BackgroundToForeground)
            .add_plot_renderer()
            .add_x_axis_renderer(
                renderer.data_store().screen_size.width,
                renderer.data_store().screen_size.height,
            )
            .add_y_axis_renderer(
                renderer.data_store().screen_size.width,
                renderer.data_store().screen_size.height,
            )
            .build();

        // Create the LineGraph instance
        Ok(Self {
            renderer,
            data_manager,
            canvas_controller,
            preset_manager: PresetManager::new(),
            multi_renderer: Some(multi_renderer),
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
    }

    pub fn set_preset(&mut self, preset_name: Option<String>) {}
}

pub fn unix_timestamp_to_string(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    datetime.unwrap().to_rfc3339()
}
