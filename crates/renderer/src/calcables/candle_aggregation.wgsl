// candle_aggregation.wgsl
// GPU compute shader for parallel OHLC candle aggregation from tick data

struct CandleParams {
    start_timestamp: u32,      // First candle's start time
    candle_timeframe: u32,     // Duration of each candle in seconds
    num_candles: u32,          // Total number of candles to generate
    tick_count: u32,           // Total number of input ticks
}

struct OhlcCandle {
    timestamp: u32,    // Candle start time
    open: f32,         // Opening price
    high: f32,         // Highest price
    low: f32,          // Lowest price
    close: f32,        // Closing price
}

@group(0) @binding(0)
var<storage, read> timestamps: array<u32>;

@group(0) @binding(1)
var<storage, read> prices: array<f32>;

@group(0) @binding(2)
var<storage, read_write> candles: array<OhlcCandle>;

@group(0) @binding(3)
var<uniform> params: CandleParams;

// Shared memory for workgroup reduction
var<workgroup> local_open: array<f32, 64>;
var<workgroup> local_high: array<f32, 64>;
var<workgroup> local_low: array<f32, 64>;
var<workgroup> local_close: array<f32, 64>;
var<workgroup> local_first_idx: array<u32, 64>;
var<workgroup> local_last_idx: array<u32, 64>;
var<workgroup> local_has_data: array<u32, 64>;

// Workgroup size of 64 - each workgroup processes one candle
@compute @workgroup_size(64)
fn main(
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let candle_idx = workgroup_id.x;
    let thread_id = local_id.x;
    
    // Bounds check
    if (candle_idx >= params.num_candles) {
        return;
    }
    
    // Calculate this candle's time range
    let candle_start = params.start_timestamp + candle_idx * params.candle_timeframe;
    let candle_end = candle_start + params.candle_timeframe;
    
    // Initialize thread-local values
    var thread_open = 0.0;
    var thread_high = -3.402823466e+38;  // -Infinity
    var thread_low = 3.402823466e+38;     // +Infinity
    var thread_close = 0.0;
    var thread_first_idx = 0xFFFFFFFFu;   // Max u32 as sentinel
    var thread_last_idx = 0u;
    var thread_has_data = 0u;
    
    // Each thread searches a portion of the tick data
    let ticks_per_thread = (params.tick_count + 63u) / 64u;
    let thread_start = min(thread_id * ticks_per_thread, params.tick_count);
    let thread_end = min((thread_id + 1u) * ticks_per_thread, params.tick_count);
    
    // Binary search optimization for sorted timestamps
    // Find approximate range using thread boundaries
    var search_start = thread_start;
    var search_end = thread_end;
    
    // Quick bounds check to skip this thread if all timestamps are outside range
    if (thread_start < params.tick_count) {
        let first_time = timestamps[thread_start];
        let last_time = timestamps[min(thread_end - 1u, params.tick_count - 1u)];
        
        // Skip if entire thread range is before or after candle
        if (last_time < candle_start || first_time >= candle_end) {
            search_start = thread_end; // Skip processing
        }
    }
    
    // Process assigned tick range
    for (var i = search_start; i < search_end; i++) {
        let timestamp = timestamps[i];
        
        // Early exit if we've passed the candle end (sorted data)
        if (timestamp >= candle_end) {
            break;
        }
        
        // Check if tick is within this candle's time range
        if (timestamp >= candle_start && timestamp < candle_end) {
            let price = prices[i];
            
            // Track first tick (open)
            if (i < thread_first_idx || thread_first_idx == 0xFFFFFFFFu) {
                thread_first_idx = i;
                thread_open = price;
            }
            
            // Track last tick (close)
            if (i >= thread_last_idx) {
                thread_last_idx = i;
                thread_close = price;
            }
            
            // Update high/low
            thread_high = max(thread_high, price);
            thread_low = min(thread_low, price);
            thread_has_data = 1u;
        }
    }
    
    // Store thread results to shared memory
    local_open[thread_id] = thread_open;
    local_high[thread_id] = thread_high;
    local_low[thread_id] = thread_low;
    local_close[thread_id] = thread_close;
    local_first_idx[thread_id] = thread_first_idx;
    local_last_idx[thread_id] = thread_last_idx;
    local_has_data[thread_id] = thread_has_data;
    
    workgroupBarrier();
    
    // Parallel reduction - only thread 0 computes final result
    if (thread_id == 0u) {
        var group_first_idx = 0xFFFFFFFFu;
        var group_last_idx = 0u;
        var group_open = 0.0;
        var group_close = 0.0;
        var group_high = -3.402823466e+38;
        var group_low = 3.402823466e+38;
        var group_has_data = 0u;
        
        // Find the actual first and last ticks across all threads
        for (var i = 0u; i < 64u; i++) {
            if (local_has_data[i] == 1u) {
                group_has_data = 1u;
                
                // Check for earliest tick (open)
                if (local_first_idx[i] < group_first_idx) {
                    group_first_idx = local_first_idx[i];
                    group_open = local_open[i];
                }
                
                // Check for latest tick (close)
                if (local_last_idx[i] > group_last_idx) {
                    group_last_idx = local_last_idx[i];
                    group_close = local_close[i];
                }
                
                // Update high/low
                group_high = max(group_high, local_high[i]);
                group_low = min(group_low, local_low[i]);
            }
        }
        
        // Write final candle data
        if (group_has_data == 1u) {
            candles[candle_idx] = OhlcCandle(
                candle_start,
                group_open,
                group_high,
                group_low,
                group_close
            );
        } else {
            // Empty candle - use previous close if available or zero
            var prev_close = 0.0;
            if (candle_idx > 0u) {
                // Note: This creates a data dependency between candles
                // In practice, we might handle this differently
                prev_close = candles[candle_idx - 1u].close;
            }
            
            candles[candle_idx] = OhlcCandle(
                candle_start,
                prev_close,
                prev_close,
                prev_close,
                prev_close
            );
        }
    }
}