//! Multi-renderer pipeline system for combining multiple render types in a single chart
//!
//! This module provides a flexible system for combining multiple renderers (lines, candles, bars, etc.)
//! within a single chart view. It manages the execution order, resource sharing, and coordination
//! between different renderer types.

use std::rc::Rc;
use wgpu::{CommandEncoder, Device, Queue, TextureView};

use crate::RenderResult;
use data_manager::DataStore;

/// Configuration for how renderers should be combined
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOrder {
    /// Render in the order they were added
    Sequential,
    /// Render background elements first (bars, areas), then foreground (lines, candles)
    BackgroundToForeground,
    /// Custom order based on renderer priority
    Priority,
}

/// Trait for renderers that can be added to a MultiRenderer
#[cfg(not(target_arch = "wasm32"))]
pub trait MultiRenderable: Send + Sync {
    /// Render the component
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        device: &Device,
        queue: &Queue,
    );

    /// Get the renderer's name for debugging and identification
    fn name(&self) -> &str;

    /// Get the render priority (lower numbers render first)
    /// Default is 100, background elements should use 0-50, foreground 100-150
    fn priority(&self) -> u32 {
        100
    }

    /// Whether this renderer should clear the render target before drawing
    /// Only the first renderer in the pipeline should return true
    fn should_clear(&self) -> bool {
        false
    }

    /// Called when the render surface is resized
    fn resize(&mut self, _width: u32, _height: u32) {}

    /// Check if the renderer is ready to render
    fn is_ready(&self) -> bool {
        true
    }

    /// Check if this renderer has compute capabilities
    fn has_compute(&self) -> bool {
        false // Default to no compute
    }

    /// Run compute passes (called before min/max calculation)
    fn compute(
        &mut self,
        _encoder: &mut CommandEncoder,
        _data_store: &DataStore,
        _device: &Device,
        _queue: &Queue,
    ) {
        // Default implementation does nothing
    }
}

/// Trait for renderers that can be added to a MultiRenderer (WASM version without Send+Sync)
pub trait MultiRenderable {
    /// Render the component
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        device: &Device,
        queue: &Queue,
    );

    /// Get the renderer's name for debugging and identification
    fn name(&self) -> &str;

    /// Get the render priority (lower numbers render first)
    /// Default is 100, background elements should use 0-50, foreground 100-150
    fn priority(&self) -> u32 {
        100
    }

    /// Whether this renderer should clear the render target before drawing
    /// Only the first renderer in the pipeline should return true
    fn should_clear(&self) -> bool {
        false
    }

    /// Handle resize events
    fn resize(&mut self, _width: u32, _height: u32) {}

    /// Check if the renderer is ready to render
    fn is_ready(&self) -> bool {
        true
    }

    /// Check if this renderer has compute capabilities
    fn has_compute(&self) -> bool {
        false // Default to no compute
    }

    /// Run compute passes (called before min/max calculation)
    fn compute(
        &mut self,
        _encoder: &mut CommandEncoder,
        _data_store: &DataStore,
        _device: &Device,
        _queue: &Queue,
    ) {
        // Default implementation does nothing
    }
}

/// Wrapper to adapt existing renderers to the MultiRenderable trait
pub struct RendererAdapter<T> {
    renderer: T,
    _name: String,
    priority: u32,
    should_clear: bool,
}

impl<T> RendererAdapter<T> {
    pub fn new(renderer: T, name: impl Into<String>) -> Self {
        Self {
            renderer,
            _name: name.into(),
            priority: 100,
            should_clear: false,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_clear(mut self) -> Self {
        self.should_clear = true;
        self
    }

    /// Get a reference to the inner renderer
    pub fn inner(&self) -> &T {
        &self.renderer
    }

    /// Get a mutable reference to the inner renderer
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.renderer
    }
}

/// Multi-renderer pipeline that combines multiple chart renderers
pub struct MultiRenderer {
    device: Rc<Device>,
    queue: Rc<Queue>,
    renderers: Vec<Box<dyn MultiRenderable>>,
    render_order: RenderOrder,
    _format: wgpu::TextureFormat,
}

impl MultiRenderer {
    /// Create a new MultiRenderer
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, format: wgpu::TextureFormat) -> Self {
        Self {
            device,
            queue,
            renderers: Vec::new(),
            render_order: RenderOrder::Sequential,
            _format: format,
        }
    }

