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

@group(0) @binding(0) var<uniform> x_min_max: MinMaxU32;
@group(0) @binding(1) var<uniform> y_min_max: MinMax;

@vertex
fn vs_main(
    @location(0) pos: vec2f
) -> VertexPayload {
    var start_ts = x_min_max.min_val;
    var output: VertexPayload;

    var projection = world_to_screen_conversion_with_margin(0., f32(x_min_max.max_val - start_ts), y_min_max.min_val, y_min_max.max_val, -1., 1.);
    output.position = projection * vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    output.position.y = pos.y;
    output.position.z = 0.01;
    output.position.w = 1.0;

    output.color = vec3<f32>(1.0, 1.0, 1.0);

    return output;
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