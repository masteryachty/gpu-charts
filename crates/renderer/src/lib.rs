//! Pure GPU rendering engine for GPU Charts
//! Implements Phase 3 optimizations for extreme performance

#![allow(clippy::uninlined_format_args)]

pub mod calcables;
pub mod charts;
pub mod compute;
pub mod compute_engine;
pub mod drawables;
pub mod mesh_builder;
pub mod multi_renderer;
pub mod pipeline_builder;
pub mod shaders;

use config_system::GpuChartsConfig;
use data_manager::DataStore;
use shared_types::{GpuChartsError, GpuChartsResult, RenderStats};
use std::rc::Rc;
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

    #[allow(dead_code)] // Will be used for quality settings and performance tuning
    settings: GpuChartsConfig,
    data_store: DataStore,
    compute_engine: compute_engine::ComputeEngine,
}

impl Renderer {
    /// Create a new renderer
    #[cfg(target_arch = "wasm32")]
    pub async fn new(
        canvas: web_sys::HtmlCanvasElement,
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        config: GpuChartsConfig,
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
            settings: config,
            data_store,
            compute_engine,
        };

        Ok(renderer)
    }

    /// Render a frame using the provided multi-renderer
    pub async fn render(&mut self, multi_renderer: &mut MultiRenderer) -> RenderResult<()> {
        // Check if rendering is needed
        if !self.data_store.is_dirty() {
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

        // Run pre-render compute passes (e.g., compute mid price)
        // This must happen BEFORE calculating min/max bounds
        if self.data_store.min_max_buffer.is_none() {
            log::debug!("[Renderer] Running pre-render compute passes...");
            self.compute_engine
                .run_compute_passes(&mut encoder, &mut self.data_store);
        }

        // Calculate Y bounds using GPU if not already calculated
        if self.data_store.min_max_buffer.is_none() {
            log::debug!("[Renderer] Calculating Y bounds using GPU min/max...");

            let (x_min, x_max) = (self.data_store.start_x, self.data_store.end_x);
            let (min_max_buffer, staging_buffer) = calculate_min_max_y(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.data_store,
                x_min,
                x_max,
            );

            // Store both GPU min/max buffer and staging buffer
            self.data_store.min_max_buffer = Some(std::rc::Rc::new(min_max_buffer));
            self.data_store.min_max_staging_buffer = Some(std::rc::Rc::new(staging_buffer));
        }

        // Update the shared bind group with GPU-calculated bounds
        self.data_store
            .update_shared_bind_group_with_gpu_buffer(&self.device);

        // Let MultiRenderer handle all rendering
        multi_renderer.render(&mut encoder, &view, &self.data_store)?;

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Present the frame
        output.present();

        // After presenting, try to read GPU bounds for next frame
        // This is non-blocking and will update the bounds when ready
        let needs_rerender = if self.data_store.gpu_min_y.is_none() {
            self.try_read_gpu_bounds()
        } else {
            false
        };

        // Mark as clean after successful render unless we need another render for GPU bounds
        if !needs_rerender {
            self.data_store.mark_clean();
        }

        Ok(())
    }

    /// Update chart type
    pub fn set_chart_type(&mut self, chart_type: data_manager::ChartType) {
        log::info!(
            "set_chart_type called: {:?} -> {:?}",
            self.data_store.chart_type,
            chart_type
        );
        let old_type = self.data_store.chart_type;
        self.data_store.chart_type = chart_type;

        // Mark data store as dirty to trigger a render
        self.data_store.mark_dirty();

        log::info!("Chart type changed from {old_type:?} to {chart_type:?} - marked dirty");
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.data_store.resized(width, height);
        log::info!("Resized surface to {{ width: {width}, height: {height} }}");
    }

    /// Get current statistics
    pub fn get_stats(&self) -> RenderStats {
        // Multi-renderer tracks its own stats
        RenderStats {
            frame_time_ms: 0.0,
            draw_calls: 0, // Would be provided by multi-renderer
            vertices_rendered: 0,
            gpu_memory_used: 0,
        }
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
        if self.data_store.gpu_min_y.is_none() && self.data_store.min_max_staging_buffer.is_some() {
            return true;
        }

        false
    }

    /// Try to read GPU-calculated bounds from the staging buffer
    /// Returns true if bounds were successfully read and a re-render is needed
    fn try_read_gpu_bounds(&mut self) -> bool {
        // Skip if we already have the bounds
        if self.data_store.gpu_min_y.is_some() {
            return false;
        }

        if let Some(staging_buffer) = self.data_store.min_max_staging_buffer.clone() {
            if self.data_store.gpu_buffer_ready {
                // Buffer should be mapped and ready to read
                let data = staging_buffer.slice(..).get_mapped_range();
                let floats: &[f32] = bytemuck::cast_slice(&data);
                if floats.len() >= 2 {
                    log::debug!(
                        "[Renderer] Read GPU bounds: min={}, max={}",
                        floats[0],
                        floats[1]
                    );
                    self.data_store.set_gpu_y_bounds(floats[0], floats[1]);
                    // Mark dirty to trigger re-render with updated labels
                    self.data_store.mark_dirty();
                    drop(data);
                    staging_buffer.unmap();
                    self.data_store.gpu_buffer_mapped = false;
                    self.data_store.gpu_buffer_ready = false;
                    return true; // Request re-render
                }
                drop(data);
                staging_buffer.unmap();
                self.data_store.gpu_buffer_mapped = false;
                self.data_store.gpu_buffer_ready = false;
            } else if !self.data_store.gpu_buffer_mapped {
                // Request mapping for next frame
                let buffer_slice = staging_buffer.slice(..);

                buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                    if result.is_ok() {
                        log::debug!("[Renderer] GPU min/max buffer mapped successfully");
                    } else {
                        log::error!("[Renderer] Failed to map GPU min/max buffer");
                    }
                });
                self.data_store.gpu_buffer_mapped = true;

                // Poll to start the mapping process
                self.device.poll(wgpu::Maintain::Poll);
            } else {
                // Buffer mapping was requested, check if it's ready
                // Poll more aggressively to check for completion
                self.device.poll(wgpu::Maintain::Wait);

                // After polling, the buffer might be ready
                // We'll mark it as ready and try to read on the next frame
                self.data_store.gpu_buffer_ready = true;
                // Mark dirty to trigger another render attempt
                self.data_store.mark_dirty();
            }
        }
        false
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

    /// Example: Create a multi-renderer with candles and volume bars
    pub fn create_candles_with_volume_renderer(&self) -> MultiRenderer {
        let width = self.data_store.screen_size.width;
        let height = self.data_store.screen_size.height;

        let mut multi_renderer = self
            .create_multi_renderer()
            .with_render_order(RenderOrder::BackgroundToForeground)
            .build();

        // Add volume bars first (background)
        let volume_renderer = drawables::volume_bars::create_custom_volume_renderer(
            self.device.clone(),
            self.queue.clone(),
            self.config.format,
        );
        multi_renderer.add_renderer(volume_renderer);

        // Add candlesticks
        let candle_renderer =
            CandlestickRenderer::new(self.device.clone(), self.queue.clone(), self.config.format);
        multi_renderer.add_renderer(Box::new(candle_renderer));

        // Add axes on top
        let x_axis = XAxisRenderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.config.format,
            width,
            height,
        );
        multi_renderer.add_renderer(Box::new(x_axis));

        let y_axis = YAxisRenderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.config.format,
            width,
            height,
        );
        multi_renderer.add_renderer(Box::new(y_axis));

        multi_renderer
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
