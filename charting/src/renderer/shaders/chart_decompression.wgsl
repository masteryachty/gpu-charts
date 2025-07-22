// Chart vertex decompression shader
// Used as part of the vertex shader to decompress packed vertices

struct CompressionParams {
    time_min: u32,
    time_max: u32,
    value_min: f32,
    value_max: f32,
}

// Decompress time from packed format
fn decompress_time(packed: u32, params: CompressionParams) -> f32 {
    let time_u16 = (packed >> 16u) & 0xFFFFu;
    let normalized = f32(time_u16) / 65535.0;
    let time_range = f32(params.time_max - params.time_min);
    return f32(params.time_min) + normalized * time_range;
}

// Decompress value from packed format
fn decompress_value(packed: u32, params: CompressionParams) -> f32 {
    let value_u16 = packed & 0xFFFFu;
    let normalized = f32(value_u16) / 65535.0;
    let value_range = params.value_max - params.value_min;
    return params.value_min + normalized * value_range;
}