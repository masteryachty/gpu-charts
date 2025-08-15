use chrono::DateTime;
use config_system::PresetManager;
use data_manager::{DataManager, DataStore};
use renderer::{
    compute_engine::ComputeEngine, MultiRenderer, MultiRendererBuilder, RenderContext, RenderOrder,
};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::controls::canvas_controller::CanvasController;
use crate::instance_manager::InstanceManager;

use js_sys::Error;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;

struct PendingReadback {
    buffer: Rc<wgpu::Buffer>,
    mapping_started: bool,
    mapping_completed: Arc<Mutex<bool>>,
}

pub struct ChartEngine {
    // WebGPU resources managed by RenderContext
    render_context: RenderContext,
    data_store: DataStore,
    compute_engine: ComputeEngine,
    // Track pending GPU readback
    pending_readback: Option<PendingReadback>,

    // ChartEngine original fields
    pub canvas_controller: CanvasController,
    pub data_manager: DataManager,
    pub preset_manager: PresetManager,
    pub multi_renderer: Option<MultiRenderer>,
    instance_id: Uuid,
}

impl ChartEngine {
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
                // power_preference: wgpu::PowerPreference::HighPerformance,
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

        let api_base_url = option_env!("API_BASE_URL")
            .unwrap_or("https://api.rednax.io")
            .to_string();

        // Create DataManager with modular approach
        let data_manager = DataManager::new(device.clone(), queue.clone(), api_base_url);

