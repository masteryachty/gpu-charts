// GPU-accelerated candlestick rendering shader
// Reads candle data directly from GPU compute output buffer

struct OhlcCandle {
    timestamp: u32,    // Candle start time
    open: f32,         // Opening price
    high: f32,         // Highest price
    low: f32,          // Lowest price
    close: f32,        // Closing price
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct MinMaxU32 {
    min_val: u32,
    max_val: u32,
};

struct MinMax {
    min_val: f32,
    max_val: f32,
};

@group(0) @binding(0) var<uniform> x_range: MinMaxU32;
@group(0) @binding(1) var<uniform> y_range: MinMax;
@group(0) @binding(2) var<uniform> candle_timeframe: f32;
@group(0) @binding(3) var<storage, read> candles: array<OhlcCandle>;

// Vertex shader for candle bodies (rectangles)
@vertex
fn vs_body(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Calculate which candle and which corner of the rectangle
    let candle_idx = vertex_idx / 6u;  // 6 vertices per candle (2 triangles)
    let corner_idx = vertex_idx % 6u;
    
    // Get the candle data
    let candle = candles[candle_idx];
    
    // Calculate candle width - use 80% of timeframe to leave gaps
    let candle_width = candle_timeframe * 0.8;
    let actual_half_width = candle_width * 0.5;
    
    // Get the top and bottom of the candle body
    let body_top = max(candle.open, candle.close);
    let body_bottom = min(candle.open, candle.close);
    
    // Ensure minimum body height for visibility (at least 0.5% of the y range)
    let y_range_size = y_range.max_val - y_range.min_val;
    let min_body_height = y_range_size * 0.005;
    let body_height = max(body_top - body_bottom, min_body_height);
    
    // Adjust body positions if needed to ensure minimum height
    let adjusted_body_top = body_bottom + body_height;
    
    // Determine vertex position based on corner index
    var x_offset: f32;
    var y_value: f32;
    
    // Create rectangle from 6 vertices (2 triangles)
    // Triangle 1: 0=top-left, 1=bottom-left, 2=bottom-right
    // Triangle 2: 3=top-left, 4=bottom-right, 5=top-right
    if (corner_idx == 0u || corner_idx == 3u) {
        // Top-left
        x_offset = -actual_half_width;
        y_value = adjusted_body_top;
    } else if (corner_idx == 1u) {
        // Bottom-left
        x_offset = -actual_half_width;
        y_value = body_bottom;
    } else if (corner_idx == 2u || corner_idx == 4u) {
        // Bottom-right
        x_offset = actual_half_width;
        y_value = body_bottom;
    } else {
        // Top-right
        x_offset = actual_half_width;
        y_value = adjusted_body_top;
    }
    
    // Calculate the x position for this vertex
    // Center the candle body on the candle's time period
    var start_ts = x_range.min_val;
    var candle_center_x = f32(candle.timestamp - start_ts) + (candle_timeframe / 2.0);
    var x_f32 = candle_center_x + x_offset;
    
    // Apply projection
    var projection = world_to_screen_conversion_with_margin(
        0., f32(x_range.max_val - start_ts), 
        y_range.min_val, y_range.max_val, 
        -1., 1.
    );
    
    out.position = projection * vec4f(x_f32, y_value, 0., 1.);
    out.position.z = 0.0;
    out.position.w = 1.0;
    
    // Color based on bullish/bearish
    if (candle.close > candle.open) {
        out.color = vec3<f32>(0.0, 1.0, 0.0); // Green for bullish
    } else if (candle.close < candle.open) {
        out.color = vec3<f32>(1.0, 0.0, 0.0); // Red for bearish
    } else {
        out.color = vec3<f32>(1.0, 1.0, 0.0); // Yellow for doji
    }
    
    return out;
}

// Vertex shader for wicks (lines)
@vertex
fn vs_wick(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Calculate which candle and which wick vertex
    let candle_idx = vertex_idx / 4u;  // 4 vertices per candle (2 lines)
    let wick_idx = vertex_idx % 4u;
    
    // Get the candle data
    let candle = candles[candle_idx];
    
    // Determine y position based on wick vertex
    var y_value: f32;
    
    if (wick_idx == 0u) {
        // Upper wick start (high)
        y_value = candle.high;
    } else if (wick_idx == 1u) {
        // Upper wick end (top of body)
        y_value = max(candle.open, candle.close);
    } else if (wick_idx == 2u) {
        // Lower wick start (bottom of body)
        y_value = min(candle.open, candle.close);
    } else {
        // Lower wick end (low)
        y_value = candle.low;
    }
    
    // Calculate center position of candle
    // The wick is centered on the candle
    var start_ts = x_range.min_val;
    var x_f32 = f32(candle.timestamp - start_ts) + (candle_timeframe / 2.0);
    
    // Apply projection
    var projection = world_to_screen_conversion_with_margin(
        0., f32(x_range.max_val - start_ts), 
        y_range.min_val, y_range.max_val, 
        -1., 1.
    );
    
    out.position = projection * vec4f(x_f32, y_value, 0., 1.);
    out.position.z = 0.0;
    out.position.w = 1.0;
    
    // Wicks are always gray
    out.color = vec3<f32>(0.6, 0.6, 0.6);
    
    return out;
}

// Fragment shader for candle bodies
@fragment
fn fs_candle(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

// Fragment shader for wicks
@fragment
fn fs_wick(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

// Utility function for coordinate transformation with margin
fn world_to_screen_conversion_with_margin(
    left: f32, right: f32,
    bottom: f32, top: f32,
    near: f32, far: f32
) -> mat4x4<f32> {
    // Apply 10% margin to Y axis (same as plot.wgsl)
    let y_range = top - bottom;
    let y_margin = y_range * 0.1;
    
    let top_m = top + y_margin;
    let bottom_m = bottom - y_margin;
    
    // No X margin to match plot.wgsl
    let rl = right - left;
    let tb = top_m - bottom_m;
    let ds = far - near;
    
    let tx = -(right + left) / rl;
    let ty = -(top_m + bottom_m) / tb;
    let tz = -near / ds;
    
    return mat4x4<f32>(
        vec4<f32>(2.0 / rl, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 2.0 / tb, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0 / ds, 0.0),
        vec4<f32>(tx, ty, tz, 1.0)
    );
}