//! Overlay renderer implementations

use gpu_charts_shared::RenderLocation;

/// Trait for overlay renderers
pub trait OverlayRenderer: Send {
    fn render(&mut self, pass: &mut wgpu::RenderPass);
    fn render_location(&self) -> RenderLocation;
    fn on_resize(&mut self, width: u32, height: u32);
}

/// Volume overlay renderer
pub struct VolumeOverlay {
    location: RenderLocation,
}

impl VolumeOverlay {
    pub fn new() -> Self {
        Self {
            location: RenderLocation::SubChart,
        }
    }
}

impl OverlayRenderer for VolumeOverlay {
    fn render(&mut self, pass: &mut wgpu::RenderPass) {
        // TODO: Implement volume rendering
    }

    fn render_location(&self) -> RenderLocation {
        self.location
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        // TODO: Handle resize
    }
}

/// Moving average overlay
pub struct MovingAverageOverlay {
    period: u32,
    location: RenderLocation,
}

impl MovingAverageOverlay {
    pub fn new(period: u32) -> Self {
        Self {
            period,
            location: RenderLocation::MainChart,
        }
    }
}

impl OverlayRenderer for MovingAverageOverlay {
    fn render(&mut self, pass: &mut wgpu::RenderPass) {
        // TODO: Implement MA rendering
    }

    fn render_location(&self) -> RenderLocation {
        self.location
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        // TODO: Handle resize
    }
}
