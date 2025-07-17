// Vertex data structure for candlestick rendering
// Each vertex contains the full OHLC data to allow flexible rendering
struct CandleVertex {
    @location(0) timestamp: u32,  // Unix timestamp in seconds
    @location(1) open: f32,       // Opening price
    @location(2) high: f32,       // Highest price in period
    @location(3) low: f32,        // Lowest price in period
    @location(4) close: f32,      // Closing price
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

// Vertex shader for candle bodies (rectangles)
@vertex
fn vs_body(
    vertex: CandleVertex,
    @builtin(vertex_index) vertex_idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Calculate candle width - use 80% of timeframe to leave gaps
    let candle_width = candle_timeframe * 0.8;
    let actual_half_width = candle_width * 0.5;
    
    // Determine which corner of the rectangle this vertex represents
    let corner_idx = vertex_idx % 6u;
    var x_offset: f32;
    var y_value: f32;
    
    // Get the top and bottom of the candle body
    let body_top = max(vertex.open, vertex.close);
    let body_bottom = min(vertex.open, vertex.close);
    
    // Ensure minimum body height for visibility (at least 0.5% of the y range)
    let y_range_size = y_range.max_val - y_range.min_val;
    let min_body_height = y_range_size * 0.005;
    let body_height = max(body_top - body_bottom, min_body_height);
    
    // Adjust body positions if needed to ensure minimum height
    let adjusted_body_top = body_bottom + body_height;
    
    // Create rectangle from 6 vertices (2 triangles)
    // Triangle 1: top-left, bottom-left, bottom-right
    // Triangle 2: top-left, bottom-right, top-right
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
    
    // Convert to relative coordinates (same as plot.wgsl)
    var start_ts = x_range.min_val;
    var x_f32 = f32(vertex.timestamp - start_ts) + x_offset;
    
    // Apply the same projection as plot.wgsl
    var projection = world_to_screen_conversion_with_margin(
        0., f32(x_range.max_val - start_ts), 
        y_range.min_val, y_range.max_val, 
        -1., 1.
    );
    
    out.position = projection * vec4f(x_f32, y_value, 0., 1.);
    out.position.z = 0.0;
    out.position.w = 1.0;
    
    // Color based on bullish/bearish
    if (vertex.close > vertex.open) {
        out.color = vec3<f32>(0.0, 1.0, 0.0); // Green for bullish
    } else if (vertex.close < vertex.open) {
        out.color = vec3<f32>(1.0, 0.0, 0.0); // Red for bearish
    } else {
        out.color = vec3<f32>(1.0, 1.0, 0.0); // Yellow for doji
    }
    
    return out;
}

// Vertex shader for wicks (lines)
@vertex
fn vs_wick(
    vertex: CandleVertex,
    @builtin(vertex_index) vertex_idx: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Determine which part of the wick this vertex represents
    let wick_idx = vertex_idx % 4u;
    var y_value: f32;
    
    if (wick_idx == 0u) {
        // Upper wick start (high)
        y_value = vertex.high;
    } else if (wick_idx == 1u) {
        // Upper wick end (top of body)
        y_value = max(vertex.open, vertex.close);
    } else if (wick_idx == 2u) {
        // Lower wick start (bottom of body)
        y_value = min(vertex.open, vertex.close);
    } else {
        // Lower wick end (low)
        y_value = vertex.low;
    }
    
    // All wick vertices are at the center of the candle (no x offset)
    var start_ts = x_range.min_val;
    var x_f32 = f32(vertex.timestamp - start_ts);
    
    // Apply the same projection as plot.wgsl
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