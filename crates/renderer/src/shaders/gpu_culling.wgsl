// Advanced GPU culling with binary search and hierarchical culling
// This shader performs high-performance culling entirely on the GPU

struct CullParams {
    viewport_start: f32,
    viewport_end: f32,
    data_count: u32,
    cull_mode: u32, // 0: frustum, 1: occlusion, 2: hierarchical
    screen_width: f32,
    screen_height: f32,
    min_pixel_size: f32,
    enable_lod: u32,
}

struct CullResult {
    start_index: u32,
    end_index: u32,
    visible_count: u32,
    lod_level: u32,
}

// Input buffers
@group(0) @binding(0) var<uniform> params: CullParams;
@group(0) @binding(1) var<storage, read> timestamps: array<f32>;
@group(0) @binding(2) var<storage, read> values: array<f32>;
@group(0) @binding(3) var<storage, read_write> visibility_mask: array<u32>;
@group(0) @binding(4) var<storage, read_write> cull_result: CullResult;

// Workgroup shared memory for collaborative culling
var<workgroup> shared_min_time: atomic<u32>;
var<workgroup> shared_max_time: atomic<u32>;
var<workgroup> shared_visible_count: atomic<u32>;
var<workgroup> workgroup_bounds: array<vec2<f32>, 256>;

// GPU binary search implementation
fn gpu_binary_search(target: f32, start_idx: u32, end_idx: u32) -> u32 {
    var left = start_idx;
    var right = end_idx;
    
    while (left < right) {
        let mid = left + (right - left) / 2u;
        if (timestamps[mid] < target) {
            left = mid + 1u;
        } else {
            right = mid;
        }
    }
    
    return left;
}

@compute @workgroup_size(256)
fn frustum_cull(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    let local_idx = local_id.x;
    
    // Initialize workgroup shared memory
    if (local_idx == 0u) {
        atomicStore(&shared_min_time, 0xFFFFFFFFu);
        atomicStore(&shared_max_time, 0u);
        atomicStore(&shared_visible_count, 0u);
    }
    workgroupBarrier();
    
    // Check bounds
    if (thread_idx >= params.data_count) {
        return;
    }
    
    // Load data
    let timestamp = timestamps[thread_idx];
    let value = values[thread_idx];
    
    // Frustum culling - check if point is within viewport
    let in_viewport = timestamp >= params.viewport_start && timestamp <= params.viewport_end;
    
    // Write visibility
    visibility_mask[thread_idx] = select(0u, 1u, in_viewport);
    
    if (in_viewport) {
        // Update workgroup bounds
        atomicMin(&shared_min_time, bitcast<u32>(timestamp));
        atomicMax(&shared_max_time, bitcast<u32>(timestamp));
        atomicAdd(&shared_visible_count, 1u);
        
        // Store bounds for LOD calculation
        workgroup_bounds[local_idx] = vec2<f32>(timestamp, value);
    }
    
    workgroupBarrier();
    
    // Final thread in workgroup updates global result
    if (local_idx == 255u || thread_idx == params.data_count - 1u) {
        let visible = atomicLoad(&shared_visible_count);
        if (visible > 0u) {
            // Update global culling result
            atomicMin(&cull_result.start_index, workgroup_id.x * 256u);
            atomicMax(&cull_result.end_index, min(workgroup_id.x * 256u + 256u, params.data_count));
            atomicAdd(&cull_result.visible_count, visible);
        }
    }
}

@compute @workgroup_size(64)
fn hierarchical_cull(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    
    // Hierarchical culling using mip-levels
    let level_0_size = params.data_count;
    let level_1_size = (level_0_size + 3u) / 4u;
    let level_2_size = (level_1_size + 3u) / 4u;
    
    // Process at current hierarchy level
    if (thread_idx < level_2_size) {
        // Level 2 - coarsest level
        let base_idx = thread_idx * 16u;
        var level_visible = false;
        
        // Check if any point in this 16-point block is visible
        for (var i = 0u; i < 16u; i = i + 1u) {
            let idx = base_idx + i;
            if (idx < params.data_count) {
                let t = timestamps[idx];
                if (t >= params.viewport_start && t <= params.viewport_end) {
                    level_visible = true;
                    break;
                }
            }
        }
        
        // If visible at coarse level, refine at finer levels
        if (level_visible) {
            // Mark entire block for finer processing
            for (var i = 0u; i < 16u; i = i + 1u) {
                let idx = base_idx + i;
                if (idx < params.data_count) {
                    visibility_mask[idx] = 1u;
                }
            }
        }
    }
}

