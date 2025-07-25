// Triangle shader for rendering trade markers

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct MinMax {
    min_val: f32,
    max_val: f32,
};

struct MinMaxU32 {
    min_val: u32,
    max_val: u32,
};

// Uniforms
@group(0) @binding(0) var<uniform> x_min_max: MinMaxU32;  // Keep as u32 for timestamp precision
@group(0) @binding(1) var<uniform> y_min_max: MinMax;
@group(0) @binding(2) var<uniform> screen_size: vec2<f32>;
@group(0) @binding(3) var<uniform> triangle_size: f32;

// Data buffers
@group(0) @binding(4) var<storage, read> time_buffer: array<u32>;
@group(0) @binding(5) var<storage, read> price_buffer: array<f32>;
@group(0) @binding(6) var<storage, read> side_buffer: array<u32>;  // Changed from f32 to u32 for proper byte interpretation

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32
) -> VertexOutput {
    var out: VertexOutput;
    
    // Read instance data from buffers
    let timestamp = time_buffer[instance_idx];  // Keep as u32
    let price = price_buffer[instance_idx];
    let side_raw = side_buffer[instance_idx];  // Read as u32 first
    let side = f32(side_raw & 0xFFu);  // Extract just the first byte (0 or 1)
    
    // Calculate normalized position (0 to 1) using u32 arithmetic to maintain precision
    let x_range = x_min_max.max_val - x_min_max.min_val;
    let x_normalized = f32(timestamp - x_min_max.min_val) / f32(x_range);
    
    // Calculate Y normalization with margin
    let y_range = y_min_max.max_val - y_min_max.min_val;
    let y_margin = y_range * 0.1;
    let y_min_with_margin = y_min_max.min_val - y_margin;
    let y_max_with_margin = y_min_max.max_val + y_margin;
    let y_range_with_margin = y_max_with_margin - y_min_with_margin;
    let y_normalized = (price - y_min_with_margin) / y_range_with_margin;
    
    // Convert to NDC (-1 to 1)
    let center_ndc = vec4<f32>(
        x_normalized * 2.0 - 1.0,
        y_normalized * 2.0 - 1.0,
        0.0,
        1.0
    );
    
    // Convert to screen space for pixel-perfect triangles
    let center_screen = vec2<f32>(
        (center_ndc.x * 0.5 + 0.5) * screen_size.x,
        (center_ndc.y * 0.5 + 0.5) * screen_size.y
    );
    
    // Generate triangle vertices in screen space
    var vertex_screen: vec2<f32>;
    let half_size = triangle_size * 0.5;
    
    // Check if this is a buy or sell trade
    // Now fixed in server: 0 = sell, 1 = buy
    let is_buy = side > 0.5;  // side 0 = sell, side 1 = buy
    
    if is_buy {
        // Buy trade - upward triangle
        switch vertex_idx {
            case 0u: {
                // Top vertex (tip pointing up)
                vertex_screen = center_screen + vec2<f32>(0.0, -half_size);
            }
            case 1u: {
                // Bottom left
                vertex_screen = center_screen + vec2<f32>(-half_size, half_size);
            }
            case 2u: {
                // Bottom right
                vertex_screen = center_screen + vec2<f32>(half_size, half_size);
            }
            default: {}
        }
    } else {
        // Sell trade - downward triangle
        switch vertex_idx {
            case 0u: {
                // Bottom vertex (tip pointing down)
                vertex_screen = center_screen + vec2<f32>(0.0, half_size);
            }
            case 1u: {
                // Top left
                vertex_screen = center_screen + vec2<f32>(-half_size, -half_size);
            }
            case 2u: {
                // Top right
                vertex_screen = center_screen + vec2<f32>(half_size, -half_size);
            }
            default: {}
        }
    }
    
    // Convert back to NDC
    out.position = vec4<f32>(
        (vertex_screen.x / screen_size.x) * 2.0 - 1.0,
        (vertex_screen.y / screen_size.y) * 2.0 - 1.0,
        0.0,
        1.0
    );
    
    // Set color based on trade side
    // Debug: make colors very distinct
    if is_buy {
        // Buy - bright green
        out.color = vec3<f32>(0.0, 1.0, 0.0);
    } else {
        // Sell - bright red
        out.color = vec3<f32>(1.0, 0.0, 0.0);
    }
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

