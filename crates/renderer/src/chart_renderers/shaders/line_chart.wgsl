// Line chart vertex and fragment shaders

struct Uniforms {
    transform: mat4x4<f32>,
    color: vec4<f32>,
    line_width: f32,
    viewport_width: f32,
    viewport_height: f32,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Transform position to clip space
    output.position = uniforms.transform * vec4<f32>(input.position, 0.0, 1.0);
    output.color = uniforms.color;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}