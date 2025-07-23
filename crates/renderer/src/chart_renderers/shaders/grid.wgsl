// Grid and axes shader

struct Uniforms {
    transform: mat4x4<f32>,
    grid_color: vec4<f32>,
    axes_color: vec4<f32>,
    viewport_size: vec2<f32>,
    grid_spacing: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // Create a full-screen quad
    let x = f32(vertex_index & 1u) * 2.0 - 1.0;
    let y = f32((vertex_index >> 1u) & 1u) * 2.0 - 1.0;
    
    output.position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    output.color = uniforms.grid_color;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let grid_x = fract(input.uv.x * uniforms.grid_spacing.x);
    let grid_y = fract(input.uv.y * uniforms.grid_spacing.y);
    
    let line_width = 2.0 / uniforms.viewport_size.x;
    
    // Draw vertical grid lines
    if (grid_x < line_width || grid_x > 1.0 - line_width) {
        return uniforms.grid_color;
    }
    
    // Draw horizontal grid lines
    if (grid_y < line_width || grid_y > 1.0 - line_width) {
        return uniforms.grid_color;
    }
    
    // Draw axes (at center)
    let center_threshold = 0.01;
    if (abs(input.uv.x - 0.5) < center_threshold) {
        return uniforms.axes_color;
    }
    if (abs(input.uv.y - 0.5) < center_threshold) {
        return uniforms.axes_color;
    }
    
    // Transparent background
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}