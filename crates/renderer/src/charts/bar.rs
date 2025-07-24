//! Bar chart renderer implementation

use crate::ChartRenderer;
use wgpu::{CommandEncoder, Device, Queue, TextureView};

pub struct BarChartRenderer {
    // TODO: Add fields
}

impl BarChartRenderer {
    pub fn new(_device: &Device) -> Self {
        Self {
            // TODO: Initialize
        }
    }
}

impl ChartRenderer for BarChartRenderer {
    fn render(
        &mut self,
        _encoder: &mut CommandEncoder,
        _view: &TextureView,
        _device: &Device,
        _queue: &Queue,
    ) {
        // TODO: Implement bar chart rendering
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        // TODO: Handle resize
    }

    fn name(&self) -> &str {
        "BarChart"
    }
}
