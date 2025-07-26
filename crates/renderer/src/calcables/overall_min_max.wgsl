// Compute shader to find overall min/max from per-metric min/max values

// Input buffer contains pairs of floats: [min0, max0, min1, max1, ...]
@group(0) @binding(0) var<storage, read> input_data: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_min_max: vec2<f32>;
@group(0) @binding(2) var<uniform> num_metrics: u32;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if (global_id.x != 0u) {
        return;
    }
    
    var overall_min = 999999999.0;
    var overall_max = -999999999.0;
    
    // Iterate through all metrics' min/max values
    // Each metric has 2 floats: min at index i*2, max at index i*2+1
    for (var i = 0u; i < num_metrics; i = i + 1u) {
        let min_idx = i * 2u;
        let max_idx = i * 2u + 1u;
        
        let metric_min = input_data[min_idx];
        let metric_max = input_data[max_idx];
        
        overall_min = min(overall_min, metric_min);
        overall_max = max(overall_max, metric_max);
    }
    
    output_min_max = vec2<f32>(overall_min, overall_max);
}