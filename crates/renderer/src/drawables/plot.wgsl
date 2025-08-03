struct Vertex {
    @location(0) x: u32,
    @location(1) y: f32,
};

struct VertexPayload {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct MinMaxU32 {
    min_val: u32,
    max_val: u32,
};

@group(0) @binding(0) var<uniform> x_min_max: MinMaxU32;
@group(0) @binding(1) var<storage, read> y_min_max: vec2<f32>;  // GPU-computed min/max
@group(0) @binding(2) var<uniform> line_color: vec3<f32>;

@vertex
fn vs_main(vertex: Vertex) -> VertexPayload {

    var start_ts = x_min_max.min_val;
    // Handle the case where vertex.x < start_ts to avoid unsigned underflow
    var x_f32: f32;
    if (vertex.x >= start_ts) {
        x_f32 = f32(vertex.x - start_ts);
    } else {
        // If x is before start, calculate negative offset
        x_f32 = -f32(start_ts - vertex.x);
    }
    var out: VertexPayload;
    var projection = world_to_screen_conversion_with_margin(0., f32(x_min_max.max_val - start_ts), y_min_max.x, y_min_max.y, -1., 1.);
    out.position = projection * vec4f(x_f32, vertex.y, 0., 1.);
    out.position.z = 0.0;  // Not the issue, but keeping at 0.0 for correctness
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