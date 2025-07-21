//! Pure GPU rendering engine for charts
//!
//! This crate is a configuration-driven renderer that accepts GPU buffers
//! from the data manager and renders various chart types with high performance.

use gpu_charts_shared::{ChartConfiguration, ChartType, DataHandle, Error, Result};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

pub mod chart_renderers;
pub mod engine;
pub mod overlays;
pub mod pipeline;

use chart_renderers::{CandlestickRenderer, ChartRenderer, LineChartRenderer};
use engine::RenderEngine;

/// Main renderer that manages all rendering operations
#[wasm_bindgen]
pub struct Renderer {
    engine: RenderEngine,
    active_renderer: Option<Box<dyn ChartRenderer>>,
    overlay_renderers: Vec<Box<dyn overlays::OverlayRenderer>>,
    current_config: Option<ChartConfiguration>,
}

#[wasm_bindgen]
impl Renderer {
    /// Create a new renderer instance
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_id: &str) -> Result<Renderer> {
        console_error_panic_hook::set_once();

        let engine = RenderEngine::new(canvas_id).await?;

        Ok(Self {
            engine,
            active_renderer: None,
            overlay_renderers: Vec::new(),
            current_config: None,
        })
    }

    /// Update the render configuration
    #[wasm_bindgen]
    pub fn update_config(&mut self, config_json: &str) -> Result<()> {
        let config: ChartConfiguration = serde_json::from_str(config_json)
            .map_err(|e| Error::InvalidConfiguration(e.to_string()))?;

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

        // Store config
        self.current_config = Some(config);

        Ok(())
    }

    /// Set GPU buffer handles for rendering
    #[wasm_bindgen]
    pub fn set_data_handles(&mut self, handles_json: &str) -> Result<()> {
        let handles: Vec<DataHandle> = serde_json::from_str(handles_json)
            .map_err(|e| Error::InvalidConfiguration(e.to_string()))?;

        // TODO: Map handles to actual GPU buffers
        // This will involve coordination with the data manager

        Ok(())
    }

    /// Render a frame
    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<()> {
        if let Some(renderer) = &mut self.active_renderer {
            self.engine.render(renderer, &self.overlay_renderers)?;
        }
        Ok(())
    }

    /// Handle resize events
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.engine.resize(width, height);

        if let Some(renderer) = &mut self.active_renderer {
            renderer.on_resize(width, height);
        }

        for overlay in &mut self.overlay_renderers {
            overlay.on_resize(width, height);
        }
    }

    /// Get performance statistics
    #[wasm_bindgen]
    pub fn get_stats(&self) -> String {
        self.engine.get_stats()
    }
}

impl Renderer {
    fn create_chart_renderer(&mut self, config: &ChartConfiguration) -> Result<()> {
        let renderer: Box<dyn ChartRenderer> = match config.chart_type {
            ChartType::Line => Box::new(LineChartRenderer::new(&self.engine)?),
            ChartType::Candlestick => Box::new(CandlestickRenderer::new(&self.engine)?),
            ChartType::Area => {
                // TODO: Implement area chart
                Box::new(LineChartRenderer::new(&self.engine)?)
            }
            ChartType::Bar => {
                // TODO: Implement bar chart
                Box::new(LineChartRenderer::new(&self.engine)?)
            }
        };

        self.active_renderer = Some(renderer);
        Ok(())
    }

    fn update_overlays(&mut self, config: &ChartConfiguration) -> Result<()> {
        // TODO: Implement overlay management
        self.overlay_renderers.clear();

        for overlay_config in &config.overlays {
            // Create appropriate overlay renderer based on type
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here
}
