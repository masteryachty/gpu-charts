// Compute shader for extracting close prices from OHLC candle data
// This allows us to use candle close prices as input for indicators like EMA

struct OhlcCandle {
    timestamp: u32,    // Candle start time
    open: f32,         // Opening price
    high: f32,         // Highest price
    low: f32,          // Lowest price
    close: f32,        // Closing price
}

struct ExtractParams {
    candle_count: u32,  // Number of candles to process
}

@group(0) @binding(0) var<storage, read> candles: array<OhlcCandle>;
@group(0) @binding(1) var<storage, read_write> close_prices: array<f32>;
@group(0) @binding(2) var<uniform> params: ExtractParams;

// Extract close prices from OHLC candles
@compute @workgroup_size(256, 1, 1)
fn extract_close(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    if (idx >= params.candle_count) {
        return;
    }
    
    // Extract the close price from the candle
    close_prices[idx] = candles[idx].close;
}