@compute @workgroup_size(256)
fn occlusion_cull(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    let local_idx = local_id.x;
    
    if (thread_idx >= params.data_count) {
        return;
    }
    
    // Load point data
    let timestamp = timestamps[thread_idx];
    let value = values[thread_idx];
    
    // First pass: basic viewport culling
    if (timestamp < params.viewport_start || timestamp > params.viewport_end) {
        visibility_mask[thread_idx] = 0u;
        return;
    }
    
    // Calculate screen position
    let normalized_x = (timestamp - params.viewport_start) / (params.viewport_end - params.viewport_start);
    let screen_x = normalized_x * params.screen_width;
    
    // Occlusion culling - check if this point would be occluded by others
    var occluded = false;
    
    // Check against neighbors in shared memory
    workgroup_bounds[local_idx] = vec2<f32>(screen_x, value);
    workgroupBarrier();
    
    // Simple occlusion test - if points are too close together, cull some
    if (local_idx > 0u) {
        let prev_screen_x = workgroup_bounds[local_idx - 1u].x;
        if (abs(screen_x - prev_screen_x) < params.min_pixel_size) {
            // Keep every Nth point based on density
            occluded = (thread_idx % 2u) == 1u;
        }
    }
    
    visibility_mask[thread_idx] = select(1u, 0u, occluded);
}

// Advanced binary search culling on GPU
@compute @workgroup_size(1)
fn binary_search_cull() {
    // Perform binary search to find viewport bounds
    let start_idx = gpu_binary_search(params.viewport_start, 0u, params.data_count);
    let end_idx = gpu_binary_search(params.viewport_end, start_idx, params.data_count);
    
    // Adjust for continuity (include one point before/after)
    let final_start = select(0u, start_idx - 1u, start_idx > 0u);
    let final_end = min(end_idx + 1u, params.data_count);
    
    // Update result
    cull_result.start_index = final_start;
    cull_result.end_index = final_end;
    cull_result.visible_count = final_end - final_start;
    
    // Determine LOD level based on visible count
    if (params.enable_lod > 0u) {
        let points_per_pixel = f32(cull_result.visible_count) / params.screen_width;
        if (points_per_pixel > 10.0) {
            cull_result.lod_level = 3u; // Very coarse
        } else if (points_per_pixel > 5.0) {
            cull_result.lod_level = 2u; // Coarse
        } else if (points_per_pixel > 2.0) {
            cull_result.lod_level = 1u; // Medium
        } else {
            cull_result.lod_level = 0u; // Full detail
        }
    }
}

// Hybrid culling combining multiple techniques
@compute @workgroup_size(256)
fn hybrid_cull(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    
    // Phase 1: Coarse binary search (done by first workgroup)
    if (workgroup_id.x == 0u && local_id.x == 0u) {
        binary_search_cull();
    }
    workgroupBarrier();
    
    // Phase 2: Fine-grained visibility within bounds
    if (thread_idx >= cull_result.start_index && thread_idx < cull_result.end_index) {
        let timestamp = timestamps[thread_idx];
        let value = values[thread_idx];
        
        // Apply LOD culling
        var visible = true;
        if (cull_result.lod_level > 0u) {
            // Simple LOD: keep every Nth point
            let lod_skip = 1u << cull_result.lod_level;
            visible = (thread_idx % lod_skip) == 0u;
        }
        
        // Apply pixel-space culling
        if (visible && params.min_pixel_size > 0.0) {
            let normalized_x = (timestamp - params.viewport_start) / (params.viewport_end - params.viewport_start);
            let screen_x = normalized_x * params.screen_width;
            
            // Check density in local neighborhood
            if (thread_idx > cull_result.start_index) {
                let prev_timestamp = timestamps[thread_idx - 1u];
                let prev_normalized_x = (prev_timestamp - params.viewport_start) / (params.viewport_end - params.viewport_start);
                let prev_screen_x = prev_normalized_x * params.screen_width;
                
                if (abs(screen_x - prev_screen_x) < params.min_pixel_size) {
                    visible = false;
                }
            }
        }
        
        visibility_mask[thread_idx] = select(0u, 1u, visible);
    } else {
        visibility_mask[thread_idx] = 0u;
    }
}