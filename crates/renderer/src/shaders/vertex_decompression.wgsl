// GPU-based vertex decompression for runtime rendering
// Decompresses 8-byte and 4-byte vertex formats back to full precision

struct DecompressionParams {
    vertex_count: u32,
    time_min: f32,
    time_max: f32,
    value_min: f32,
    value_max: f32,
    compression_mode: u32,
    _padding: vec2<u32>,
}

struct CompressedVertex {
    time_value: u32,
    metadata: u32,
}

// Unpack u16 to normalized float
fn unpack_from_u16(packed: u32, min_val: f32, max_val: f32) -> f32 {
    let normalized = f32(packed) / 65535.0;
    return min_val + normalized * (max_val - min_val);
}

// Unpack u12 to normalized float
fn unpack_from_u12(packed: u32, min_val: f32, max_val: f32) -> f32 {
    let normalized = f32(packed & 0xFFFu) / 4095.0;
    return min_val + normalized * (max_val - min_val);
}

// Vertex shader for decompressing vertices on the fly
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> compressed_vertices: array<CompressedVertex>;
@group(0) @binding(1) var<uniform> params: DecompressionParams;
@group(0) @binding(2) var<uniform> transform: mat4x4<f32>;

@vertex
fn vs_decompress(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    let compressed = compressed_vertices[vertex_idx];
    var output: VertexOutput;
    
    var time: f32;
    var value: f32;
    
    // Check compression type from metadata
    let compression_type = (compressed.metadata >> 30u) & 0x3u;
    
    if (compression_type == 0u) {
        // Standard 8-byte compression (16-bit precision)
        let time_u16 = compressed.time_value >> 16u;
        let value_u16 = compressed.time_value & 0xFFFFu;
        
        time = unpack_from_u16(time_u16, params.time_min, params.time_max);
        value = unpack_from_u16(value_u16, params.value_min, params.value_max);
    } else if (compression_type == 1u) {
        // Ultra 4-byte compression (12-bit precision)
        let time_u12 = compressed.time_value >> 20u;
        let value_u12 = (compressed.time_value >> 8u) & 0xFFFu;
        
        time = unpack_from_u12(time_u12, params.time_min, params.time_max);
        value = unpack_from_u12(value_u12, params.value_min, params.value_max);
    } else if (compression_type == 2u) {
        // Delta compression
        if (vertex_idx == 0u || (compressed.metadata & 0x40000000u) != 0u) {
            // Absolute value
            let time_u16 = compressed.time_value >> 16u;
            let value_u16 = compressed.time_value & 0xFFFFu;
            
            time = unpack_from_u16(time_u16, params.time_min, params.time_max);
            value = unpack_from_u16(value_u16, params.value_min, params.value_max);
        } else {
            // Delta value - need to accumulate from previous
            // In practice, this would require a prefix sum pass
            time = 0.0; // Placeholder
            value = 0.0; // Placeholder
        }
    }
    
    // Transform to screen space
    let x = (time - params.time_min) / (params.time_max - params.time_min) * 2.0 - 1.0;
    let y = (value - params.value_min) / (params.value_max - params.value_min) * 2.0 - 1.0;
    
    output.position = transform * vec4<f32>(x, y, 0.0, 1.0);
    
    // Extract color from metadata
    let color_index = (compressed.metadata >> 24u) & 0xFFu;
    let color_normalized = f32(color_index) / 255.0;
    
    // Create color based on value
    if (color_normalized > 0.5) {
        output.color = vec4<f32>(1.0, color_normalized, 0.0, 1.0); // Yellow to red
    } else {
        output.color = vec4<f32>(0.0, color_normalized * 2.0, 1.0, 1.0); // Blue to green
    }
    
    output.uv = vec2<f32>(x * 0.5 + 0.5, y * 0.5 + 0.5);
    
    return output;
}

// Compute shader for batch decompression
@compute @workgroup_size(256)
fn decompress_batch(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    
    if (thread_idx >= params.vertex_count) {
        return;
    }
    
    let compressed = compressed_vertices[thread_idx];
    
    // Decompress based on format
    var time: f32;
    var value: f32;
    
    let time_u16 = compressed.time_value >> 16u;
    let value_u16 = compressed.time_value & 0xFFFFu;
    
    time = unpack_from_u16(time_u16, params.time_min, params.time_max);
    value = unpack_from_u16(value_u16, params.value_min, params.value_max);
    
    // Write to output buffer (would need additional binding)
    // output_vertices[thread_idx] = vec2<f32>(time, value);
}

