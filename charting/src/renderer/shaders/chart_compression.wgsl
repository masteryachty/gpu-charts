// Chart vertex compression compute shader
// Compresses vertex data from separate x/y buffers into packed format

struct CompressionParams {
    time_min: u32,
    time_max: u32,
    value_min: f32,
    value_max: f32,
}

struct CompressedVertex {
    time_value: u32,  // time (16 bits) | value (16 bits)
    metadata: u32,    // series_index (8) | flags (8) | reserved (16)
}

@group(0) @binding(0) var<storage, read> x_data: array<u32>;
@group(0) @binding(1) var<storage, read> y_data: array<f32>;
@group(0) @binding(2) var<storage, write> compressed: array<CompressedVertex>;
@group(0) @binding(3) var<uniform> params: CompressionParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let data_len = arrayLength(&x_data);
    
    if (index >= data_len) {
        return;
    }
    
    let time = x_data[index];
    let value = y_data[index];
    
    // Normalize time to 0-1 range
    let time_range = f32(params.time_max - params.time_min);
    let normalized_time = f32(time - params.time_min) / time_range;
    let time_u16 = u32(clamp(normalized_time, 0.0, 1.0) * 65535.0);
    
    // Normalize value to 0-1 range
    let value_range = params.value_max - params.value_min;
    let normalized_value = (value - params.value_min) / value_range;
    let value_u16 = u32(clamp(normalized_value, 0.0, 1.0) * 65535.0);
    
    // Pack into compressed format
    compressed[index].time_value = (time_u16 << 16u) | value_u16;
    compressed[index].metadata = 0u;
}