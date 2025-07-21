//! Pure GPU rendering engine for charts
//!
//! This crate is a configuration-driven renderer that accepts GPU buffers
//! from the data manager and renders various chart types with high performance.

use gpu_charts_shared::{ChartConfiguration, ChartType, DataHandle, Result, VisualConfig};
use std::collections::HashMap;
use std::sync::Arc;

pub mod chart_renderers;
pub mod engine;
pub mod overlays;
pub mod pipeline;
pub mod culling;
pub mod lod;
pub mod config;

use chart_renderers::ChartRenderer;
use engine::RenderEngine;
use overlays::OverlayRenderer;

/// GPU buffer set passed from data manager
pub struct GpuBufferSet {
    pub buffers: HashMap<String, Vec<wgpu::Buffer>>,
    pub metadata: gpu_charts_shared::DataMetadata,
}

/// Render context passed to renderers
pub struct RenderContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub viewport: Viewport,
    pub visual_config: &'a VisualConfig,
    pub frame_time: f32,
}

/// Viewport information
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub zoom_level: f32,
    pub time_range: gpu_charts_shared::TimeRange,
}

/// Main renderer that manages all rendering operations
pub struct Renderer {
    engine: RenderEngine,
    active_renderer: Option<Box<dyn ChartRenderer>>,
    overlay_renderers: Vec<Box<dyn OverlayRenderer>>,
    current_config: Option<ChartConfiguration>,
    viewport: Viewport,
    buffer_handles: HashMap<uuid::Uuid, Arc<GpuBufferSet>>,
    performance_metrics: PerformanceMetrics,
}

/// Performance metrics for monitoring
#[derive(Default, Debug)]
pub struct PerformanceMetrics {
    pub frame_time_ms: f32,
    pub gpu_time_ms: f32,
    pub cpu_time_ms: f32,
    pub draw_calls: u32,
    pub vertices_rendered: u64,
    pub triangles_rendered: u64,
}

impl Renderer {
    /// Create a new renderer instance with shared device/queue
    pub fn new_with_device(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let engine = RenderEngine::new_with_device(device, queue, surface, width, height)?;
        
        Ok(Self {
            engine,
            active_renderer: None,
            overlay_renderers: Vec::new(),
            current_config: None,
            viewport: Viewport {
                x: 0.0,
                y: 0.0,
                width: width as f32,
                height: height as f32,
                zoom_level: 1.0,
                time_range: gpu_charts_shared::TimeRange::new(0, 1000),
            },
            buffer_handles: HashMap::new(),
            performance_metrics: PerformanceMetrics::default(),
        })
    }

    /// Update the render configuration
    pub fn update_config(&mut self, config: ChartConfiguration) -> Result<()> {
        // Check if we need to recreate the renderer
        let needs_new_renderer = match &self.current_config {
            None => true,
            Some(current) => current.chart_type != config.chart_type,
        };

        if needs_new_renderer {
            self.create_chart_renderer(&config)?;
        }

        // Update overlays
        self.update_overlays(&config)?;
        
        // Update visual config on existing renderer
        if let Some(renderer) = &mut self.active_renderer {
            renderer.update_visual_config(&config.visual_config);
        }

        // Store config
        self.current_config = Some(config);

        Ok(())
    }

    /// Register GPU buffer handles from data manager
    pub fn register_buffer_set(&mut self, handle: DataHandle, buffers: Arc<GpuBufferSet>) {
        self.buffer_handles.insert(handle.id, buffers);
    }
    
    /// Remove buffer set when data handle is released
    pub fn unregister_buffer_set(&mut self, handle_id: &uuid::Uuid) {
        self.buffer_handles.remove(handle_id);
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        // Get active buffer sets for current config
        let buffer_sets = self.get_active_buffer_sets();
        
        if let Some(renderer) = &mut self.active_renderer {
            self.engine.render(
                renderer.as_mut(),
                &mut self.overlay_renderers,
                &buffer_sets,
                &self.viewport,
                &self.current_config.as_ref().unwrap().visual_config,
                &mut self.performance_metrics,
            )?;
        }
        
        self.performance_metrics.frame_time_ms = start_time.elapsed().as_secs_f32() * 1000.0;
        Ok(())
    }

    /// Handle resize events
    pub fn resize(&mut self, width: u32, height: u32) {
        self.engine.resize(width, height);
        self.viewport.width = width as f32;
        self.viewport.height = height as f32;

        if let Some(renderer) = &mut self.active_renderer {
            renderer.on_resize(width, height);
        }

        for overlay in &mut self.overlay_renderers {
            overlay.as_mut().on_resize(width, height);
        }
    }
    
    /// Update viewport (pan/zoom)
    pub fn update_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
        
