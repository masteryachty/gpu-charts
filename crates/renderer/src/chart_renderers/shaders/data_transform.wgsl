// Compute shader for transforming time/price data to vertices

struct DataRange {
    time_min: f32,
    time_max: f32,
    price_min: f32,
    price_max: f32,
}

@group(0) @binding(0)
var<uniform> data_range: DataRange;

@group(0) @binding(1)
var<storage, read> time_data: array<f32>;

@group(0) @binding(2)
var<storage, read> price_data: array<f32>;

@group(0) @binding(3)
var<storage, read_write> vertices: array<vec2<f32>>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    let data_count = arrayLength(&time_data);
    
    if (index >= data_count) {
        return;
    }
    
    // Read time and price values
    let time = time_data[index];
    let price = price_data[index];
    
    // Normalize to clip space [-1, 1]
    let x = (time - data_range.time_min) / (data_range.time_max - data_range.time_min) * 2.0 - 1.0;
    let y = (price - data_range.price_min) / (data_range.price_max - data_range.price_min) * 2.0 - 1.0;
    
    // Store vertex
    vertices[index] = vec2<f32>(x, y);
}