    /// Set the render order strategy
    pub fn with_render_order(mut self, order: RenderOrder) -> Self {
        self.render_order = order;
        self
    }

    /// Add a renderer to the pipeline
    pub fn add_renderer(&mut self, renderer: Box<dyn MultiRenderable>) {
        log::debug!("MultiRenderer: Adding renderer '{}'", renderer.name());
        self.renderers.push(renderer);
        self.sort_renderers();
    }

    /// Remove all renderers
    pub fn clear_renderers(&mut self) {
        log::debug!("MultiRenderer: Clearing all renderers");
        self.renderers.clear();
    }

    /// Get the number of active renderers
    pub fn renderer_count(&self) -> usize {
        self.renderers.len()
    }

    /// Get renderer names for debugging
    pub fn get_renderer_names(&self) -> Vec<&str> {
        self.renderers.iter().map(|r| r.name()).collect()
    }

    /// Sort renderers based on the current render order strategy
    fn sort_renderers(&mut self) {
        match self.render_order {
            RenderOrder::Sequential => {
                // No sorting needed, keep insertion order
            }
            RenderOrder::BackgroundToForeground | RenderOrder::Priority => {
                // Sort by priority (stable sort preserves insertion order for equal priorities)
                self.renderers.sort_by_key(|r| r.priority());
            }
        }
    }

    /// Run compute passes for all renderers that support it
    /// This should be called BEFORE calculating min/max bounds
    pub fn run_compute_passes(
        &mut self,
        encoder: &mut CommandEncoder,
        data_store: &DataStore,
        device: &Device,
        queue: &Queue,
    ) {
        log::debug!(
            "[MultiRenderer] Running compute passes for {} renderers",
            self.renderers.len()
        );

        for renderer in &mut self.renderers {
            if renderer.has_compute() {
                log::debug!(
                    "[MultiRenderer] Running compute pass for '{}'",
                    renderer.name()
                );
                renderer.compute(encoder, data_store, device, queue);
            }
        }
    }

    /// Render all components in the pipeline
    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
    ) -> RenderResult<()> {
        if self.renderers.is_empty() {
            log::debug!("MultiRenderer: No renderers to execute");
            return Ok(());
        }

        // Check if any renderer should clear
        let needs_clear = self.renderers.iter().any(|r| r.should_clear());

        // If no renderer wants to clear, we'll do it ourselves on the first render
        if needs_clear || !self.renderers.is_empty() {
            // Clear pass if needed
            let should_clear_here = needs_clear || self.renderers.iter().all(|r| !r.should_clear());
            if should_clear_here {
                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("MultiRenderer Clear Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
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
                drop(render_pass);
            }
        }

        // Execute each renderer in order
        let renderer_count = self.renderers.len();
        for (index, renderer) in self.renderers.iter_mut().enumerate() {
            if !renderer.is_ready() {
                log::debug!(
                    "MultiRenderer: Skipping renderer '{}' (not ready)",
                    renderer.name()
                );
                continue;
            }

            log::trace!(
                "MultiRenderer: Executing renderer {} of {}: '{}'",
                index + 1,
                renderer_count,
                renderer.name()
            );

            renderer.render(encoder, view, data_store, &self.device, &self.queue);
        }

        Ok(())
    }

    /// Handle resize for all renderers
    pub fn resize(&mut self, width: u32, height: u32) {
        log::debug!("MultiRenderer: Resizing to {width}x{height}");
        for renderer in &mut self.renderers {
            renderer.resize(width, height);
        }
    }
}

// Implement MultiRenderable for common renderer types

impl MultiRenderable for crate::PlotRenderer {
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        _device: &Device,
        _queue: &Queue,
    ) {
        self.render(encoder, view, data_store);
    }

    fn name(&self) -> &str {
        "PlotRenderer"
    }

    fn priority(&self) -> u32 {
        100 // Default priority for lines
    }
}

/// Wrapper for PlotRenderer with configurable data columns
pub struct ConfigurablePlotRenderer {
    renderer: crate::PlotRenderer,
    name: String,
    _data_columns: Vec<(String, String)>,
}

impl ConfigurablePlotRenderer {
    pub fn new(
        device: Rc<Device>,
        queue: Rc<Queue>,
        format: wgpu::TextureFormat,
        name: String,
        data_columns: Vec<(String, String)>,
    ) -> Self {
        let mut renderer = crate::PlotRenderer::new(device, queue, format);
        renderer.set_data_filter(Some(data_columns.clone()));

        Self {
            renderer,
            name,
            _data_columns: data_columns,
        }
    }
}

