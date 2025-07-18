pub mod min_max;
pub mod candle_aggregator;

pub use candle_aggregator::{CandleAggregator, GpuOhlcCandle};

// Simple OHLC data structure used by candlestick renderer
#[derive(Debug, Clone, Copy)]
pub struct OhlcData {
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ohlc_data_creation() {
        let ohlc = OhlcData {
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
        };

        assert_eq!(ohlc.open, 100.0);
        assert_eq!(ohlc.high, 110.0);
        assert_eq!(ohlc.low, 95.0);
        assert_eq!(ohlc.close, 105.0);
    }

    #[test]
    fn test_candle_type_detection() {
        // Bullish candle
        let bullish = OhlcData {
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
        };
        assert!(bullish.close > bullish.open, "Should be bullish");

        // Bearish candle
        let bearish = OhlcData {
            open: 105.0,
            high: 110.0,
            low: 95.0,
            close: 100.0,
        };
        assert!(bearish.close < bearish.open, "Should be bearish");

        // Doji candle
        let doji = OhlcData {
            open: 100.0,
            high: 105.0,
            low: 95.0,
            close: 100.0,
        };
        assert_eq!(doji.close, doji.open, "Should be doji");
    }

    #[test]
    fn test_candle_boundary_calculations() {
        let view_start = 1030u32;
        let view_end = 1970u32;
        let timeframe = 60u32;

        // First candle that includes or precedes view start
        let first_candle_start = (view_start / timeframe) * timeframe;
        assert_eq!(first_candle_start, 1020);

        // Last candle that includes or extends past view end
        let last_candle_end = ((view_end + timeframe - 1) / timeframe) * timeframe;
        assert_eq!(last_candle_end, 1980);

        // Number of candles
        let num_candles = (last_candle_end - first_candle_start) / timeframe;
        assert_eq!(num_candles, 16);
    }

    #[test]
    fn test_binary_search_for_candles() {
        let timestamps = vec![100u32, 200, 300, 400, 500, 600, 700, 800];
        let candle_start = 350u32;

        let mut start_idx = 0;
        let mut end_idx = timestamps.len();

        // Binary search to find first tick >= candle_start
        while start_idx < end_idx {
            let mid = start_idx + (end_idx - start_idx) / 2;
            if timestamps[mid] < candle_start {
                start_idx = mid + 1;
            } else {
                end_idx = mid;
            }
        }

        assert_eq!(start_idx, 3);
        assert_eq!(timestamps[start_idx], 400);
    }

    #[test]
    fn test_vertex_size_calculations() {
        // Each vertex: u32 (4 bytes) + 4 * f32 (16 bytes) = 20 bytes
        let vertex_size = std::mem::size_of::<u32>() + 4 * std::mem::size_of::<f32>();
        assert_eq!(vertex_size, 20);

        // 6 vertices per candle body (2 triangles)
        let body_size_per_candle = 6 * vertex_size;
        assert_eq!(body_size_per_candle, 120);

        // 4 vertices per candle wick (2 lines)
        let wick_size_per_candle = 4 * vertex_size;
        assert_eq!(wick_size_per_candle, 80);
    }
}
