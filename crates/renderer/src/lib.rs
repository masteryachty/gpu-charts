pub mod buffer_pool;
pub mod calcables;
pub mod charts;
pub mod compute;
pub mod compute_engine;
pub mod drawables;
pub mod multi_renderer;
pub mod pipeline_builder;
pub mod render_context;
pub mod shaders;

use shared_types::{GpuChartsError, GpuChartsResult};

pub use calcables::{candle_aggregator::CandleAggregator, min_max::calculate_min_max_y};
pub use charts::TriangleRenderer;
pub use drawables::{
    candlestick::CandlestickRenderer, plot::PlotRenderer, tooltip::TooltipRenderer,
    x_axis::XAxisRenderer, y_axis::YAxisRenderer,
};
pub use multi_renderer::{
    ConfigurablePlotRenderer, MultiRenderable, MultiRenderer, MultiRendererBuilder, RenderOrder,
    RendererAdapter,
};
pub use render_context::RenderContext;

/// Re-export error types
pub type RenderError = GpuChartsError;
pub type RenderResult<T> = GpuChartsResult<T>;
