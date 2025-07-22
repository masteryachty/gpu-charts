// Delta compression for time-series data
// Achieves high compression ratios by storing differences between consecutive values

struct DeltaCompressionParams {
    vertex_count: u32,
    base_time: f32,
    base_value: f32,
    time_scale: f32,
    value_scale: f32,
    block_size: u32,
    _padding: vec2<u32>,
}

struct Vertex {
    time: f32,
    value: f32,
}

struct DeltaBlock {
    base_time: f32,
    base_value: f32,
    time_scale: f32,
    value_scale: f32,
    start_index: u32,
    count: u32,
    _padding: vec2<u32>,
}

// Packed delta format (16 bits per delta)
struct PackedDelta {
    packed: u32, // time_delta (8 bits) + value_delta (8 bits) + repeat_count (16 bits)
}

@group(0) @binding(0) var<storage, read> input_vertices: array<Vertex>;
@group(0) @binding(1) var<storage, read_write> delta_blocks: array<DeltaBlock>;
@group(0) @binding(2) var<storage, read_write> packed_deltas: array<PackedDelta>;
@group(0) @binding(3) var<uniform> params: DeltaCompressionParams;

// Workgroup shared memory for collaborative compression
var<workgroup> block_stats: array<vec4<f32>, 64>; // min_time, max_time, min_value, max_value
var<workgroup> delta_histogram: array<atomic<u32>, 256>;

@compute @workgroup_size(256)
fn delta_compress(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    let local_idx = local_id.x;
    let block_idx = workgroup_id.x;
    
    // Initialize workgroup memory
    if (local_idx < 64u) {
        block_stats[local_idx] = vec4<f32>(1e10, -1e10, 1e10, -1e10);
    }
    if (local_idx < 256u) {
        atomicStore(&delta_histogram[local_idx], 0u);
    }
    workgroupBarrier();
    
    // Process vertices in blocks
    let block_start = block_idx * params.block_size;
    let block_end = min(block_start + params.block_size, params.vertex_count);
    
    if (thread_idx >= block_end) {
        return;
    }
    
    // Phase 1: Analyze block statistics
    if (thread_idx >= block_start && thread_idx < block_end) {
        let vertex = input_vertices[thread_idx];
        
        // Update block statistics
        let stat_idx = (thread_idx - block_start) / 4u;
        if (stat_idx < 64u) {
            block_stats[stat_idx].x = min(block_stats[stat_idx].x, vertex.time);
            block_stats[stat_idx].y = max(block_stats[stat_idx].y, vertex.time);
            block_stats[stat_idx].z = min(block_stats[stat_idx].z, vertex.value);
            block_stats[stat_idx].w = max(block_stats[stat_idx].w, vertex.value);
        }
    }
    workgroupBarrier();
    
    // Phase 2: Calculate optimal scaling factors
    var block_min_time = 1e10;
    var block_max_time = -1e10;
    var block_min_value = 1e10;
    var block_max_value = -1e10;
    
    if (local_idx == 0u) {
        for (var i = 0u; i < 64u; i = i + 1u) {
            block_min_time = min(block_min_time, block_stats[i].x);
            block_max_time = max(block_max_time, block_stats[i].y);
            block_min_value = min(block_min_value, block_stats[i].z);
            block_max_value = max(block_max_value, block_stats[i].w);
        }
        
        // Write block header
        var block: DeltaBlock;
        block.base_time = block_min_time;
        block.base_value = block_min_value;
        block.time_scale = (block_max_time - block_min_time) / 255.0;
        block.value_scale = (block_max_value - block_min_value) / 255.0;
        block.start_index = block_start;
        block.count = block_end - block_start;
        
        delta_blocks[block_idx] = block;
    }
    workgroupBarrier();
    
    // Phase 3: Compress deltas
    if (thread_idx > block_start && thread_idx < block_end) {
        let curr = input_vertices[thread_idx];
        let prev = input_vertices[thread_idx - 1u];
        
        let block = delta_blocks[block_idx];
        
        // Calculate deltas
        let time_delta = curr.time - prev.time;
        let value_delta = curr.value - prev.value;
        
        // Quantize deltas to 8 bits each
        let time_delta_u8 = u32(clamp(time_delta / block.time_scale, 0.0, 255.0));
        let value_delta_u8 = u32(clamp((value_delta + 127.0 * block.value_scale) / block.value_scale, 0.0, 255.0));
        
        // Update histogram for analysis
        atomicAdd(&delta_histogram[time_delta_u8], 1u);
        
        // Check for run-length encoding opportunity
        var repeat_count = 1u;
        var next_idx = thread_idx + 1u;
        
        while (next_idx < block_end && repeat_count < 65535u) {
            let next = input_vertices[next_idx];
            let next_time_delta = next.time - input_vertices[next_idx - 1u].time;
            let next_value_delta = next.value - input_vertices[next_idx - 1u].value;
            
            let next_time_u8 = u32(clamp(next_time_delta / block.time_scale, 0.0, 255.0));
            let next_value_u8 = u32(clamp((next_value_delta + 127.0 * block.value_scale) / block.value_scale, 0.0, 255.0));
            
            if (next_time_u8 == time_delta_u8 && next_value_u8 == value_delta_u8) {
                repeat_count += 1u;
                next_idx += 1u;
            } else {
                break;
            }
        }
        
        // Pack delta with repeat count
        var packed: PackedDelta;
        packed.packed = (time_delta_u8 << 24u) | (value_delta_u8 << 16u) | repeat_count;
        
        packed_deltas[thread_idx - block_start - 1u + block_start] = packed;
    }
}

