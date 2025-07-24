//! Pure GPU rendering engine for GPU Charts
//! Implements Phase 3 optimizations for extreme performance

pub mod calcables;
pub mod charts;
pub mod drawables;
pub mod mesh_builder;
pub mod pipeline_builder;
pub mod shaders;

use config_system::GpuChartsConfig;
use data_manager::DataStore;
use shared_types::{GpuChartsError, GpuChartsResult, RenderStats};
use std::rc::Rc;
use wgpu::{CommandEncoder, Device, Queue, TextureView};

pub use calcables::{candle_aggregator::CandleAggregator, min_max::calculate_min_max_y};
pub use drawables::{
    candlestick::CandlestickRenderer, plot::PlotRenderer, x_axis::XAxisRenderer,
    y_axis::YAxisRenderer,
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

    // Specific renderers
    plot_renderer: Option<PlotRenderer>,
    candlestick_renderer: Option<CandlestickRenderer>,
    x_axis_renderer: Option<XAxisRenderer>,
    y_axis_renderer: Option<YAxisRenderer>,
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
                message: format!("Failed to create surface: {}", e),
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

        let mut renderer = Self {
            surface,
            device,
            queue,
            config: surface_config,
            settings: config,
            data_store,
            plot_renderer: None,
            candlestick_renderer: None,
            x_axis_renderer: None,
            y_axis_renderer: None,
        };

        // Set up initial renderers
        renderer.setup_renderers();

        Ok(renderer)
    }

    /// Setup renderers based on chart type
    pub fn setup_renderers(&mut self) {
        let format = self.config.format;
        let chart_type = self.data_store.chart_type;

        log::info!("Setting up renderers for chart type: {chart_type:?}");

        // Create shared references for device and queue
        let device = self.device.clone();
        let queue = self.queue.clone();

        let width = self.data_store.screen_size.width;
        let height = self.data_store.screen_size.height;

        // Clear only chart-specific renderers, preserve axis renderers to avoid recalculation
        self.plot_renderer = None;
        self.candlestick_renderer = None;

        // Create renderer based on chart type
        match self.data_store.chart_type {
            data_manager::ChartType::Line => {
                self.plot_renderer = Some(PlotRenderer::new(device.clone(), queue.clone(), format));
            }
            data_manager::ChartType::Candlestick => {
                self.candlestick_renderer = Some(CandlestickRenderer::new(
                    device.clone(),
                    queue.clone(),
                    format,
                ));
            }
        }

        // Create axis renderers only if they don't exist
        // This preserves their cached values when switching chart types
        if self.x_axis_renderer.is_none() {
            self.x_axis_renderer = Some(XAxisRenderer::new(
                device.clone(),
                queue.clone(),
                format,
                width,
                height,
            ));
        }

        if self.y_axis_renderer.is_none() {
            self.y_axis_renderer = Some(YAxisRenderer::new(
                device.clone(),
                queue.clone(),
                format,
                width,
                height,
            ));
        }
    }

    /// Render a frame
    pub async fn render(&mut self) -> RenderResult<()> {
        // Check if rendering is needed
        if !self.data_store.is_dirty() {
            // log::debug!("Skipping render - data store not dirty");
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

        // Clear pass
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Explicitly drop the render pass to end it
            drop(render_pass);
        }

        // Render based on current chart type
        match self.data_store.chart_type {
            data_manager::ChartType::Line => {
                if let Some(ref mut plot_renderer) = self.plot_renderer {
                    plot_renderer.render(&mut encoder, &view, &self.data_store);
                }
            }
            data_manager::ChartType::Candlestick => {
                if let Some(ref mut candlestick_renderer) = self.candlestick_renderer {
                    candlestick_renderer.render(
                        &mut encoder,
                        &view,
                        &self.data_store,
                        &self.device,
                        &self.queue,
                    );
                }
            }
        }

        // Render axes
        if let Some(ref mut x_axis) = self.x_axis_renderer {
            x_axis.render(
                &mut encoder,
                &view,
                &self.data_store,
                &self.device,
                &self.queue,
            );
        }

        if let Some(ref mut y_axis) = self.y_axis_renderer {
            y_axis.render(
                &mut encoder,
                &view,
                &self.data_store,
                &self.device,
                &self.queue,
            );
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Present the frame
        output.present();

        // Mark as clean after successful render
        self.data_store.mark_clean();

        Ok(())
    }

    /// Update chart type
    pub fn set_chart_type(&mut self, chart_type: data_manager::ChartType) {
        log::info!(
            "set_chart_type called: {:?} -> {:?}",
            self.data_store.chart_type,
            chart_type
        );
        let old_type = self.data_store.chart_type.clone();
        self.data_store.chart_type = chart_type;

        self.setup_renderers();

        // Mark data store as dirty to trigger a render
        self.data_store.mark_dirty();

        log::info!(
            "Chart type changed from {:?} to {:?} - marked dirty",
            old_type,
            chart_type
        );
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
        // Only count the active renderer based on chart type
        let mut draw_calls = match self.data_store.chart_type {
            data_manager::ChartType::Line => {
                if self.plot_renderer.is_some() {
                    1
                } else {
                    0
                }
            }
            data_manager::ChartType::Candlestick => {
                if self.candlestick_renderer.is_some() {
                    1
                } else {
                    0
                }
            }
        };
        if self.x_axis_renderer.is_some() {
            draw_calls += 1;
        }
        if self.y_axis_renderer.is_some() {
            draw_calls += 1;
        }

        RenderStats {
            frame_time_ms: 0.0,
            draw_calls,
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
        // For now, always return true
        // In the future, we could track dirty state
        true
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
