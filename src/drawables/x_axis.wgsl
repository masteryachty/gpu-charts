struct VertexPayload {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) pos: vec2f
) -> VertexPayload {
    var output: VertexPayload;
    // Transform x from [0,1] to [-1,1] range for proper viewport mapping
    output.position = vec4f(
        pos.x * 2.0 - 1.0,  // Normalize x from [0,1] to [-1,1]
        pos.y,              // y is already in [-1,1]
        0.01,               // Small z value to avoid z-fighting
        1.0                 // Required homogeneous coordinate
    );
    output.color = vec3f(1.0, 1.0, 1.0);
    return output;
}

@fragment
fn fs_main(in: VertexPayload) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}