impl MultiRenderable for ConfigurablePlotRenderer {
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        _device: &Device,
        _queue: &Queue,
    ) {
        self.renderer.render(encoder, view, data_store);
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u32 {
        100 // Default priority for lines
    }
}

impl MultiRenderable for crate::CandlestickRenderer {
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        device: &Device,
        queue: &Queue,
    ) {
        self.render(encoder, view, data_store, device, queue);
    }

    fn name(&self) -> &str {
        "CandlestickRenderer"
    }

    fn priority(&self) -> u32 {
        50 // Render candles before lines
    }
}

impl MultiRenderable for crate::XAxisRenderer {
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        device: &Device,
        queue: &Queue,
    ) {
        crate::XAxisRenderer::render(self, encoder, view, data_store, device, queue);
    }

    fn name(&self) -> &str {
        "XAxisRenderer"
    }

    fn priority(&self) -> u32 {
        150 // Render axes on top
    }

    // fn resize uses default implementation
}

impl MultiRenderable for crate::YAxisRenderer {
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        device: &Device,
        queue: &Queue,
    ) {
        crate::YAxisRenderer::render(self, encoder, view, data_store, device, queue);
    }

    fn name(&self) -> &str {
        "YAxisRenderer"
    }

    fn priority(&self) -> u32 {
        150 // Render axes on top
    }

    // fn resize uses default implementation
}

// Builder pattern for convenient MultiRenderer construction
pub struct MultiRendererBuilder {
    device: Rc<Device>,
    queue: Rc<Queue>,
    format: wgpu::TextureFormat,
    renderers: Vec<Box<dyn MultiRenderable>>,
    render_order: RenderOrder,
}

impl MultiRendererBuilder {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>, format: wgpu::TextureFormat) -> Self {
        Self {
            device,
            queue,
            format,
            renderers: Vec::new(),
            render_order: RenderOrder::Sequential,
        }
    }

    pub fn with_render_order(mut self, order: RenderOrder) -> Self {
        self.render_order = order;
        self
    }

    pub fn add_renderer(mut self, renderer: Box<dyn MultiRenderable>) -> Self {
        self.renderers.push(renderer);
        self
    }

    pub fn add_plot_renderer(mut self) -> Self {
        let renderer =
            crate::PlotRenderer::new(self.device.clone(), self.queue.clone(), self.format);
        self.renderers.push(Box::new(renderer));
        self
    }

    pub fn add_candlestick_renderer(mut self) -> Self {
        let renderer =
            crate::CandlestickRenderer::new(self.device.clone(), self.queue.clone(), self.format);
        self.renderers.push(Box::new(renderer));
        self
    }

    pub fn add_x_axis_renderer(mut self, width: u32, height: u32) -> Self {
        let renderer = crate::XAxisRenderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.format,
            width,
            height,
        );
        self.renderers.push(Box::new(renderer));
        self
    }

    pub fn add_y_axis_renderer(mut self, width: u32, height: u32) -> Self {
        let renderer = crate::YAxisRenderer::new(
            self.device.clone(),
            self.queue.clone(),
            self.format,
            width,
            height,
        );
        self.renderers.push(Box::new(renderer));
        self
    }

    pub fn build(self) -> MultiRenderer {
        log::debug!("[MULTIRENDER] build render pipeline");
        let mut renderer = MultiRenderer::new(self.device, self.queue, self.format);
        renderer.render_order = self.render_order;
        for r in self.renderers {
            renderer.add_renderer(r);
        }
        renderer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRenderer {
        name: String,
        priority: u32,
        render_called: std::cell::RefCell<bool>,
    }

    impl MockRenderer {
        fn new(name: impl Into<String>, priority: u32) -> Self {
            Self {
                name: name.into(),
                priority,
                render_called: std::cell::RefCell::new(false),
            }
        }
    }

    impl MultiRenderable for MockRenderer {
        fn render(
            &mut self,
            _encoder: &mut CommandEncoder,
            _view: &TextureView,
            _data_store: &DataStore,
            _device: &Device,
            _queue: &Queue,
        ) {
            *self.render_called.borrow_mut() = true;
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> u32 {
            self.priority
        }
    }
}
