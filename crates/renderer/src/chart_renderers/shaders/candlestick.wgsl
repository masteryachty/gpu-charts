// Candlestick chart shaders

struct Uniforms {
    transform: mat4x4<f32>,
    bullish_color: vec4<f32>,
    bearish_color: vec4<f32>,
    wick_width: f32,
    viewport_width: f32,
    viewport_height: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct CandleInput {
    @location(0) time: f32,
    @location(1) ohlc: vec4<f32>, // open, high, low, close
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

// Vertex shader for candle bodies
@vertex
fn vs_body(@builtin(vertex_index) vertex_idx: u32, input: CandleInput) -> VertexOutput {
    var output: VertexOutput;
    
    let open = input.ohlc.x;
    let close = input.ohlc.w;
    let is_bullish = close > open;
    
    // Generate box vertices (6 vertices for 2 triangles)
    let candle_width = 10.0; // Width in pixels
    let half_width = candle_width * 0.5 / uniforms.viewport_width;
    
    let x_left = input.time - half_width;
    let x_right = input.time + half_width;
    let y_bottom = min(open, close);
    let y_top = max(open, close);
    
    // Generate vertices for a quad
    var positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(x_left, y_bottom),   // 0: bottom-left
        vec2<f32>(x_right, y_bottom),  // 1: bottom-right
        vec2<f32>(x_left, y_top),      // 2: top-left
        vec2<f32>(x_right, y_bottom),  // 3: bottom-right
        vec2<f32>(x_right, y_top),     // 4: top-right
        vec2<f32>(x_left, y_top)       // 5: top-left
    );
    
    let pos = positions[vertex_idx % 6u];
    output.position = uniforms.transform * vec4<f32>(pos, 0.0, 1.0);
    output.color = select(uniforms.bearish_color, uniforms.bullish_color, is_bullish);
    
    return output;
}

// Vertex shader for candle wicks
@vertex
fn vs_wick(@builtin(vertex_index) vertex_idx: u32, input: CandleInput) -> VertexOutput {
    var output: VertexOutput;
    
    let high = input.ohlc.y;
    let low = input.ohlc.z;
    let open = input.ohlc.x;
    let close = input.ohlc.w;
    let is_bullish = close > open;
    
    // Generate wick lines (2 vertices per wick, 4 total)
    var positions: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
        vec2<f32>(input.time, low),          // 0: bottom of lower wick
        vec2<f32>(input.time, min(open, close)), // 1: top of lower wick
        vec2<f32>(input.time, max(open, close)), // 2: bottom of upper wick
        vec2<f32>(input.time, high)          // 3: top of upper wick
    );
    
    let pos = positions[vertex_idx % 4u];
    output.position = uniforms.transform * vec4<f32>(pos, 0.0, 1.0);
    output.color = select(uniforms.bearish_color, uniforms.bullish_color, is_bullish);
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}