pub mod calcables;
pub mod charts;
pub mod compute;
pub mod compute_engine;
pub mod drawables;
pub mod multi_renderer;
pub mod pipeline_builder;
pub mod shaders;

use config_system::ChartPreset;
use data_manager::DataStore;
use shared_types::{GpuChartsError, GpuChartsResult};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wgpu::{CommandEncoder, Device, Queue, TextureView};

pub use calcables::{candle_aggregator::CandleAggregator, min_max::calculate_min_max_y};
pub use charts::TriangleRenderer;
pub use drawables::{
    candlestick::CandlestickRenderer, plot::PlotRenderer, x_axis::XAxisRenderer,
    y_axis::YAxisRenderer,
};
pub use multi_renderer::{
    ConfigurablePlotRenderer, MultiRenderable, MultiRenderer, MultiRendererBuilder, RenderOrder,
    RendererAdapter,
};

/// Re-export error types
pub type RenderError = GpuChartsError;
pub type RenderResult<T> = GpuChartsResult<T>;

/// Main renderer that orchestrates all rendering operations
pub struct Renderer {
    // WebGPU resources (previously in RenderEngine)
    pub surface: wgpu::Surface<'static>,
    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
    pub config: wgpu::SurfaceConfiguration,
    data_store: DataStore,
    compute_engine: compute_engine::ComputeEngine,
    // Track pending GPU readback
    pending_readback: Option<PendingReadback>,
}

struct PendingReadback {
    buffer: Rc<wgpu::Buffer>,
    mapping_started: bool,
    mapping_completed: Arc<Mutex<bool>>,
}

impl Renderer {
    /// Create a new renderer
    pub async fn new(
        canvas: web_sys::HtmlCanvasElement,
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        data_store: DataStore,
    ) -> RenderResult<Self> {
        // Create WebGPU instance and surface
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| GpuChartsError::Surface {
                message: format!("Failed to create surface: {e}"),
            })?;

        // Get surface capabilities
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or_else(|| GpuChartsError::GpuInit {
                message: "Failed to get adapter".to_string(),
            })?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats[0];

        let width = data_store.screen_size.width;
        let height = data_store.screen_size.height;

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

        // Create compute engine
        let compute_engine = compute_engine::ComputeEngine::new(device.clone(), queue.clone());

        let renderer = Self {
            surface,
            device,
            queue,
            config: surface_config,
            data_store,
            compute_engine,
            pending_readback: None,
        };