        // Surface configuration (previously in Renderer::new)
        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats[0];

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoNoVsync, // Use AutoNoVsync for better performance
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1, // Reduced latency
        };
        surface.configure(&device, &surface_config);

        // Create RenderContext to manage GPU resources
        let render_context =
            RenderContext::new(device.clone(), queue.clone(), surface, surface_config);

        // Create compute engine
        let compute_engine = ComputeEngine::new(device, queue);

        // Create immediate updater
        let instance_id = Uuid::new_v4();

        // Create the ChartEngine instance
        Ok(Self {
            render_context,
            data_store,
            compute_engine,
            pending_readback: None,
            data_manager,
            canvas_controller,
            preset_manager: PresetManager::new(),
            multi_renderer: None,
            instance_id,
        })
    }

    pub fn needs_render(&self) -> bool {
        // Check if data is dirty
        if self.data_store.is_dirty() {
            return true;
        }

        // Check if we're waiting for GPU bounds to be read
        if self.data_store.gpu_min_y.is_none() && self.pending_readback.is_some() {
            return true;
        }

        false
    }

    /// Get mutable access to data store
    pub fn data_store_mut(&mut self) -> &mut DataStore {
        &mut self.data_store
    }

    /// Get access to data store
    pub fn data_store(&self) -> &DataStore {
        &self.data_store
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Check if rendering is needed
        if !self.data_store.is_dirty() {
            return Ok(());
        }

        // Get current texture
        let output = self.render_context.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder =
            self.render_context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Start GPU bounds calculation if needed
        if self.data_store.gpu_min_y.is_none() && self.data_store.min_max_buffer.is_none() {
            // Run pre-render compute passes (e.g., compute mid price)
            self.compute_engine
                .run_compute_passes(&mut encoder, &mut self.data_store);

            // Calculate Y bounds using GPU
            let (x_min, x_max) = (self.data_store.start_x, self.data_store.end_x);
            let (min_max_buffer, staging_buffer) = renderer::calculate_min_max_y(
                &self.render_context.device,
                &self.render_context.queue,
                &mut encoder,
                &self.data_store,
                x_min,
                x_max,
            );

            // Store buffers
            self.data_store.min_max_buffer = Some(Rc::new(min_max_buffer));
            self.data_store
                .update_shared_bind_group_with_gpu_buffer(&self.render_context.device);

            self.pending_readback = Some(PendingReadback {
                buffer: Rc::new(staging_buffer),
                mapping_started: false,
                mapping_completed: Arc::new(Mutex::new(false)),
            });
        }

        // Let MultiRenderer handle all rendering
        if let Some(ref mut multi_renderer) = self.multi_renderer {
            multi_renderer
                .render(&mut encoder, &view, &self.data_store)
                .map_err(|e| match e {
                    shared_types::GpuChartsError::Surface { .. } => wgpu::SurfaceError::Outdated,
                    _ => wgpu::SurfaceError::Outdated,
                })?;
        } else {
            // This should never happen since we create a default multi-renderer in new()
            return Err(wgpu::SurfaceError::Outdated);
        }

        // Submit commands
        self.render_context
            .queue
            .submit(std::iter::once(encoder.finish()));

        // Present the frame
        output.present();

        // Log successful render completion

        // Clear dirty flag before processing readback
        // This ensures that if readback marks the store dirty, it stays dirty for next frame
        self.data_store.mark_clean();

        self.rerender();
        Ok(())
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.render_context.resize(width, height);
        self.data_store.resized(width, height);

        // Also resize the multi-renderer if present
        if let Some(ref mut multi_renderer) = self.multi_renderer {
            multi_renderer.resize(width, height);
        }

        // Use state diff to determine required actions
        // let actions = diff.get_required_actions();

        // if actions.needs_pipeline_rebuild {
        //     self.on_resized(width, height); // Resizing requires pipeline rebuild
        // } else if actions.needs_render {
        //     // self.on_view_changed();
        // }
    }

    pub fn set_preset_and_symbol(
        &mut self,
        preset_name: Option<String>,
        symbol_name: Option<String>,
    ) {
        // Update config state if preset changed
        if let Some(name) = preset_name.clone() {
            let preset = self.preset_manager.find_preset(&name).cloned();
            if let Some(preset) = preset {
                // Update data store with preset
                self.data_store
                    .set_preset_and_symbol(Some(&preset), symbol_name.clone());

                // Rebuild multi-renderer based on preset configuration
                self.rebuild_multi_renderer_for_preset(&preset);

                // Clear GPU bounds to force recalculation when switching presets
                self.data_store.clear_gpu_bounds();

                // Also clear any pending readback operations
                self.pending_readback = None;
            } else {
                self.data_store
                    .set_preset_and_symbol(None, symbol_name.clone());
            }
        } else {
            self.data_store
                .set_preset_and_symbol(None, symbol_name.clone());
        }
    }

    /// Called when resized
    pub fn on_resized(&mut self, _width: u32, _height: u32) {}

    /// Called when metric visibility changes
    pub fn on_metric_visibility_changed(&mut self) {
        // Rebuild the renderer if we have a preset
        if let Some(preset) = &self.data_store.preset {
            let preset_clone = preset.clone();
            self.rebuild_multi_renderer_for_preset(&preset_clone);

            // Clear GPU bounds to force recalculation
            self.data_store.clear_gpu_bounds();

            // Also clear any pending readback operations
            self.pending_readback = None;
        }
    }

    /// Set instance ID (used by instance manager)
    pub fn set_instance_id(&mut self, id: Uuid) {
        self.instance_id = id;
    }

    /// Rebuild the multi-renderer based on preset configuration
    fn rebuild_multi_renderer_for_preset(&mut self, preset: &config_system::ChartPreset) {
        // Get current screen dimensions
        let width = self.data_store.screen_size.width;
        let height = self.data_store.screen_size.height;

        // Create new multi-renderer
        let mut builder = self
            .create_multi_renderer()
            .with_render_order(RenderOrder::BackgroundToForeground);

        // Add renderers based on preset chart types
        for chart_type in &preset.chart_types {
            if !chart_type.visible {
                continue;
            }

            match chart_type.render_type {
                config_system::RenderType::Line => {
                    // Create a configurable plot renderer with specific data columns
                    let plot_renderer = renderer::ConfigurablePlotRenderer::new(
                        self.render_context.device.clone(),
                        self.render_context.queue.clone(),
                        self.render_context.config.format,
                        chart_type.label.clone(),
                        chart_type.data_columns.clone(),
                    );
                    builder = builder.add_renderer(Box::new(plot_renderer));
                }
                config_system::RenderType::Triangle => {
                    let triangle_renderer = renderer::charts::TriangleRenderer::new(
                        self.render_context.device.clone(),
                        self.render_context.queue.clone(),
                        self.render_context.config.format,
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

    /// Process pending readback with improved safety and timeout
    fn process_pending_readback(&mut self) -> bool {
        if let Some(pending) = &mut self.pending_readback {
            // Use try_lock to avoid potential deadlocks
            let mapping_completed = match pending.mapping_completed.try_lock() {
                Ok(guard) => guard,
                Err(_) => {
                    // Lock is held by another thread, skip this frame
                    return false;
                }
            };

            // Check if we need to initiate the mapping
            if !pending.mapping_started && !*mapping_completed {
                // Mark that we've started mapping to avoid double mapping
                pending.mapping_started = true;

                // Drop the lock before initiating mapping to avoid holding it during async operation
                drop(mapping_completed);

                // Clone what we need for the closure
                let mapping_completed_clone = pending.mapping_completed.clone();
                let buffer_slice = pending.buffer.slice(..);

                // Now initiate the mapping with error handling
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    if let Ok(mut guard) = mapping_completed_clone.try_lock() {
                        *guard = result.is_ok();
                    }
                });

                // Poll to make progress on async operations
                self.render_context.device.poll(wgpu::Maintain::Poll);

                return false; // Come back next frame to check completion
            }

            // Check if mapping is complete
            if *mapping_completed {
                // Drop the lock before processing
                drop(mapping_completed);

                // Take ownership to read the buffer
                if let Some(pending) = self.pending_readback.take() {
                    // Process buffer data
                    let buffer_slice = pending.buffer.slice(..);
                    let mapped = buffer_slice.get_mapped_range();
                    let data: &[f32] = bytemuck::cast_slice(&mapped);

                    if data.len() >= 2 {
                        // Update bounds
                        let min_val = data[0];
                        let max_val = data[1];

                        // Validate bounds before applying
                        if min_val.is_finite() && max_val.is_finite() && min_val < max_val {
                            self.data_store.set_gpu_y_bounds(min_val, max_val);
                            // Update the bind group with the new bounds
                            self.data_store.update_shared_bind_group_with_gpu_buffer(
                                &self.render_context.device,
                            );
                        } else {
                            // Use sensible defaults if GPU bounds are invalid
                            log::warn!("Invalid GPU bounds: min={min_val}, max={max_val}");
                        }
                    }

                    // Always clean up
                    drop(mapped);
                    pending.buffer.unmap();

                    return true;
                }
            }
        }
        false
    }

    /// Create a multi-renderer pipeline for complex visualizations
    ///
    /// Example usage:
    /// ```rust,ignore
    /// let multi_renderer = chart_engine.create_multi_renderer()
    ///     .with_render_order(RenderOrder::BackgroundToForeground)
    ///     .add_candlestick_renderer()
    ///     .add_plot_renderer()
    ///     .add_x_axis_renderer(width, height)
    ///     .add_y_axis_renderer(width, height)
    ///     .build();
    /// ```
    pub fn create_multi_renderer(&self) -> MultiRendererBuilder {
        MultiRendererBuilder::new(
            self.render_context.device.clone(),
            self.render_context.queue.clone(),
            self.render_context.config.format,
        )
    }

    /// Example: Create a multi-renderer with multiple line plots
    pub fn create_multi_line_renderer(&self) -> MultiRenderer {
        let width = self.data_store.screen_size.width;
        let height = self.data_store.screen_size.height;
        // In a real implementation, you'd create multiple PlotRenderers
        // with different data sources/colors
        self.create_multi_renderer()
            .with_render_order(RenderOrder::Sequential)
            .add_plot_renderer()
            .add_x_axis_renderer(width, height)
            .add_y_axis_renderer(width, height)
            .build()
    }

    fn rerender(&mut self) {
        // Check if we need to schedule another render
        if self.pending_readback.is_some() {
            // Process any pending readback
            let compute_complete = self.process_pending_readback();

            let instance_id = self.instance_id;
            if compute_complete {
                wasm_bindgen_futures::spawn_local(async move {
                    // Use InstanceManager to access the instance
                    InstanceManager::with_instance_mut(&instance_id, |instance| {
                        let _ = instance.chart_engine.render();
                    });
                })
            } else {
                wasm_bindgen_futures::spawn_local(async move {
                    let promise = js_sys::Promise::new(&mut |resolve, _| {
                        web_sys::window()
                            .unwrap()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 10)
                            .unwrap();
                    });
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

                    // Use InstanceManager to access the instance
                    InstanceManager::with_instance_mut(&instance_id, |instance| {
                        instance.chart_engine.rerender();
                    });
                });
            }
        }
    }

    /// Handle cursor events with simplified canvas controller
    pub fn handle_cursor_event(&mut self, event: shared_types::events::WindowEvent) {
        use shared_types::events::{ElementState, MouseScrollDelta, WindowEvent};

        match event {
            WindowEvent::MouseWheel {
                delta, phase: _, ..
            } => {
                let MouseScrollDelta::PixelDelta(position) = delta;

                let start_x = self.data_store.start_x;
                let end_x = self.data_store.end_x;
                let range = end_x - start_x;

                // Zoom factor based on scroll amount
                let zoom_factor = 0.001; // Reduced zoom factor for smoother zooming
                let zoom_amount = position.y.abs() * zoom_factor;

                // Calculate zoom centered on mouse position
                // position.x contains the mouse x coordinate relative to canvas
                let mouse_x_ratio = if self.data_store.screen_size.width > 0 {
                    position.x / self.data_store.screen_size.width as f64
                } else {
                    0.5 // Default to center if width not set
                };

                let (new_start, new_end) = if position.y < 0. {
                    // Scrolling up = zoom in (shrink range)
                    let zoom_pixels = (range as f64 * zoom_amount) as u32;
                    let left_zoom = (zoom_pixels as f64 * mouse_x_ratio) as u32;
                    let right_zoom = zoom_pixels - left_zoom;

                    let new_start = start_x + left_zoom;
                    let new_end = end_x - right_zoom;

                    // Ensure we don't zoom in too much (minimum range of 10 units)
                    if new_end > new_start + 10 {
                        (new_start, new_end)
                    } else {
                        (start_x, end_x) // Keep current range if too zoomed in
                    }
                } else if position.y > 0. {
                    // Scrolling down = zoom out (expand range)
                    let zoom_pixels = (range as f64 * zoom_amount) as u32;
                    let left_zoom = (zoom_pixels as f64 * mouse_x_ratio) as u32;
                    let right_zoom = zoom_pixels - left_zoom;

                    let new_start = start_x.saturating_sub(left_zoom);
                    let new_end = end_x + right_zoom;
                    (new_start, new_end)
                } else {
                    (start_x, end_x) // No change
                };

                // Only update if range actually changed
                if new_start != start_x || new_end != end_x {
                    self.data_store.set_x_range(new_start, new_end);
                    self.data_store.mark_dirty();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.canvas_controller.update_position(position.into());
            }
            WindowEvent::MouseInput {
                state, button: _, ..
            } => {
                match state {
                    ElementState::Pressed => {
                        self.canvas_controller.start_drag();
                    }
                    ElementState::Released => {
                        if let Some((start_pos, end_pos)) = self.canvas_controller.end_drag() {
                            // Apply drag zoom
                            let start_ts = self.data_store.screen_to_world_with_margin(
                                start_pos.x as f32,
                                start_pos.y as f32,
                            );
                            let end_ts = self
                                .data_store
                                .screen_to_world_with_margin(end_pos.x as f32, end_pos.y as f32);

                            // Ensure start is less than end
                            let (new_start, new_end) = if start_ts.0 < end_ts.0 {
                                (start_ts.0 as u32, end_ts.0 as u32)
                            } else {
                                (end_ts.0 as u32, start_ts.0 as u32)
                            };

                            self.data_store.set_x_range(new_start, new_end);
                            self.data_store.mark_dirty();
                        }
                    }
                }
            }
        }
    }
}

impl Drop for ChartEngine {
    fn drop(&mut self) {
        // Clean up any pending GPU operations
        if self.pending_readback.is_some() {
            self.pending_readback = None;
        }

        // Ensure all GPU work is completed before dropping
        self.render_context.device.poll(wgpu::Maintain::Wait);
    }
}

pub fn unix_timestamp_to_string(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    datetime.unwrap().to_rfc3339()
}