// Specialized decompression for line strips
@vertex
fn vs_decompress_line_strip(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32
) -> VertexOutput {
    var output: VertexOutput;
    
    // For line strips, we need to handle connectivity
    let compressed = compressed_vertices[vertex_idx];
    
    // Decompress vertex
    let time_u16 = compressed.time_value >> 16u;
    let value_u16 = compressed.time_value & 0xFFFFu;
    
    let time = unpack_from_u16(time_u16, params.time_min, params.time_max);
    let value = unpack_from_u16(value_u16, params.value_min, params.value_max);
    
    // Apply instance offset for multi-line rendering
    let instance_offset = f32(instance_idx) * 0.01;
    
    let x = (time - params.time_min) / (params.time_max - params.time_min) * 2.0 - 1.0;
    let y = (value - params.value_min) / (params.value_max - params.value_min) * 2.0 - 1.0 + instance_offset;
    
    output.position = transform * vec4<f32>(x, y, 0.0, 1.0);
    
    // Line color based on instance
    let hue = f32(instance_idx) / 10.0;
    output.color = hsv_to_rgb(vec3<f32>(hue, 1.0, 1.0));
    
    output.uv = vec2<f32>(f32(vertex_idx) / f32(params.vertex_count), 0.5);
    
    return output;
}

// Helper function for color conversion
fn hsv_to_rgb(hsv: vec3<f32>) -> vec4<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;
    
    let c = v * s;
    let x = c * (1.0 - abs(h % 2.0 - 1.0));
    let m = v - c;
    
    var rgb: vec3<f32>;
    
    if (h < 1.0) {
        rgb = vec3<f32>(c, x, 0.0);
    } else if (h < 2.0) {
        rgb = vec3<f32>(x, c, 0.0);
    } else if (h < 3.0) {
        rgb = vec3<f32>(0.0, c, x);
    } else if (h < 4.0) {
        rgb = vec3<f32>(0.0, x, c);
    } else if (h < 5.0) {
        rgb = vec3<f32>(x, 0.0, c);
    } else {
        rgb = vec3<f32>(c, 0.0, x);
    }
    
    return vec4<f32>(rgb + vec3<f32>(m), 1.0);
}

// Advanced decompression with quality reconstruction
@vertex
fn vs_decompress_adaptive(
    @builtin(vertex_index) vertex_idx: u32
) -> VertexOutput {
    var output: VertexOutput;
    
    let compressed = compressed_vertices[vertex_idx];
    
    // Check if high precision flag is set
    let is_high_precision = (compressed.metadata & 0x80000000u) != 0u;
    
    var time: f32;
    var value: f32;
    
    if (is_high_precision) {
        // Full 16-bit precision
        let time_u16 = compressed.time_value >> 16u;
        let value_u16 = compressed.time_value & 0xFFFFu;
        
        time = unpack_from_u16(time_u16, params.time_min, params.time_max);
        value = unpack_from_u16(value_u16, params.value_min, params.value_max);
    } else {
        // Reduced 12-bit precision with interpolation
        let time_u12 = compressed.time_value >> 20u;
        let value_u12 = (compressed.time_value >> 8u) & 0xFFFu;
        
        time = unpack_from_u12(time_u12, params.time_min, params.time_max);
        value = unpack_from_u12(value_u12, params.value_min, params.value_max);
        
        // Apply smoothing for low precision data
        if (vertex_idx > 0u && vertex_idx < params.vertex_count - 1u) {
            let prev = compressed_vertices[vertex_idx - 1u];
            let next = compressed_vertices[vertex_idx + 1u];
            
            // Simple smoothing (would be more sophisticated in practice)
            let prev_value = unpack_from_u12(
                (prev.time_value >> 8u) & 0xFFFu,
                params.value_min,
                params.value_max
            );
            let next_value = unpack_from_u12(
                (next.time_value >> 8u) & 0xFFFu,
                params.value_min,
                params.value_max
            );
            
            value = value * 0.5 + (prev_value + next_value) * 0.25;
        }
    }
    
    // Transform to screen space
    let x = (time - params.time_min) / (params.time_max - params.time_min) * 2.0 - 1.0;
    let y = (value - params.value_min) / (params.value_max - params.value_min) * 2.0 - 1.0;
    
    output.position = transform * vec4<f32>(x, y, 0.0, 1.0);
    
    // Color based on precision level
    if (is_high_precision) {
        output.color = vec4<f32>(0.0, 1.0, 0.0, 1.0); // Green for high precision
    } else {
        output.color = vec4<f32>(1.0, 1.0, 0.0, 1.0); // Yellow for low precision
    }
    
    output.uv = vec2<f32>(time, value);
    
    return output;
}