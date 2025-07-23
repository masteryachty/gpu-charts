//! Pure GPU rendering engine for GPU Charts
//! Implements Phase 3 optimizations for extreme performance

pub mod render_engine;
pub mod pipeline_builder;
pub mod mesh_builder;
pub mod drawables;
pub mod calcables;
pub mod shaders;
pub mod charts;

use std::rc::Rc;
use std::cell::RefCell;
use wgpu::{Device, Queue, TextureView, CommandEncoder};
use shared_types::{RenderStats, GpuChartsError, GpuChartsResult};
use config_system::GpuChartsConfig;
use data_manager::DataStore;

pub use render_engine::RenderEngine;
pub use drawables::plot::RenderListener;
pub use drawables::{plot::PlotRenderer, x_axis::XAxisRenderer, y_axis::YAxisRenderer, candlestick::CandlestickRenderer};
pub use calcables::{min_max::calculate_min_max_y, candle_aggregator::CandleAggregator};

/// Re-export error types
pub type RenderError = GpuChartsError;
pub type RenderResult<T> = GpuChartsResult<T>;

/// Main renderer that orchestrates all rendering operations
pub struct Renderer {
    pub render_engine: Rc<RefCell<RenderEngine>>,
    config: GpuChartsConfig,
    data_store: Rc<RefCell<DataStore>>,
    
    // Specific renderers
    plot_renderer: Option<Box<dyn RenderListener>>,
    x_axis_renderer: Option<XAxisRenderer>,
    y_axis_renderer: Option<YAxisRenderer>,
}

impl Renderer {
    /// Create a new renderer
    pub async fn new(
        render_engine: Rc<RefCell<RenderEngine>>,
        config: GpuChartsConfig,
        data_store: Rc<RefCell<DataStore>>,
    ) -> RenderResult<Self> {
        let mut renderer = Self {
            render_engine,
            config,
            data_store,
            plot_renderer: None,
            x_axis_renderer: None,
            y_axis_renderer: None,
        };
        
        // Set up initial renderers
        renderer.setup_renderers();
        
        Ok(renderer)
    }
    
    /// Setup renderers based on chart type
    pub fn setup_renderers(&mut self) {
        let format = self.render_engine.borrow().config.format;
        let chart_type = self.data_store.borrow().chart_type;
        
        log::info!("Setting up renderers for chart type: {chart_type:?}");
        
        // Create plot renderer based on chart type
        self.plot_renderer = match chart_type {
            data_manager::ChartType::Line => Some(Box::new(PlotRenderer::new(
                self.render_engine.clone(),
                format,
                self.data_store.clone(),
            ))),
            data_manager::ChartType::Candlestick => Some(Box::new(CandlestickRenderer::new(
                self.render_engine.clone(),
                format,
                self.data_store.clone(),
            ))),
        };
        
        // Create axis renderers
        self.x_axis_renderer = Some(XAxisRenderer::new(
            self.render_engine.clone(),
            format,
            self.data_store.clone(),
        ));
        
        self.y_axis_renderer = Some(YAxisRenderer::new(
            self.render_engine.clone(),
            format,
            self.data_store.clone(),
        ));
    }

    /// Get the render engine
    pub fn engine(&self) -> Rc<RefCell<RenderEngine>> {
        self.render_engine.clone()
    }

    /// Render a frame
    pub async fn render(&mut self) -> RenderResult<()> {
        // Check if rendering is needed
        if !self.data_store.borrow().is_dirty() {
            return Ok(());
        }
        
        // Get current texture
        let engine_borrow = self.render_engine.borrow();
        let output = engine_borrow.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create command encoder
        let mut encoder = engine_borrow.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        // Clear pass
        {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        }
        
        // Render plot
        if let Some(ref mut plot_renderer) = self.plot_renderer {
            plot_renderer.on_render(
                &engine_borrow.queue,
                &engine_borrow.device,
                &mut encoder,
                &view,
                self.data_store.clone(),
            );
        }
        
        // Render axes
        if let Some(ref mut x_axis) = self.x_axis_renderer {
            x_axis.on_render(
                &engine_borrow.queue,
                &engine_borrow.device,
                &mut encoder,
                &view,
                self.data_store.clone(),
            );
        }
        
        if let Some(ref mut y_axis) = self.y_axis_renderer {
            y_axis.on_render(
                &engine_borrow.queue,
                &engine_borrow.device,
                &mut encoder,
                &view,
                self.data_store.clone(),
            );
        }
        
        // Submit commands
        engine_borrow.queue.submit(std::iter::once(encoder.finish()));
        drop(engine_borrow); // Drop the borrow before present
        output.present();
        
        // Mark as clean after successful render
        self.data_store.borrow_mut().mark_clean();
        
        Ok(())
    }
    
    /// Update chart type
    pub fn set_chart_type(&mut self, chart_type: data_manager::ChartType) {
        self.data_store.borrow_mut().chart_type = chart_type;
        self.setup_renderers();
    }
    
    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        self.render_engine.borrow_mut().resized(width, height);
        self.data_store.borrow_mut().resized(width, height);
    }


    /// Get current statistics
    pub fn get_stats(&self) -> RenderStats {
        let mut draw_calls = 0;
        if self.plot_renderer.is_some() { draw_calls += 1; }
        if self.x_axis_renderer.is_some() { draw_calls += 1; }
        if self.y_axis_renderer.is_some() { draw_calls += 1; }
        
        RenderStats {
            frame_time_ms: 0.0,
            draw_calls,
            vertices_rendered: 0,
            gpu_memory_used: 0,
        }
    }
}

/// Trait for chart-specific renderers
pub trait ChartRenderer: Send + Sync {
    /// Render the chart
    fn render(&mut self, encoder: &mut CommandEncoder, view: &TextureView, device: &Device, queue: &Queue);
    
    /// Handle resize
    fn resize(&mut self, width: u32, height: u32);
    
    /// Get renderer name
    fn name(&self) -> &str;
}