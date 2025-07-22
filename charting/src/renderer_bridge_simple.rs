//! Simplified bridge demonstrating the new modular renderer architecture
//! This version works with the existing charting codebase

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::controls::canvas_controller::CanvasController;
use crate::line_graph::LineGraph;

/// Configuration for chart rendering
#[derive(Clone)]
pub struct ChartConfig {
    pub chart_type: String,
    pub background_color: [f32; 4],
    pub grid_color: [f32; 4],
    pub text_color: [f32; 4],
    pub show_grid: bool,
    pub show_axes: bool,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            chart_type: "line".to_string(),
            background_color: [0.0, 0.0, 0.0, 1.0],
            grid_color: [0.2, 0.2, 0.2, 1.0],
            text_color: [1.0, 1.0, 1.0, 1.0],
            show_grid: true,
            show_axes: true,
        }
    }
}

/// Performance metrics for monitoring
#[derive(Default)]
pub struct PerformanceMetrics {
    pub frame_time_ms: f32,
    pub draw_calls: u32,
    pub vertices_rendered: u64,
}

/// Simplified bridge that wraps existing LineGraph with new architecture concepts
pub struct RendererBridge {
    line_graph: Rc<RefCell<LineGraph>>,
    canvas_controller: CanvasController,
    config: ChartConfig,
    metrics: PerformanceMetrics,
    width: u32,
    height: u32,
}

impl RendererBridge {
    /// Create a new renderer bridge
    pub async fn new(canvas: HtmlCanvasElement, width: u32, height: u32) -> Result<Self, JsValue> {
        // Use existing LineGraph initialization
        let line_graph = LineGraph::new(width, height, canvas)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create LineGraph: {:?}", e)))?;

        let line_graph = Rc::new(RefCell::new(line_graph));

        // Create canvas controller
        let data_store = line_graph.borrow().data_store.clone();
        let engine = line_graph.borrow().engine.clone();
        let canvas_controller = CanvasController::new(data_store, engine);

        Ok(Self {
            line_graph,
            canvas_controller,
            config: ChartConfig::default(),
            metrics: PerformanceMetrics::default(),
            width,
            height,
        })
    }

    /// Update chart configuration
    pub fn update_config(&mut self, config: ChartConfig) -> Result<(), JsValue> {
        self.config = config;
        // In a real implementation, this would update renderer settings
        Ok(())
    }

    /// Render a frame
    pub async fn render(&mut self) -> Result<(), JsValue> {
        let start = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);

        // Render using existing LineGraph
        self.line_graph
            .borrow()
            .render()
            .await
            .map_err(|e| JsValue::from_str(&format!("Render failed: {:?}", e)))?;

        // Update metrics
        let end = web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0);
        self.metrics.frame_time_ms = (end - start) as f32;

        Ok(())
    }

    /// Handle resize
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.line_graph.borrow_mut().resized(width, height);
    }

    /// Handle mouse wheel event
    pub async fn handle_mouse_wheel(&mut self, delta: f64, x: f64, y: f64) -> Result<(), JsValue> {
        use crate::events::{MouseScrollDelta, PhysicalPosition, TouchPhase, WindowEvent};

        let window_event = WindowEvent::MouseWheel {
            delta: MouseScrollDelta::PixelDelta(PhysicalPosition::new(x, delta)),
            phase: TouchPhase::Moved,
        };
        self.canvas_controller.handle_cursor_event(window_event);
        Ok(())
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> String {
        serde_json::json!({
            "frame_time_ms": self.metrics.frame_time_ms,
            "draw_calls": self.metrics.draw_calls,
            "vertices_rendered": self.metrics.vertices_rendered,
        })
        .to_string()
    }

    /// Get detailed stats
    pub fn get_stats(&self) -> String {
        serde_json::json!({
            "performance": {
                "frame_time_ms": self.metrics.frame_time_ms,
                "fps": if self.metrics.frame_time_ms > 0.0 {
                    1000.0 / self.metrics.frame_time_ms
                } else {
                    0.0
                },
            },
            "viewport": {
                "width": self.width,
                "height": self.height,
            },
            "config": {
                "chart_type": &self.config.chart_type,
                "show_grid": self.config.show_grid,
                "show_axes": self.config.show_axes,
            }
        })
        .to_string()
    }
}
