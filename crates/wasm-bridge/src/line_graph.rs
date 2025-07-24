use chrono::DateTime;
use config_system::GpuChartsConfig;
use data_manager::{ChartType, DataManager, DataStore};
use js_sys::{Float32Array, Uint32Array};
use renderer::Renderer;
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
}

impl LineGraph {
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
        let mut data_manager = DataManager::new(
            device.clone(),
            queue.clone(),
            "https://api.rednax.io".to_string(),
        );

        // Create config
        let config = GpuChartsConfig::default();

        // Set the time range BEFORE creating the renderer
        data_store.set_x_range(start as u32, end as u32);

        // Create Renderer with modular approach
        let mut renderer = Renderer::new(canvas, device.clone(), queue.clone(), config, data_store)
            .await
            .map_err(|e| Error::new(&format!("Failed to create renderer: {e:?}")))?;

        // Try to fetch initial data using DataManager
        log::info!("Attempting initial data fetch...");
        let selected_metrics = vec!["time", "best_bid", "best_ask"];
        match data_manager
            .fetch_data(&topic, start as u64, end as u64, &selected_metrics)
            .await
        {
            Ok(data_handle) => {
                log::info!("Data fetched successfully, processing...");
                // Process the data and add it to the DataStore
                if let Err(e) = Self::process_data_handle(
                    &data_handle,
                    &mut data_manager,
                    renderer.data_store_mut(),
                    &device,
                ) {
                    log::error!("Failed to process data: {e:?}");
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch initial data: {e:?}");
            }
        }

        log::info!("LineGraph initialization completed (data fetch may have failed gracefully)");

        // Create the LineGraph instance
        Ok(Self {
            renderer,
            data_manager,
        })
    }

    pub async fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Use the new Renderer
        self.renderer.render().await.map_err(|e| match e {
            shared_types::GpuChartsError::Surface { .. } => wgpu::SurfaceError::Outdated,
            _ => wgpu::SurfaceError::Outdated,
        })
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
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

    fn process_data_handle(
        data_handle: &DataHandle,
        data_manager: &mut DataManager,
        data_store: &mut DataStore,
        device: &Rc<wgpu::Device>,
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

        // Calculate min/max Y values from the loaded data
        Self::calculate_data_bounds(data_store, device)?;

        Ok(())
    }

    fn calculate_data_bounds(
        data_store: &mut DataStore,
        device: &wgpu::Device,
    ) -> Result<(), shared_types::GpuChartsError> {
        // Get all data groups and calculate min/max
        let mut global_min = f32::INFINITY;
        let mut global_max = f32::NEG_INFINITY;
        let mut found_data = false;

        // Check if we have any data groups
        if data_store.data_groups.is_empty() {
            log::warn!("No data groups available for bounds calculation");
            return Ok(());
        }

        // Get the time range for filtering
        let start_x = data_store.start_x;
        let end_x = data_store.end_x;
        log::info!("Calculating bounds for time range: {start_x} - {end_x}");

        // Iterate through all data groups
        for (group_idx, data_group) in data_store.data_groups.iter().enumerate() {
            // Skip if this group is not active
            if !data_store.active_data_group_indices.contains(&group_idx) {
                continue;
            }

            // Get time data from the group
            let x_data = Uint32Array::new(&data_group.x_raw);
            let x_length = x_data.length();

            // Check each metric in this group
            for metric in &data_group.metrics {
                if !metric.visible {
                    continue;
                }

                // Get the Y data from the metric
                let y_data = Float32Array::new(&metric.y_raw);
                let y_length = y_data.length();

                // Ensure arrays have same length
                let length = x_length.min(y_length);

                log::debug!(
                    "Processing metric '{}' with {} data points",
                    metric.name,
                    length
                );

                // Find min/max in the visible range
                let mut points_in_range = 0;
                for i in 0..length {
                    let x_val = x_data.get_index(i);
                    if x_val >= start_x && x_val <= end_x {
                        points_in_range += 1;
                        let y_val = y_data.get_index(i);
                        if !y_val.is_nan() && !y_val.is_infinite() {
                            global_min = global_min.min(y_val);
                            global_max = global_max.max(y_val);
                            found_data = true;
                        }
                    }
                }

                if points_in_range > 0 {
                    log::debug!(
                        "Found {} points in time range {}-{} for metric '{}'",
                        points_in_range,
                        start_x,
                        end_x,
                        metric.name
                    );
                }
            }
        }

        if found_data {
            // Add some margin (10% on each side)
            let range = global_max - global_min;
            let margin = range * 0.1;
            global_min -= margin;
            global_max += margin;

            log::info!(
                "Calculated Y bounds from data: min={global_min}, max={global_max}"
            );
            data_store.update_min_max_y(global_min, global_max);
            // Update the shared bind group with new bounds
            data_store.update_shared_bind_group(device);
        } else {
            log::warn!("No valid data found for Y bounds calculation");
            // Set reasonable defaults for financial data
            data_store.update_min_max_y(0.0, 100000.0);
            // Update the shared bind group with default bounds
            data_store.update_shared_bind_group(device);
        }

        Ok(())
    }

    // Helper function to convert HSV to RGB
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
