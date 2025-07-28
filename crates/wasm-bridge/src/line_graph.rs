use chrono::DateTime;
use config_system::{GpuChartsConfig, PresetManager};
use data_manager::{ChartType, DataManager, DataStore};
use renderer::{MultiRenderer, Renderer};
use shared_types::DataHandle;
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use crate::wrappers::js::get_query_params;
#[cfg(target_arch = "wasm32")]
use js_sys::Error;

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

pub struct LineGraph {
    pub renderer: Renderer,
    pub data_manager: DataManager,
    pub preset_manager: PresetManager,
    pub multi_renderer: Option<MultiRenderer>,
}

impl LineGraph {
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
    #[cfg(target_arch = "wasm32")]
    pub async fn new(
        width: u32,
        height: u32,
        canvas: HtmlCanvasElement,
    ) -> Result<LineGraph, Error> {
        let params = get_query_params();

        // Handle missing query parameters gracefully (for React integration)
        let topic = params
            .get("topic")
            .unwrap_or(&"default_topic".to_string())
            .clone();
        let start = params
            .get("start")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                // Default to last hour if no start time provided
                chrono::Utc::now().timestamp() - 3600
            });
        let end = params
            .get("end")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                // Default to current time if no end time provided
                chrono::Utc::now().timestamp()
            });

        // Create DataStore
        let mut data_store = DataStore::new(width, height);
        data_store.topic = Some(topic.clone());

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

        // Create config
        let config = GpuChartsConfig::default();

        // Set the time range BEFORE creating the renderer
        data_store.set_x_range(start as u32, end as u32);

        // Log initial state before moving data_store
        log::info!("Initial DataStore state:");
        log::info!(
            "  - X range: {} to {}",
            data_store.start_x,
            data_store.end_x
        );
        log::info!(
            "  - Y bounds: min={:?}, max={:?}",
            data_store.min_y,
            data_store.max_y
        );
        log::info!(
            "  - Excluded columns: {:?}",
            data_store.get_excluded_columns()
        );

        // Create Renderer with modular approach
        let renderer = Renderer::new(canvas, device.clone(), queue.clone(), config, data_store)
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
            preset_manager: PresetManager::new(),
            multi_renderer: Some(multi_renderer),
        })
    }

    pub async fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

    // Switch chart type
    pub fn set_chart_type(&mut self, chart_type: &str) {
        let new_type = match chart_type {
            "candlestick" => ChartType::Candlestick,
            _ => ChartType::Line,
        };

        log::info!("Setting chart type to {new_type:?}");
        self.renderer.set_chart_type(new_type);
        log::info!("Chart type changed - renderer should be marked dirty");
    }

    // Set candle timeframe (in seconds)
    pub fn set_candle_timeframe(&mut self, timeframe_seconds: u32) {
        self.renderer
            .data_store_mut()
            .set_candle_timeframe(timeframe_seconds);
    }

    #[allow(dead_code)]
    fn process_data_handle(
        data_handle: &DataHandle,
        data_manager: &mut DataManager,
        data_store: &mut DataStore,
        _device: &Rc<wgpu::Device>,
    ) -> Result<(), shared_types::GpuChartsError> {
        // Get the GPU buffer set from the data manager
        let gpu_buffer_set = data_manager.get_buffers(data_handle).ok_or_else(|| {
            shared_types::GpuChartsError::DataNotFound {
                resource: "GPU buffers for data handle".to_string(),
            }
        })?;

        // Clear existing data groups before adding new data
        data_store.data_groups.clear();
        data_store.active_data_group_indices.clear();

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

        // Add the data group with time as x-axis
        data_store.add_data_group((time_buffer.clone(), time_gpu_buffers.clone()), true);

        let data_group_index = 0; // We just added the first group

        // Mark the group as active
        data_store.active_data_group_indices.push(data_group_index);

        // Add each metric column
        for (i, column_name) in gpu_buffer_set.metadata.columns.iter().enumerate() {
            if column_name == "time" {
                continue; // Skip time column as it's already the x-axis
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
                    _ => {
                        // Generate a color based on index
                        let hue = (i as f32 * 137.5) % 360.0;
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

        log::info!(
            "Successfully added {} columns to DataStore",
            gpu_buffer_set.metadata.columns.len()
        );

        log::info!("[process_data_handle] Data loaded, before Y bounds calculation:");
        log::info!(
            "  - Excluded columns: {:?}",
            data_store.get_excluded_columns()
        );
        log::info!("  - Total data groups: {}", data_store.data_groups.len());

        // Log all metrics that were added
        for (idx, group) in data_store.data_groups.iter().enumerate() {
            log::info!("  - Data group[{}]: {} metrics", idx, group.metrics.len());
            for metric in &group.metrics {
                log::info!(
                    "    - Metric: '{}' (visible={})",
                    metric.name,
                    metric.visible
                );
            }
        }

        // Mark as dirty so bounds will be calculated on next render
        data_store.mark_dirty();

        Ok(())
    }

    // Helper function to convert HSV to RGB
    #[allow(dead_code)]
    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r_prime, g_prime, b_prime) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        (r_prime + m, g_prime + m, b_prime + m)
    }
}

pub fn unix_timestamp_to_string(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    datetime.unwrap().to_rfc3339()
}
