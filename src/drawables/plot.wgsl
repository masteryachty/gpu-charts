struct Vertex {
    @location(0) x: f32,
    @location(1) y: f32,
};

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
fn vs_main(vertex: Vertex) -> VertexPayload {
    var out: VertexPayload;
    var projection = ortho_with_margin(x_min_max.min_val, x_min_max.max_val, y_min_max.min_val, y_min_max.max_val, -1., 1.);
    // out.position =  projection * view  * vec4f(vertex.position.x, vertex.position.y, 0, 1);

    out.position = projection * vec4f(vertex.x, vertex.y, 0, 1);
    // out.position.x = vertex.position.x;
    // out.position.y = vertex.position.y;

    out.position.z = 1.;
    out.position.w = 1.;
    out.color = vec3f(1., 1., 1.);
    return out;
}

@fragment
fn fs_main(in: VertexPayload) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}


fn ortho_with_margin(
    left: f32, right: f32,
    bottom: f32, top: f32,
    near: f32, far: f32
) -> mat4x4<f32> {
    let rl = right - left;
    let tb = top - bottom;

    let x_margin = rl * 0.10;
    let y_margin = tb * 0.10;

    let left_m = left - x_margin;
    let right_m = right + x_margin;
    let bottom_m = bottom - y_margin;
    let top_m = top + y_margin;

    let rl_m = right_m - left_m;
    let tb_m = top_m - bottom_m;
    let ds = far - near;

    let tx = -(right_m + left_m) / rl_m;
    let ty = -(top_m + bottom_m) / tb_m;
    let tz = -near / ds;

    return mat4x4<f32>(
        vec4<f32>(2.0 / rl_m, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 2.0 / tb_m, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0 / ds, 0.0),
        vec4<f32>(tx, ty, tz, 1.0)
    );
}