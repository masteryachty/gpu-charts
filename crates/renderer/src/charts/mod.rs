//! Chart-specific rendering implementations

// Legacy chart renderers - commented out as they depend on removed ChartRenderer trait
// pub mod area;
// pub mod bar;
// pub mod candlestick;
// pub mod line;
pub mod triangle_renderer;

// pub use area::AreaChartRenderer;
// pub use bar::BarChartRenderer;
// pub use candlestick::CandlestickChartRenderer;
// pub use line::LineChartRenderer;
pub use triangle_renderer::TriangleRenderer;
