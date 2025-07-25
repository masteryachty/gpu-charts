// Shader for rendering computed lines (e.g., mid price)

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) line_coord: f32, // For dashed/dotted lines
}

struct LineParams {
    color: vec3<f32>,
    style: u32, // 0=solid, 1=dashed, 2=dotted
}

// Storage buffers
@group(0) @binding(0) var<storage, read> time_buffer: array<u32>;
@group(0) @binding(1) var<storage, read> value_buffer: array<f32>;

// Uniforms
@group(0) @binding(2) var<uniform> x_range: vec2<f32>;
@group(0) @binding(3) var<uniform> y_range: vec2<f32>;
@group(0) @binding(4) var<uniform> line_params: LineParams;

@vertex
fn vs_main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    var out: VertexOutput;
    
    // Read data
    let time = f32(time_buffer[vertex_idx]);
    let value = value_buffer[vertex_idx];
    
    // Skip invalid values
    if (value <= 0.0) {
        out.position = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        out.color = vec3<f32>(0.0, 0.0, 0.0);
        out.line_coord = 0.0;
        return out;
    }
    
    // Normalize to 0-1
    let x_normalized = (time - x_range.x) / (x_range.y - x_range.x);
    
    // Add margin to Y range
    let y_margin = (y_range.y - y_range.x) * 0.1;
    let y_min_with_margin = y_range.x - y_margin;
    let y_max_with_margin = y_range.y + y_margin;
    let y_normalized = (value - y_min_with_margin) / (y_max_with_margin - y_min_with_margin);
    
    // Convert to NDC
    out.position = vec4<f32>(
        x_normalized * 2.0 - 1.0,
        y_normalized * 2.0 - 1.0,
        0.0,
        1.0
    );
    
    out.color = line_params.color;
    out.line_coord = f32(vertex_idx) * 0.1; // For dashed line pattern
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var alpha = 1.0;
    
    // Apply line style
    if (line_params.style == 1u) { // Dashed
        let pattern = sin(in.line_coord * 10.0);
        if (pattern < 0.0) {
            discard;
        }
    } else if (line_params.style == 2u) { // Dotted
        let pattern = sin(in.line_coord * 20.0);
        if (pattern < -0.5) {
            discard;
        }
    }
    
    return vec4<f32>(in.color, alpha);
}