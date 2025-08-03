struct VertexPayload {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct MinMax {
    min_val: f32,
    max_val: f32,
};

@group(0) @binding(0) var<uniform> x_min_max: MinMax;
@group(0) @binding(1) var<uniform> y_min_max: MinMax;

@vertex
fn vs_main(
    @location(0) pos: vec2f
) -> VertexPayload {
    var output: VertexPayload;
    
    // Only transform X coordinate, Y is already in clip space (-1 to 1)
    let x_normalized = (pos.x - x_min_max.min_val) / (x_min_max.max_val - x_min_max.min_val);
    output.position.x = x_normalized * 2.0 - 1.0;
    output.position.y = pos.y;  // Already in clip space
    output.position.z = 0.5;
    output.position.w = 1.0;
    output.color = vec3f(0.6, 0.6, 0.6); // Light gray color for grid lines
    return output;
}

@fragment
fn fs_main(in: VertexPayload) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 0.2); // Semi-transparent grid lines
}


fn world_to_screen_conversion_with_margin(
    left: f32, right: f32,
    bottom: f32, top: f32,
    near: f32, far: f32
) -> mat4x4<f32> {
    // For X-axis vertical lines, we don't want Y margin
    // Apply 10% margin to Y only for horizontal elements
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

fn world_to_screen_conversion_no_margin(
    left: f32, right: f32,
    bottom: f32, top: f32,
    near: f32, far: f32
) -> mat4x4<f32> {
    // No margin version for vertical lines
    let rl = right - left;
    let tb = top - bottom;
    let ds = far - near;

    let tx = -(right + left) / rl;
    let ty = -(top + bottom) / tb;
    let tz = -near / ds;

    return mat4x4<f32>(
        vec4<f32>(2.0 / rl, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 2.0 / tb, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0 / ds, 0.0),
        vec4<f32>(tx, ty, tz, 1.0)
    );
}