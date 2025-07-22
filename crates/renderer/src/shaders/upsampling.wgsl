// High-quality upsampling shader for multi-resolution rendering
// Uses bilinear filtering with edge-aware sharpening

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Fullscreen triangle vertex shader
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    // Generate fullscreen triangle
    let x = f32((vertex_index << 1u) & 2u);
    let y = f32(vertex_index & 2u);
    
    output.position = vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
    output.uv = vec2<f32>(x, 1.0 - y);
    
    return output;
}

@group(0) @binding(0) var low_res_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> upsampling_params: UpsamplingParams;

struct UpsamplingParams {
    source_resolution: vec2<f32>,
    target_resolution: vec2<f32>,
    sharpening_strength: f32,
    edge_threshold: f32,
    temporal_blend: f32,
    _padding: f32,
}

// Catmull-Rom interpolation for smoother upsampling
fn catmull_rom(p0: vec4<f32>, p1: vec4<f32>, p2: vec4<f32>, p3: vec4<f32>, t: f32) -> vec4<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    
    return 0.5 * (
        (2.0 * p1) +
        (-p0 + p2) * t +
        (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
        (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
    );
}

// Sample with edge-aware filtering
fn sample_edge_aware(uv: vec2<f32>) -> vec4<f32> {
    let texel_size = 1.0 / upsampling_params.source_resolution;
    
    // Sample 3x3 neighborhood
    var samples: array<vec4<f32>, 9>;
    var index = 0u;
    
    for (var y = -1; y <= 1; y = y + 1) {
        for (var x = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel_size;
            samples[index] = textureSample(low_res_texture, texture_sampler, uv + offset);
            index = index + 1u;
        }
    }
    
    // Calculate edge strength
    let center = samples[4];
    var edge_sum = vec4<f32>(0.0);
    
    for (var i = 0u; i < 9u; i = i + 1u) {
        if (i != 4u) {
            edge_sum += abs(samples[i] - center);
        }
    }
    
    let edge_strength = length(edge_sum) / 8.0;
    
    // Apply edge-aware filtering
    if (edge_strength > upsampling_params.edge_threshold) {
        // Sharp edge - use bilinear
        return textureSample(low_res_texture, texture_sampler, uv);
    } else {
        // Smooth area - use bicubic
        return sample_bicubic(uv);
    }
}

// Bicubic sampling for smooth areas
fn sample_bicubic(uv: vec2<f32>) -> vec4<f32> {
    let texel_size = 1.0 / upsampling_params.source_resolution;
    let pixel = uv * upsampling_params.source_resolution;
    let pixel_floor = floor(pixel);
    let frac = pixel - pixel_floor;
    
    // Sample 4x4 grid
    var rows: array<vec4<f32>, 4>;
    
    for (var y = -1; y <= 2; y = y + 1) {
        var row_samples: array<vec4<f32>, 4>;
        
        for (var x = -1; x <= 2; x = x + 1) {
            let sample_pos = (pixel_floor + vec2<f32>(f32(x), f32(y)) + 0.5) * texel_size;
            row_samples[x + 1] = textureSample(low_res_texture, texture_sampler, sample_pos);
        }
        
        rows[y + 1] = catmull_rom(row_samples[0], row_samples[1], row_samples[2], row_samples[3], frac.x);
    }
    
    return catmull_rom(rows[0], rows[1], rows[2], rows[3], frac.y);
}

// Lanczos filter for high-quality upsampling
fn lanczos(x: f32, a: f32) -> f32 {
    if (abs(x) < 0.0001) { return 1.0; }
    if (abs(x) >= a) { return 0.0; }
    
    let pi_x = x * 3.14159265359;
    let pi_x_over_a = pi_x / a;
    
    return (sin(pi_x) / pi_x) * (sin(pi_x_over_a) / pi_x_over_a);
}

// High-quality Lanczos upsampling
fn sample_lanczos(uv: vec2<f32>) -> vec4<f32> {
    let a = 3.0; // Lanczos kernel size
    let texel_size = 1.0 / upsampling_params.source_resolution;
    let pixel = uv * upsampling_params.source_resolution;
    let pixel_center = floor(pixel) + 0.5;
    
    var color = vec4<f32>(0.0);
    var weight_sum = 0.0;
    
    for (var y = -3; y <= 3; y = y + 1) {
        for (var x = -3; x <= 3; x = x + 1) {
            let sample_pos = pixel_center + vec2<f32>(f32(x), f32(y));
            let delta = pixel - sample_pos;
            
            let weight = lanczos(delta.x, a) * lanczos(delta.y, a);
            
            if (weight > 0.0) {
                let sample_uv = sample_pos * texel_size;
                color += textureSample(low_res_texture, texture_sampler, sample_uv) * weight;
                weight_sum += weight;
            }
        }
    }
    
    return color / weight_sum;
}

// Fragment shader with multiple upsampling modes
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let scale_factor = upsampling_params.target_resolution / upsampling_params.source_resolution;
    
    var color: vec4<f32>;
    
    // Choose upsampling method based on scale factor
    if (scale_factor.x <= 2.0 && scale_factor.y <= 2.0) {
        // Small upscale - use edge-aware filtering
        color = sample_edge_aware(input.uv);
    } else if (scale_factor.x <= 4.0 && scale_factor.y <= 4.0) {
        // Medium upscale - use bicubic
        color = sample_bicubic(input.uv);
    } else {
        // Large upscale - use Lanczos
        color = sample_lanczos(input.uv);
    }
    
    // Apply sharpening
    if (upsampling_params.sharpening_strength > 0.0) {
        let texel_size = 1.0 / upsampling_params.target_resolution;
        
        // Sample neighbors for sharpening
        let n = textureSample(low_res_texture, texture_sampler, input.uv + vec2<f32>(0.0, -texel_size.y));
        let s = textureSample(low_res_texture, texture_sampler, input.uv + vec2<f32>(0.0, texel_size.y));
        let e = textureSample(low_res_texture, texture_sampler, input.uv + vec2<f32>(texel_size.x, 0.0));
        let w = textureSample(low_res_texture, texture_sampler, input.uv + vec2<f32>(-texel_size.x, 0.0));
        
        let laplacian = (n + s + e + w) - 4.0 * color;
        color = color - laplacian * upsampling_params.sharpening_strength;
    }
    
    // Clamp to valid range
    return clamp(color, vec4<f32>(0.0), vec4<f32>(1.0));
}

// Advanced temporal upsampling shader
@fragment
fn fs_temporal(input: VertexOutput) -> @location(0) vec4<f32> {
    // Current frame
    let current = fs_main(input);
    
    // TODO: Sample previous frame with motion vectors
    // For now, just return current frame
    return current;
}

// Debug visualization shader
@fragment
fn fs_debug(input: VertexOutput) -> @location(0) vec4<f32> {
    // Visualize upsampling quality
    let scale_factor = upsampling_params.target_resolution / upsampling_params.source_resolution;
    let quality = 1.0 - (length(scale_factor - vec2<f32>(1.0)) / 16.0);
    
    // Color code by quality
    var color: vec3<f32>;
    if (quality > 0.8) {
        color = vec3<f32>(0.0, 1.0, 0.0); // Green - high quality
    } else if (quality > 0.5) {
        color = vec3<f32>(1.0, 1.0, 0.0); // Yellow - medium quality
    } else {
        color = vec3<f32>(1.0, 0.0, 0.0); // Red - low quality
    }
    
    // Mix with actual upsampled image
    let image = fs_main(input);
    return vec4<f32>(mix(image.rgb, color, 0.2), 1.0);
}