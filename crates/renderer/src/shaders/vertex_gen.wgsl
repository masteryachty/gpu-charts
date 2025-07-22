// GPU-driven vertex generation compute shader
// This shader generates vertices entirely on the GPU based on input data and LOD

struct VertexGenParams {
    viewport_start: f32,
    viewport_end: f32,
    screen_width: f32,
    screen_height: f32,
    total_points: u32,
    lod_factor: f32,
    min_pixel_spacing: f32,
    estimated_output_vertices: u32,
    zoom_level: f32,
    _padding: vec3<f32>,
}

struct GpuVertex {
    position: vec2<f32>,
    color: vec4<f32>,
}

struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

// Input data buffer (timestamps or values)
@group(0) @binding(0) var<storage, read> input_data: array<f32>;

// Output vertex buffer
@group(0) @binding(1) var<storage, read_write> output_vertices: array<GpuVertex>;

// Indirect draw buffer
@group(0) @binding(2) var<storage, read_write> indirect_args: DrawIndirectArgs;

// Parameters
@group(0) @binding(3) var<uniform> params: VertexGenParams;

// Atomic counter for output vertices
var<workgroup> output_counter: atomic<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    // Check bounds
    if (index >= params.total_points) {
        return;
    }
    
    // Apply LOD sampling
    if (params.lod_factor < 1.0) {
        // Simple LOD: skip points based on factor
        let lod_skip = u32(1.0 / params.lod_factor);
        if (index % lod_skip != 0u) {
            return;
        }
    }
    
    // Read input value (timestamp or data point)
    let value = input_data[index];
    
    // Check if point is within viewport
    if (value < params.viewport_start || value > params.viewport_end) {
        return;
    }
    
    // Calculate screen position
    let normalized_x = (value - params.viewport_start) / (params.viewport_end - params.viewport_start);
    let screen_x = normalized_x * params.screen_width;
    
    // Check minimum pixel spacing (avoid overdraw)
    if (index > 0u) {
        let prev_value = input_data[index - 1u];
        let prev_normalized_x = (prev_value - params.viewport_start) / (params.viewport_end - params.viewport_start);
        let prev_screen_x = prev_normalized_x * params.screen_width;
        
        if (abs(screen_x - prev_screen_x) < params.min_pixel_spacing) {
            return;
        }
    }
    
    // Get output index
    let output_index = atomicAdd(&output_counter, 1u);
    
    // Safety check
    if (output_index >= params.estimated_output_vertices) {
        atomicSub(&output_counter, 1u);
        return;
    }
    
    // Create vertex
    var vertex: GpuVertex;
    
    // Convert to NDC coordinates (-1 to 1)
    vertex.position.x = (screen_x / params.screen_width) * 2.0 - 1.0;
    vertex.position.y = 0.0; // Will be set by value data in render pass
    
    // Set color based on zoom level (example: fade out when zoomed out)
    let alpha = mix(0.3, 1.0, params.zoom_level);
    vertex.color = vec4<f32>(1.0, 1.0, 1.0, alpha);
    
    // Write vertex
    output_vertices[output_index] = vertex;
    
    // Update indirect draw arguments (only by first thread)
    if (global_id.x == 0u) {
        indirect_args.vertex_count = atomicLoad(&output_counter);
        indirect_args.instance_count = 1u;
        indirect_args.first_vertex = 0u;
        indirect_args.first_instance = 0u;
    }
}

// Advanced vertex generation with multiple data columns
@compute @workgroup_size(256)
fn main_multi_column(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let index = global_id.x;
    
    // Shared memory for coalesced reads
    var<workgroup> shared_data: array<f32, 256>;
    
    // Load data into shared memory
    if (index < params.total_points) {
        shared_data[local_id.x] = input_data[index];
    }
    workgroupBarrier();
    
    // Check bounds
    if (index >= params.total_points) {
        return;
    }
    
    let value = shared_data[local_id.x];
    
    // Viewport culling
    if (value < params.viewport_start || value > params.viewport_end) {
        return;
    }
    
    // Calculate position with subpixel precision
    let viewport_range = params.viewport_end - params.viewport_start;
    let normalized_pos = (value - params.viewport_start) / viewport_range;
    
    // Apply non-linear scaling for better visualization at different zoom levels
    let scaled_pos = apply_zoom_scaling(normalized_pos, params.zoom_level);
    
    // Convert to screen space
    let screen_x = scaled_pos * params.screen_width;
    
    // Advanced LOD with perceptual importance
    let importance = calculate_point_importance(index, params.total_points);
    if (importance < (1.0 - params.lod_factor)) {
        return;
    }
    
    // Get output slot
    let output_index = atomicAdd(&output_counter, 1u);
    if (output_index >= params.estimated_output_vertices) {
        atomicSub(&output_counter, 1u);
        return;
    }
    
    // Generate vertex with enhanced attributes
    var vertex: GpuVertex;
    vertex.position.x = (screen_x / params.screen_width) * 2.0 - 1.0;
    vertex.position.y = 0.0;
    
    // Dynamic coloring based on density
    let density = estimate_local_density(index, local_id.x);
    vertex.color = density_to_color(density, params.zoom_level);
    
    output_vertices[output_index] = vertex;
}

// Helper functions

fn apply_zoom_scaling(normalized_pos: f32, zoom: f32) -> f32 {
    // Apply logarithmic scaling for better detail at high zoom
    if (zoom > 2.0) {
        let log_zoom = log2(zoom);
        return normalized_pos * (1.0 + log_zoom * 0.1);
    }
    return normalized_pos;
}

fn calculate_point_importance(index: u32, total_points: u32) -> f32 {
    // Points at peaks and valleys are more important
    // This is a simplified version - real implementation would analyze local extrema
    let position_factor = f32(index) / f32(total_points);
    
    // Give higher importance to points in the middle of the dataset
    let center_bias = 1.0 - abs(position_factor - 0.5) * 2.0;
    
    return mix(0.5, 1.0, center_bias);
}

fn estimate_local_density(global_index: u32, local_index: u32) -> f32 {
    // Estimate density by checking neighboring points in shared memory
    var density = 0.0;
    let check_radius = 5u;
    
    for (var i = 0u; i < check_radius; i = i + 1u) {
        if (local_index >= i && local_index + i < 256u) {
            let left_idx = local_index - i;
            let right_idx = local_index + i;
            
            // Simple density: inverse of spacing
            let spacing = abs(shared_data[right_idx] - shared_data[left_idx]);
            if (spacing > 0.0) {
                density += 1.0 / spacing;
            }
        }
    }
    
    return clamp(density / f32(check_radius), 0.0, 1.0);
}

fn density_to_color(density: f32, zoom: f32) -> vec4<f32> {
    // Color based on point density
    // High density = blue, low density = red
    let r = 1.0 - density;
    let b = density;
    let g = 0.5;
    
    // Adjust alpha based on zoom
    let alpha = mix(0.3, 1.0, clamp(zoom, 0.0, 1.0));
    
    return vec4<f32>(r, g, b, alpha);
}