// GPU Vertex Generation Compute Shader for Chart Rendering
// Generates vertices directly on GPU from raw data, with LOD and culling

struct VertexGenParams {
    viewport_start: f32,
    viewport_end: f32,
    screen_width: f32,
    screen_height: f32,
    total_points: u32,
    lod_factor: f32,
    min_pixel_spacing: f32,
    output_vertex_count: u32,
    zoom_level: f32,
    _padding: vec3<f32>,
}

struct GpuVertex {
    position: vec2<f32>,
    color: vec4<f32>,
    _padding: vec2<f32>,
}

struct DrawIndirectArgs {
    vertex_count: atomic<u32>,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

// Input data buffer (timestamps)
@group(0) @binding(0) var<storage, read> input_data: array<u32>;

// Output vertex buffer
@group(0) @binding(1) var<storage, read_write> output_vertices: array<GpuVertex>;

// Indirect draw arguments
@group(0) @binding(2) var<storage, read_write> indirect_args: DrawIndirectArgs;

// Parameters
@group(0) @binding(3) var<uniform> params: VertexGenParams;

// Workgroup shared memory for output counter
var<workgroup> output_counter: atomic<u32>;
var<workgroup> local_max_spacing: atomic<u32>;

@compute @workgroup_size(256)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let index = global_id.x;
    
    // Initialize workgroup atomics
    if (local_id.x == 0u) {
        atomicStore(&output_counter, 0u);
        atomicStore(&local_max_spacing, 0u);
    }
    workgroupBarrier();
    
    // Check bounds
    if (index >= params.total_points) {
        return;
    }
    
    // Read timestamp value
    let timestamp = input_data[index];
    let time_f32 = f32(timestamp);
    
    // Viewport culling - skip points outside visible range
    if (time_f32 < params.viewport_start || time_f32 > params.viewport_end) {
        return;
    }
    
    // Apply LOD based on zoom level
    if (params.lod_factor < 1.0) {
        // Skip points based on LOD factor
        let lod_skip = u32(1.0 / params.lod_factor);
        if (index % lod_skip != 0u) {
            return;
        }
    }
    
    // Calculate screen position
    let viewport_range = params.viewport_end - params.viewport_start;
    let normalized_x = (time_f32 - params.viewport_start) / viewport_range;
    let screen_x = normalized_x * params.screen_width;
    
    // Pixel-level culling - avoid generating vertices too close together
    var should_generate = true;
    if (index > 0u && params.min_pixel_spacing > 0.0) {
        let prev_timestamp = input_data[index - 1u];
        let prev_time_f32 = f32(prev_timestamp);
        
        if (prev_time_f32 >= params.viewport_start) {
            let prev_normalized_x = (prev_time_f32 - params.viewport_start) / viewport_range;
            let prev_screen_x = prev_normalized_x * params.screen_width;
            let pixel_spacing = abs(screen_x - prev_screen_x);
            
            if (pixel_spacing < params.min_pixel_spacing) {
                should_generate = false;
            }
            
            // Track maximum spacing in workgroup for adaptive LOD
            atomicMax(&local_max_spacing, u32(pixel_spacing));
        }
    }
    
    if (!should_generate) {
        return;
    }
    
    // Reserve output slot
    let local_output_idx = atomicAdd(&output_counter, 1u);
    
    // Sync before accessing workgroup counter
    workgroupBarrier();
    
    // Calculate global output index
    let workgroup_offset = workgroup_id.x * 256u;
    let output_index = workgroup_offset + local_output_idx;
    
    // Safety check
    if (output_index >= params.output_vertex_count) {
        return;
    }
    
    // Create vertex
    var vertex: GpuVertex;
    
    // Convert to NDC coordinates (-1 to 1)
    vertex.position.x = (screen_x / params.screen_width) * 2.0 - 1.0;
    vertex.position.y = 0.0; // Y will be set by value data in render pass
    
    // Color based on zoom level and density
    let base_alpha = mix(0.5, 1.0, clamp(params.zoom_level / 10.0, 0.0, 1.0));
    
    // Adaptive opacity based on point density
    let max_spacing = f32(atomicLoad(&local_max_spacing));
    let density_factor = 1.0 - clamp(params.min_pixel_spacing / max_spacing, 0.0, 1.0);
    let alpha = base_alpha * mix(0.7, 1.0, density_factor);
    
    vertex.color = vec4<f32>(1.0, 1.0, 1.0, alpha);
    vertex._padding = vec2<f32>(0.0, 0.0);
    
    // Write vertex
    output_vertices[output_index] = vertex;
    
    // Update indirect draw arguments
    // Only one thread per workgroup should update the global counter
    if (local_id.x == 0u) {
        let vertices_in_workgroup = atomicLoad(&output_counter);
        atomicAdd(&indirect_args.vertex_count, vertices_in_workgroup);
    }
}

// Alternative entry point for value data generation
@compute @workgroup_size(256)
fn generate_with_values(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let index = global_id.x;
    
    if (index >= params.total_points) {
        return;
    }
    
    // For this variant, input_data contains interleaved time/value pairs
    let time_idx = index * 2u;
    let value_idx = time_idx + 1u;
    
    let timestamp = input_data[time_idx];
    let value_bits = input_data[value_idx];
    let value = bitcast<f32>(value_bits);
    
    let time_f32 = f32(timestamp);
    
    // Viewport culling
    if (time_f32 < params.viewport_start || time_f32 > params.viewport_end) {
        return;
    }
    
    // Calculate positions
    let viewport_range = params.viewport_end - params.viewport_start;
    let normalized_x = (time_f32 - params.viewport_start) / viewport_range;
    
    // For Y, we need value bounds (could be passed in params)
    let normalized_y = value; // Assumes pre-normalized values
    
    // Convert to NDC
    let ndc_x = normalized_x * 2.0 - 1.0;
    let ndc_y = normalized_y * 2.0 - 1.0;
    
    // Get output slot
    let output_idx = atomicAdd(&indirect_args.vertex_count, 1u);
    if (output_idx >= params.output_vertex_count) {
        atomicSub(&indirect_args.vertex_count, 1u);
        return;
    }
    
    // Create complete vertex with position
    var vertex: GpuVertex;
    vertex.position = vec2<f32>(ndc_x, ndc_y);
    vertex.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    vertex._padding = vec2<f32>(0.0, 0.0);
    
    output_vertices[output_idx] = vertex;
}