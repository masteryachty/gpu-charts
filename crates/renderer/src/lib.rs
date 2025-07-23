//! Pure GPU rendering engine for GPU Charts
//! Implements Phase 3 optimizations for extreme performance

pub mod render_engine;
pub mod pipeline_builder;
pub mod mesh_builder;
pub mod drawables;
pub mod calcables;
pub mod shaders;
pub mod charts;

use std::sync::Arc;
use wgpu::{Device, Queue, Surface, TextureView, CommandEncoder};
use shared_types::{ChartType, ChartConfiguration, RenderStats};
use config_system::GpuChartsConfig;
use thiserror::Error;

pub use render_engine::RenderEngine;
pub use drawables::plot::RenderListener;
pub use drawables::{plot::PlotRenderer, x_axis::XAxisRenderer, y_axis::YAxisRenderer, candlestick::CandlestickRenderer};
pub use calcables::{min_max::calculate_min_max_y, candle_aggregator::CandleAggregator};

#[derive(Error, Debug)]
pub enum RenderError {
    #[error("GPU error: {0}")]
    GpuError(String),
    
    #[error("Surface error: {0}")]
    SurfaceError(#[from] wgpu::SurfaceError),
    
    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

/// Main renderer that orchestrates all rendering operations
pub struct Renderer {
    pub render_engine: RenderEngine,
    config: GpuChartsConfig,
    chart_renderers: Vec<Box<dyn ChartRenderer>>,
}

impl Renderer {
    /// Get the render engine
    pub fn engine(&self) -> &RenderEngine {
        &self.render_engine
    }
    
    /// Get mutable render engine
    pub fn engine_mut(&mut self) -> &mut RenderEngine {
        &mut self.render_engine
    }

    /// Update chart configuration
    pub fn update_config(&mut self, chart_config: ChartConfiguration) -> Result<(), RenderError> {
        // Clear existing renderers
        self.chart_renderers.clear();
        
        // Create new renderers based on chart type
        match chart_config.chart_type {
            ChartType::Line => {
                // Add line chart renderer
                // self.chart_renderers.push(Box::new(LineChartRenderer::new(...)));
            }
            ChartType::Candlestick => {
                // Add candlestick renderer
                // self.chart_renderers.push(Box::new(CandlestickRenderer::new(...)));
            }
            ChartType::Bar => {
                // Add bar chart renderer
            }
            ChartType::Area => {
                // Add area chart renderer
            }
        }
        
        Ok(())
    }


    /// Get current statistics
    pub fn get_stats(&self) -> RenderStats {
        RenderStats {
            frame_time_ms: 0.0,
            draw_calls: self.chart_renderers.len() as u32,
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