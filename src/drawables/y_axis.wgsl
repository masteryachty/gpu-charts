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
    // Transform x from [0,1] to [-1,1] range for proper viewport mapping
    var projection = ortho_with_margin(x_min_max.min_val, x_min_max.max_val, y_min_max.min_val, y_min_max.max_val, -1., 1.);
    output.position = projection * vec4f(pos.x, pos.y, 0, 1);
    output.position.x = pos.x;
    output.position.z = 0.01;
    output.position.w = 1.;
    output.color = vec3f(1.0, 1.0, 1.0);
    return output;
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
    let ds = far - near;

    let tx = -(right + left) / rl;
    let ty = -(top + bottom) / tb;
    let tz = -near / ds;

    return mat4x4<f32>(
        vec4<f32>(2.0 / rl, 0.0,       0.0,    0.0),
        vec4<f32>(0.0,      2.0 / tb,  0.0,    0.0),
        vec4<f32>(0.0,      0.0,       1.0 / ds, 0.0),
        vec4<f32>(tx,       ty,        tz,     1.0),
    );
}
