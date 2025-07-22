// GPU-based vertex compression for ultra-compact data representation
// Compresses f32 vertex pairs to 8-byte or 4-byte formats

struct CompressionParams {
    vertex_count: u32,
    time_min: f32,
    time_max: f32,
    value_min: f32,
    value_max: f32,
    compression_mode: u32, // 0: 8-byte, 1: 4-byte ultra
    _padding: vec2<u32>,
}

struct UncompressedVertex {
    time: f32,
    value: f32,
}

struct CompressedVertex8 {
    time_value: u32, // time (16 bits) + value (16 bits)
    metadata: u32,   // color (8) + flags (8) + extra (16)
}

struct CompressedVertex4 {
    packed: u32, // time (12) + value (12) + flags (8)
}

// Input/Output buffers
@group(0) @binding(0) var<storage, read> input_vertices: array<UncompressedVertex>;
@group(0) @binding(1) var<storage, read_write> output_vertices: array<CompressedVertex8>;
@group(0) @binding(2) var<uniform> params: CompressionParams;

// Shared memory for workgroup statistics
var<workgroup> min_time: atomic<u32>;
var<workgroup> max_time: atomic<u32>;
var<workgroup> min_value: atomic<u32>;
var<workgroup> max_value: atomic<u32>;

// Pack float to normalized u16
fn pack_to_u16(value: f32, min_val: f32, max_val: f32) -> u32 {
    let normalized = clamp((value - min_val) / (max_val - min_val), 0.0, 1.0);
    return u32(normalized * 65535.0);
}

// Pack float to normalized u12
fn pack_to_u12(value: f32, min_val: f32, max_val: f32) -> u32 {
    let normalized = clamp((value - min_val) / (max_val - min_val), 0.0, 1.0);
    return u32(normalized * 4095.0);
}

@compute @workgroup_size(256)
fn compress_vertices(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    
    // Initialize workgroup atomics
    if (local_id.x == 0u) {
        atomicStore(&min_time, 0xFFFFFFFFu);
        atomicStore(&max_time, 0u);
        atomicStore(&min_value, 0xFFFFFFFFu);
        atomicStore(&max_value, 0u);
    }
    workgroupBarrier();
    
    if (thread_idx >= params.vertex_count) {
        return;
    }
    
    // Load vertex
    let vertex = input_vertices[thread_idx];
    
    // Standard 8-byte compression
    if (params.compression_mode == 0u) {
        // Pack time (16 bits)
        let time_u16 = pack_to_u16(vertex.time, params.time_min, params.time_max);
        
        // Pack value (16 bits)
        let value_u16 = pack_to_u16(vertex.value, params.value_min, params.value_max);
        
        // Combine into 32-bit word
        let time_value = (time_u16 << 16u) | value_u16;
        
        // Calculate metadata
        var metadata = 0u;
        
        // Color index based on value (8 bits)
        let normalized_value = (vertex.value - params.value_min) / (params.value_max - params.value_min);
        let color_index = u32(clamp(normalized_value * 255.0, 0.0, 255.0));
        metadata |= color_index << 24u;
        
        // Flags (8 bits)
        var flags = 0u;
        if (vertex.value > params.value_max * 0.9) { flags |= 1u; } // High value flag
        if (vertex.value < params.value_min * 0.1) { flags |= 2u; } // Low value flag
        metadata |= flags << 16u;
        
        // Store compressed vertex
        var compressed: CompressedVertex8;
        compressed.time_value = time_value;
        compressed.metadata = metadata;
        output_vertices[thread_idx] = compressed;
    }
    
    // Update workgroup statistics
    atomicMin(&min_time, bitcast<u32>(vertex.time));
    atomicMax(&max_time, bitcast<u32>(vertex.time));
    atomicMin(&min_value, bitcast<u32>(vertex.value));
    atomicMax(&max_value, bitcast<u32>(vertex.value));
}