// Advanced delta compression with pattern detection
@compute @workgroup_size(256)
fn delta_compress_advanced(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    
    if (thread_idx >= params.vertex_count - 1u) {
        return;
    }
    
    // Load current and next vertex
    let curr = input_vertices[thread_idx];
    let next = input_vertices[thread_idx + 1u];
    
    // Calculate first-order delta
    let delta1_time = next.time - curr.time;
    let delta1_value = next.value - curr.value;
    
    // Calculate second-order delta for trend detection
    var delta2_time = 0.0;
    var delta2_value = 0.0;
    
    if (thread_idx > 0u) {
        let prev = input_vertices[thread_idx - 1u];
        let prev_delta_time = curr.time - prev.time;
        let prev_delta_value = curr.value - prev.value;
        
        delta2_time = delta1_time - prev_delta_time;
        delta2_value = delta1_value - prev_delta_value;
    }
    
    // Pattern detection
    var pattern_type = 0u; // 0: random, 1: linear, 2: polynomial, 3: periodic
    
    if (abs(delta2_time) < 0.001 && abs(delta2_value) < 0.001) {
        pattern_type = 1u; // Linear pattern
    } else if (thread_idx > 2u) {
        // Check for polynomial pattern
        let prev2 = input_vertices[thread_idx - 2u];
        let prev3_delta = (curr.value - prev2.value) / (curr.time - prev2.time);
        let curr_delta = delta1_value / delta1_time;
        
        if (abs(curr_delta - prev3_delta) < 0.01) {
            pattern_type = 2u; // Polynomial pattern
        }
    }
    
    // Encode based on pattern type
    var packed: PackedDelta;
    
    if (pattern_type == 1u) {
        // Linear pattern - store slope once
        let slope = delta1_value / delta1_time;
        let slope_u16 = u32(clamp((slope + 1000.0) / 2000.0 * 65535.0, 0.0, 65535.0));
        packed.packed = (1u << 30u) | slope_u16; // Pattern flag + slope
    } else {
        // Random pattern - store full deltas
        let time_delta_u8 = u32(clamp(delta1_time / params.time_scale, 0.0, 255.0));
        let value_delta_u8 = u32(clamp((delta1_value + 127.0 * params.value_scale) / params.value_scale, 0.0, 255.0));
        packed.packed = (time_delta_u8 << 24u) | (value_delta_u8 << 16u);
    }
    
    packed_deltas[thread_idx] = packed;
}

// Decompression shader for delta-compressed data
@compute @workgroup_size(256)
fn delta_decompress(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    
    if (thread_idx >= params.vertex_count) {
        return;
    }
    
    // Find which block this vertex belongs to
    let block_idx = thread_idx / params.block_size;
    let block = delta_blocks[block_idx];
    let local_vertex_idx = thread_idx - block.start_index;
    
    // Prefix sum for delta decompression
    var accumulated_time = block.base_time;
    var accumulated_value = block.base_value;
    
    // First vertex in block uses base values
    if (local_vertex_idx == 0u) {
        var vertex: Vertex;
        vertex.time = accumulated_time;
        vertex.value = accumulated_value;
        // Would write to output buffer
        return;
    }
    
    // Accumulate deltas
    for (var i = 0u; i < local_vertex_idx; i = i + 1u) {
        let delta = packed_deltas[block.start_index + i];
        
        let pattern_flag = (delta.packed >> 30u) & 0x3u;
        
        if (pattern_flag == 1u) {
            // Linear pattern
            let slope_u16 = delta.packed & 0xFFFFu;
            let slope = (f32(slope_u16) / 65535.0) * 2000.0 - 1000.0;
            
            accumulated_time += block.time_scale;
            accumulated_value += slope * block.time_scale;
        } else {
            // Random pattern
            let time_delta_u8 = (delta.packed >> 24u) & 0xFFu;
            let value_delta_u8 = (delta.packed >> 16u) & 0xFFu;
            let repeat_count = delta.packed & 0xFFFFu;
            
            let time_delta = f32(time_delta_u8) * block.time_scale;
            let value_delta = (f32(value_delta_u8) - 127.0) * block.value_scale;
            
            accumulated_time += time_delta * f32(repeat_count);
            accumulated_value += value_delta * f32(repeat_count);
        }
    }
    
    var vertex: Vertex;
    vertex.time = accumulated_time;
    vertex.value = accumulated_value;
    // Would write to output buffer
}