        Ok(renderer)
    }

    /// Calculate bounds using GPU compute
    pub fn calculate_bounds(&mut self, mut encoder: wgpu::CommandEncoder) -> RenderResult<()> {
        // Run pre-render compute passes (e.g., compute mid price)
        self.compute_engine
            .run_compute_passes(&mut encoder, &mut self.data_store);

        // Calculate Y bounds using GPU
        let (x_min, x_max) = (self.data_store.start_x, self.data_store.end_x);
        let (min_max_buffer, staging_buffer) = calculate_min_max_y(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.data_store,
            x_min,
            x_max,
        );

        // Store GPU min/max buffer
        self.data_store.min_max_buffer = Some(std::rc::Rc::new(min_max_buffer));

        // Update the shared bind group with GPU-calculated bounds
        self.data_store
            .update_shared_bind_group_with_gpu_buffer(&self.device);

        // Submit the command buffer
        self.queue.submit(std::iter::once(encoder.finish()));

        // Queue non-blocking readback
        self.queue_bounds_readback(std::rc::Rc::new(staging_buffer));

        Ok(())
    }

    /// Render a frame using the provided multi-renderer
    pub fn render(&mut self, multi_renderer: &mut MultiRenderer) -> RenderResult<()> {
        // Check if rendering is needed
        if !self.data_store.is_dirty() && self.pending_readback.is_none() {
            return Ok(());
        }

        // Get current texture
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self
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
            let (min_max_buffer, staging_buffer) = calculate_min_max_y(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.data_store,
                x_min,
                x_max,
            );

            // Store buffers
            self.data_store.min_max_buffer = Some(Rc::new(min_max_buffer));
            self.data_store
                .update_shared_bind_group_with_gpu_buffer(&self.device);

            // Queue non-blocking readback
            self.queue_bounds_readback(Rc::new(staging_buffer));
        }

        // Let MultiRenderer handle all rendering
        multi_renderer.render(&mut encoder, &view, &self.data_store)?;

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Present the frame
        output.present();

        // Clear dirty flag before processing readback
        // This ensures that if readback marks the store dirty, it stays dirty for next frame
        self.data_store.mark_clean();

        // Process any pending readback
        self.process_pending_readback();

        Ok(())
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.data_store.resized(width, height);
    }

    /// Get mutable access to data store
    pub fn data_store_mut(&mut self) -> &mut DataStore {
        &mut self.data_store
    }

    /// Get access to data store
    pub fn data_store(&self) -> &DataStore {
        &self.data_store
    }

    /// Check if the renderer needs to render
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

    /// Queue a non-blocking readback of GPU bounds
    fn queue_bounds_readback(&mut self, staging_buffer: Rc<wgpu::Buffer>) {
        // Store the buffer for later mapping
        // We'll initiate the mapping in process_pending_readback after device.poll()
        self.pending_readback = Some(PendingReadback {
            buffer: staging_buffer,
            mapping_started: false,
            mapping_completed: Arc::new(Mutex::new(false)),
        });
    }

    /// Process pending readback (non-blocking)
    fn process_pending_readback(&mut self) {
        if let Some(pending) = &mut self.pending_readback {
            // Poll to make progress on async operations
            self.device.poll(wgpu::Maintain::Poll);

            let mut mapping_completed = pending.mapping_completed.lock().unwrap();

            // Check if we need to initiate the mapping
            if !pending.mapping_started && !*mapping_completed {
                // Mark that we've started mapping to avoid double mapping
                pending.mapping_started = true;

                // Drop the lock before initiating mapping to avoid holding it during async operation
                drop(mapping_completed);

                // Clone what we need for the closure
                let mapping_completed_clone = pending.mapping_completed.clone();
                let buffer_slice = pending.buffer.slice(..);

                // Now initiate the mapping
                buffer_slice.map_async(wgpu::MapMode::Read, move |result| match result {
                    Ok(()) => {
                        *mapping_completed_clone.lock().unwrap() = true;
                    }
                    Err(e) => {}
                });

                // Poll again to potentially complete the mapping immediately
                self.device.poll(wgpu::Maintain::Poll);

                // Re-acquire the lock to check if mapping completed
                mapping_completed = pending.mapping_completed.lock().unwrap();
            }

            // Check if mapping is complete
            if *mapping_completed {
                // Drop the lock before processing
                drop(mapping_completed);

                // Take ownership to read the buffer
                if let Some(pending) = self.pending_readback.take() {
                    let buffer_slice = pending.buffer.slice(..);
                    let mapped = buffer_slice.get_mapped_range();
                    let data: &[f32] = bytemuck::cast_slice(&mapped);

                    if data.len() >= 2 {
                        // Update bounds
                        let min_val = data[0];
                        let max_val = data[1];

                        // Check if we got valid bounds
                        if min_val >= max_val || (min_val == 0.0 && max_val == 1.0) {
                            // Use sensible defaults if GPU bounds are invalid
                            self.data_store.set_gpu_y_bounds(0.0, 100.0);
                        } else {
                            self.data_store.set_gpu_y_bounds(min_val, max_val);
                        }

                        // Clean up
                        drop(mapped);
                        pending.buffer.unmap();
                    } else {
                        drop(mapped);
                        pending.buffer.unmap();
                    }
                }
            }
        }
    }

    /// Process a state diff for incremental updates
    pub fn process_state_change(&mut self, _diff: &shared_types::StateDiff) {
        // This method is kept for API compatibility but incremental updates were not implemented
    }

    /// Create a multi-renderer pipeline for complex visualizations
    ///
    /// Example usage:
    /// ```rust,ignore
    /// let multi_renderer = renderer.create_multi_renderer()
    ///     .with_render_order(RenderOrder::BackgroundToForeground)
    ///     .add_candlestick_renderer()
    ///     .add_plot_renderer()
    ///     .add_x_axis_renderer(width, height)
    ///     .add_y_axis_renderer(width, height)
    ///     .build();
    /// ```
    pub fn create_multi_renderer(&self) -> MultiRendererBuilder {
        MultiRendererBuilder::new(self.device.clone(), self.queue.clone(), self.config.format)
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

    pub fn set_preset_and_symbol(
        &mut self,
        preset: Option<&ChartPreset>,
        symbol_name: Option<String>,
    ) {
        self.data_store_mut()
            .set_preset_and_symbol(preset, symbol_name);
    }
}

/// Trait for chart-specific renderers
pub trait ChartRenderer: Send + Sync {
    /// Render the chart
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        device: &Device,
        queue: &Queue,
    );

    /// Handle resize
    fn resize(&mut self, width: u32, height: u32);

    /// Get renderer name
    fn name(&self) -> &str;
}
