use std::rc::Rc;

/// Holds core WebGPU resources needed for rendering
/// This provides proper separation of concerns between the renderer and wasm-bridge crates
///
/// Note: The 'static lifetime on surface is appropriate in the WebAssembly context
/// where the canvas element lives for the entire application lifetime.
pub struct RenderContext {
    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}

impl RenderContext {
    pub fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        Self {
            device,
            queue,
            surface,
            config,
        }
    }

    /// Resize the surface configuration
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Get the current surface texture
    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    /// Configure the surface
    pub fn configure_surface(&self) {
        self.surface.configure(&self.device, &self.config);
    }
}
