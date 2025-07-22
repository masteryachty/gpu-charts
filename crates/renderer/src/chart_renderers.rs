//! Chart renderer implementations

use crate::{GpuBufferSet, RenderContext, Viewport};
use gpu_charts_shared::VisualConfig;
use std::sync::Arc;

/// Trait for all chart renderers
pub trait ChartRenderer {
    /// Render the chart
    fn render<'a>(
        &'a mut self,
        pass: &mut wgpu::RenderPass<'a>,
        buffer_sets: &[Arc<GpuBufferSet>],
        context: &RenderContext,
    );

    /// Update visual configuration
    fn update_visual_config(&mut self, config: &VisualConfig);

    /// Handle resize events
    fn on_resize(&mut self, width: u32, height: u32);

    /// Handle viewport changes (pan/zoom)
    fn on_viewport_change(&mut self, viewport: &Viewport);

    /// Get the number of draw calls this renderer will make
    fn get_draw_call_count(&self) -> u32;
}

mod area_chart;
mod bar_chart;
mod candlestick_chart;
mod line_chart;

pub use area_chart::AreaChartRenderer;
pub use bar_chart::BarChartRenderer;
pub use candlestick_chart::CandlestickRenderer;
pub use line_chart::LineChartRenderer;
