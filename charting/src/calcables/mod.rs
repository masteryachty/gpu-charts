pub mod min_max;

// Simple OHLC data structure used by candlestick renderer
#[derive(Debug, Clone, Copy)]
pub struct OhlcData {
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
}
