// Compressed vertex shader for plot rendering

struct CompressedVertex {
    @location(0) time_value: u32,  // time (16 bits) | value (16 bits)
    @location(1) metadata: u32,     // series_index (8) | flags (8) | reserved (16)
};

struct VertexPayload {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct MinMax {
    min_val: f32,
    max_val: f32,
};

struct MinMaxU32 {
    min_val: u32,
    max_val: u32,
};

struct CompressionParams {
    time_min: u32,
    time_max: u32,
    value_min: f32,
    value_max: f32,
};

@group(0) @binding(0) var<uniform> x_min_max: MinMaxU32;
@group(0) @binding(1) var<uniform> y_min_max: MinMax;
@group(0) @binding(2) var<uniform> line_color: vec3<f32>;
@group(0) @binding(3) var<uniform> compression_params: CompressionParams;

// Decompress time from packed format
fn decompress_time(packed: u32) -> f32 {
    let time_u16 = (packed >> 16u) & 0xFFFFu;
    let normalized = f32(time_u16) / 65535.0;
    let time_range = f32(compression_params.time_max - compression_params.time_min);
    return f32(compression_params.time_min) + normalized * time_range;
}

// Decompress value from packed format
fn decompress_value(packed: u32) -> f32 {
    let value_u16 = packed & 0xFFFFu;
    let normalized = f32(value_u16) / 65535.0;
    let value_range = compression_params.value_max - compression_params.value_min;
    return compression_params.value_min + normalized * value_range;
}

@vertex
fn vs_main(vertex: CompressedVertex) -> VertexPayload {
    // Decompress vertex data
    let time = decompress_time(vertex.time_value);
    let value = decompress_value(vertex.time_value);
    
    var start_ts = x_min_max.min_val;
    var x_f32 = time - f32(start_ts);
    var out: VertexPayload;
    var projection = world_to_screen_conversion_with_margin(0., f32(x_min_max.max_val - start_ts), y_min_max.min_val, y_min_max.max_val, -1., 1.);
    out.position = projection * vec4f(x_f32, value, 0., 1.);
    out.position.z = 1.;
    out.position.w = 1.;
    out.color = line_color;
    return out;
}

@fragment
fn fs_main(in: VertexPayload) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

fn world_to_screen_conversion_with_margin(
    left: f32, right: f32,
    bottom: f32, top: f32,
    near: f32, far: f32
) -> mat4x4<f32> {
    // Apply 10% margin to Y
    let y_range = top - bottom;
    let y_margin = y_range * 0.1;

    let top_m = top + y_margin;
    let bottom_m = bottom - y_margin;

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