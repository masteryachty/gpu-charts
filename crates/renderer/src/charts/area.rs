//! Area chart renderer implementation

use crate::ChartRenderer;
use wgpu::{Device, Queue, CommandEncoder, TextureView};

pub struct AreaChartRenderer {
    // TODO: Add fields
}

impl AreaChartRenderer {
    pub fn new(_device: &Device) -> Self {
        Self {
            // TODO: Initialize
        }
    }
}

impl ChartRenderer for AreaChartRenderer {
    fn render(&mut self, _encoder: &mut CommandEncoder, _view: &TextureView, _device: &Device, _queue: &Queue) {
        // TODO: Implement area chart rendering
    }
    
    fn resize(&mut self, _width: u32, _height: u32) {
        // TODO: Handle resize
    }
    
    fn name(&self) -> &str {
        "AreaChart"
    }
}