        // Notify renderers of viewport change
        if let Some(renderer) = &mut self.active_renderer {
            renderer.on_viewport_change(&viewport);
        }
    }

    /// Get performance statistics
    pub fn get_performance_metrics(&self) -> &PerformanceMetrics {
        &self.performance_metrics
    }
    
    /// Get detailed stats as JSON
    pub fn get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "performance": {
                "frame_time_ms": self.performance_metrics.frame_time_ms,
                "gpu_time_ms": self.performance_metrics.gpu_time_ms,
                "cpu_time_ms": self.performance_metrics.cpu_time_ms,
                "draw_calls": self.performance_metrics.draw_calls,
                "vertices": self.performance_metrics.vertices_rendered,
                "triangles": self.performance_metrics.triangles_rendered,
            },
            "engine": self.engine.get_stats(),
            "viewport": {
                "x": self.viewport.x,
                "y": self.viewport.y,
                "width": self.viewport.width,
                "height": self.viewport.height,
                "zoom": self.viewport.zoom_level,
            },
            "buffers": self.buffer_handles.len(),
        })
    }
}

impl Renderer {
    fn create_chart_renderer(&mut self, config: &ChartConfiguration) -> Result<()> {
        use chart_renderers::*;
        
        let renderer: Box<dyn ChartRenderer> = match config.chart_type {
            ChartType::Line => Box::new(LineChartRenderer::new(
                self.engine.device(),
                &config.visual_config,
            )?),
            ChartType::Candlestick => Box::new(CandlestickRenderer::new(
                self.engine.device(),
                &config.visual_config,
            )?),
            ChartType::Area => Box::new(AreaChartRenderer::new(
                self.engine.device(),
                &config.visual_config,
            )?),
            ChartType::Bar => Box::new(BarChartRenderer::new(
                self.engine.device(),
                &config.visual_config,
            )?),
        };

        self.active_renderer = Some(renderer);
        Ok(())
    }

    fn update_overlays(&mut self, config: &ChartConfiguration) -> Result<()> {
        use overlays::*;
        
        self.overlay_renderers.clear();

        for overlay_config in &config.overlays {
            let overlay: Box<dyn OverlayRenderer> = match overlay_config.overlay_type.as_str() {
                "volume" => Box::new(VolumeOverlay::new(
                    self.engine.device(),
                    &config.visual_config,
                )?),
                "moving_average" => Box::new(MovingAverageOverlay::new(
                    self.engine.device(),
                    &config.visual_config,
                    overlay_config.parameters.clone(),
                )?),
                _ => continue, // Skip unknown overlay types
            };
            
            self.overlay_renderers.push(overlay);
        }

        Ok(())
    }
    
    fn get_active_buffer_sets(&self) -> Vec<Arc<GpuBufferSet>> {
        // Get buffer sets for active data handles
        if let Some(config) = &self.current_config {
            config.data_handles.iter()
                .filter_map(|handle| self.buffer_handles.get(&handle.id))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpu_charts_shared::{TimeRange, DataMetadata};
    
    #[test]
    fn test_viewport_creation() {
        let viewport = Viewport {
            x: 10.0,
            y: 20.0,
            width: 800.0,
            height: 600.0,
            zoom_level: 1.5,
            time_range: TimeRange::new(1000, 2000),
        };
        
        assert_eq!(viewport.x, 10.0);
        assert_eq!(viewport.y, 20.0);
        assert_eq!(viewport.width, 800.0);
        assert_eq!(viewport.height, 600.0);
        assert_eq!(viewport.zoom_level, 1.5);
        assert_eq!(viewport.time_range.start, 1000);
        assert_eq!(viewport.time_range.end, 2000);
    }
    
    #[test]
    fn test_gpu_buffer_set() {
        let mut buffers = HashMap::new();
        buffers.insert("test".to_string(), vec![]);
        
        let buffer_set = GpuBufferSet {
            buffers,
            metadata: DataMetadata {
                symbol: "TEST".to_string(),
                time_range: TimeRange::new(0, 100),
                columns: vec!["test".to_string()],
                row_count: 100,
                byte_size: 400,
                creation_time: 1234567890,
            },
        };
        
        assert_eq!(buffer_set.metadata.symbol, "TEST");
        assert_eq!(buffer_set.metadata.row_count, 100);
        assert!(buffer_set.buffers.contains_key("test"));
    }
    
    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::default();
        
        assert_eq!(metrics.frame_time_ms, 0.0);
        assert_eq!(metrics.draw_calls, 0);
        
        metrics.frame_time_ms = 16.67;
        metrics.draw_calls = 5;
        metrics.vertices_rendered = 10000;
        
        assert!(metrics.frame_time_ms > 16.0);
        assert_eq!(metrics.draw_calls, 5);
        assert_eq!(metrics.vertices_rendered, 10000);
    }
}
