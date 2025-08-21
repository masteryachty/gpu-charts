// Tooltip rendering shader for vertical line and labels

struct Uniforms {
    view_matrix: mat4x4<f32>,
    screen_size: vec2<f32>,
    line_x: f32,
    is_active: f32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

struct LabelData {
    position: vec2<f32>,  // Screen position in pixels
    value: f32,
    color: vec4<f32>,
    _padding: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> labels: array<LabelData>;

// Vertex shader for vertical line
@vertex
fn vs_line(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // Only render if tooltip is active
    if uniforms.is_active < 0.5 {
        output.position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return output;
    }
    
    // Create a vertical line from top to bottom at the specified X position
    let x_ndc = (uniforms.line_x / uniforms.screen_size.x) * 2.0 - 1.0;
    
    // Two vertices for the line
    if vertex_index == 0u {
        output.position = vec4<f32>(x_ndc, 1.0, 0.0, 1.0);  // Top
    } else {
        output.position = vec4<f32>(x_ndc, -1.0, 0.0, 1.0); // Bottom
    }
    
    // Semi-transparent white line
    output.color = vec4<f32>(1.0, 1.0, 1.0, 0.7);
    output.uv = vec2<f32>(0.0, 0.0);
    
    return output;
}

// Vertex shader for label backgrounds and text
@vertex
fn vs_label(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var output: VertexOutput;
    
    // Only render if tooltip is active
    if uniforms.is_active < 0.5 {
        output.position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        return output;
    }
    
    let label = labels[instance_index];
    
    // Create a quad for each label background
    // Each label is rendered as a rectangle with the line's color
    let box_width = 80.0;  // Fixed width for label boxes
    let box_height = 20.0; // Fixed height for label boxes
    
    // Position the box just to the right of the line
    let box_x = uniforms.line_x + 5.0;
    let box_y = label.position.y;
    
    // Convert pixel coordinates to NDC
    var x: f32;
    var y: f32;
    
    // Create quad vertices (triangle strip)
    if (vertex_index == 0u) { // Top-left
        x = box_x;
        y = box_y;
    } else if (vertex_index == 1u) { // Top-right
        x = box_x + box_width;
        y = box_y;
    } else if (vertex_index == 2u) { // Bottom-left
        x = box_x;
        y = box_y + box_height;
    } else if (vertex_index == 3u) { // Bottom-right
        x = box_x + box_width;
        y = box_y + box_height;
    } else {
        x = 0.0;
        y = 0.0;
    }
    
    // Convert to NDC
    let x_ndc = (x / uniforms.screen_size.x) * 2.0 - 1.0;
    let y_ndc = 1.0 - (y / uniforms.screen_size.y) * 2.0; // Flip Y for screen coordinates
    
    output.position = vec4<f32>(x_ndc, y_ndc, 0.0, 1.0);
    
    // Use the line's color with background opacity
    output.color = vec4<f32>(label.color.rgb, label.color.a * 0.9);
    
    // UV coordinates for potential text rendering
    var u: f32;
    var v: f32;
    if (vertex_index == 0u || vertex_index == 2u) {
        u = 0.0; // Left side
    } else {
        u = 1.0; // Right side
    }
    if (vertex_index == 0u || vertex_index == 1u) {
        v = 0.0; // Top
    } else {
        v = 1.0; // Bottom
    }
    output.uv = vec2<f32>(u, v);
    
    return output;
}

// Fragment shader
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}