// Ultra compression (4 bytes per vertex)
@compute @workgroup_size(256)
fn ultra_compress_vertices(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    
    if (thread_idx >= params.vertex_count) {
        return;
    }
    
    let vertex = input_vertices[thread_idx];
    
    // Pack time (12 bits)
    let time_u12 = pack_to_u12(vertex.time, params.time_min, params.time_max);
    
    // Pack value (12 bits)
    let value_u12 = pack_to_u12(vertex.value, params.value_min, params.value_max);
    
    // Flags (8 bits)
    var flags = 0u;
    let normalized_value = (vertex.value - params.value_min) / (params.value_max - params.value_min);
    
    // Quantize slope for trend detection
    if (thread_idx > 0u) {
        let prev_vertex = input_vertices[thread_idx - 1u];
        let slope = (vertex.value - prev_vertex.value) / (vertex.time - prev_vertex.time);
        
        if (slope > 0.1) { flags |= 1u; }      // Rising
        else if (slope < -0.1) { flags |= 2u; } // Falling
        else { flags |= 4u; }                    // Flat
    }
    
    // Quality hint
    if (normalized_value > 0.9 || normalized_value < 0.1) {
        flags |= 8u; // Extreme value
    }
    
    // Pack everything into 32 bits
    let packed = (time_u12 << 20u) | (value_u12 << 8u) | flags;
    
    // Store as CompressedVertex8 with packed data in first field
    var compressed: CompressedVertex8;
    compressed.time_value = packed;
    compressed.metadata = 0u;
    output_vertices[thread_idx] = compressed;
}

// Adaptive compression based on data characteristics
@compute @workgroup_size(256)
fn adaptive_compress(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    let local_idx = local_id.x;
    
    // Shared memory for local data analysis
    var<workgroup> local_variance: f32;
    var<workgroup> compression_quality: array<f32, 256>;
    
    if (thread_idx >= params.vertex_count) {
        return;
    }
    
    let vertex = input_vertices[thread_idx];
    
    // Analyze local data characteristics
    var quality_score = 1.0;
    
    // Check data density
    if (thread_idx > 0u && thread_idx < params.vertex_count - 1u) {
        let prev = input_vertices[thread_idx - 1u];
        let next = input_vertices[thread_idx + 1u];
        
        let time_density = 2.0 / ((next.time - prev.time) + 0.001);
        let value_change = abs(next.value - prev.value) / (params.value_max - params.value_min);
        
        quality_score = clamp(time_density * value_change, 0.0, 1.0);
    }
    
    compression_quality[local_idx] = quality_score;
    workgroupBarrier();
    
    // Decide compression level based on quality requirements
    let needs_high_precision = quality_score > 0.7;
    
    if (needs_high_precision) {
        // Use 8-byte compression for high precision areas
        let time_u16 = pack_to_u16(vertex.time, params.time_min, params.time_max);
        let value_u16 = pack_to_u16(vertex.value, params.value_min, params.value_max);
        
        var compressed: CompressedVertex8;
        compressed.time_value = (time_u16 << 16u) | value_u16;
        compressed.metadata = 0x80000000u; // High precision flag
        output_vertices[thread_idx] = compressed;
    } else {
        // Use 4-byte ultra compression for low precision areas
        let time_u12 = pack_to_u12(vertex.time, params.time_min, params.time_max);
        let value_u12 = pack_to_u12(vertex.value, params.value_min, params.value_max);
        
        var compressed: CompressedVertex8;
        compressed.time_value = (time_u12 << 20u) | (value_u12 << 8u);
        compressed.metadata = 0u; // Low precision flag
        output_vertices[thread_idx] = compressed;
    }
}

// Specialized compression for time-series data with delta encoding
@compute @workgroup_size(256)
fn delta_compress(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>
) {
    let thread_idx = global_id.x;
    
    if (thread_idx >= params.vertex_count) {
        return;
    }
    
    let vertex = input_vertices[thread_idx];
    
    if (thread_idx == 0u) {
        // First vertex - store absolute values
        let time_u16 = pack_to_u16(vertex.time, params.time_min, params.time_max);
        let value_u16 = pack_to_u16(vertex.value, params.value_min, params.value_max);
        
        var compressed: CompressedVertex8;
        compressed.time_value = (time_u16 << 16u) | value_u16;
        compressed.metadata = 0x40000000u; // Absolute value flag
        output_vertices[thread_idx] = compressed;
    } else {
        // Subsequent vertices - store deltas
        let prev_vertex = input_vertices[thread_idx - 1u];
        
        let time_delta = vertex.time - prev_vertex.time;
        let value_delta = vertex.value - prev_vertex.value;
        
        // Use smaller ranges for deltas
        let delta_time_range = (params.time_max - params.time_min) * 0.1;
        let delta_value_range = (params.value_max - params.value_min) * 0.2;
        
        let time_delta_u16 = pack_to_u16(
            time_delta + delta_time_range * 0.5,
            0.0,
            delta_time_range
        );
        
        let value_delta_u16 = pack_to_u16(
            value_delta + delta_value_range * 0.5,
            -delta_value_range * 0.5,
            delta_value_range * 0.5
        );
        
        var compressed: CompressedVertex8;
        compressed.time_value = (time_delta_u16 << 16u) | value_delta_u16;
        compressed.metadata = 0x20000000u; // Delta value flag
        output_vertices[thread_idx] = compressed;
    }
}