// Bar chart vertex and fragment shaders

struct Uniforms {
    transform: mat4x4<f32>,
    bar_color: vec4<f32>,
    bar_width: f32,
    bar_spacing: f32,
    viewport_width: f32,
    viewport_height: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct InstanceInput {
    @location(0) x_pos: f32,
    @location(1) value: f32,
    @location(2) bar_index: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    instance: InstanceInput
) -> VertexOutput {
    var output: VertexOutput;
    
    // Calculate bar position
    let bar_total_width = uniforms.bar_width + uniforms.bar_spacing;
    let x_center = instance.x_pos + instance.bar_index * bar_total_width;
    let half_width = uniforms.bar_width * 0.5 / uniforms.viewport_width;
    
    // Generate vertices for a quad (6 vertices for 2 triangles)
    var positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(x_center - half_width, 0.0),        // 0: bottom-left
        vec2<f32>(x_center + half_width, 0.0),        // 1: bottom-right
        vec2<f32>(x_center - half_width, instance.value), // 2: top-left
        vec2<f32>(x_center + half_width, 0.0),        // 3: bottom-right
        vec2<f32>(x_center + half_width, instance.value), // 4: top-right
        vec2<f32>(x_center - half_width, instance.value)  // 5: top-left
    );
    
    let pos = positions[vertex_idx % 6u];
    output.position = uniforms.transform * vec4<f32>(pos, 0.0, 1.0);
    output.color = uniforms.bar_color;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}