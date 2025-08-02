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
    
    var overall_min = 3.402823466e+38;  // +Infinity
    var overall_max = -3.402823466e+38; // -Infinity
    
    let data_length = arrayLength(&input_data);
    if (data_length == 0u || num_metrics == 0u) {
        output_min_max = vec2<f32>(0.0, 1.0);
        return;
    }
    
    // Iterate through all metrics' min/max values
    // Each metric has 2 floats: min at index i*2, max at index i*2+1
    for (var i = 0u; i < num_metrics; i = i + 1u) {
        let min_idx = i * 2u;
        let max_idx = i * 2u + 1u;
        
        let metric_min = input_data[min_idx];
        let metric_max = input_data[max_idx];
        
        // Handle min and max separately, skipping infinity values
        if (metric_min < 3.402823466e+38 && metric_min > -3.402823466e+38) {
            overall_min = min(overall_min, metric_min);
        }
        
        if (metric_max > -3.402823466e+38 && metric_max < 3.402823466e+38) {
            overall_max = max(overall_max, metric_max);
        }
    }
    
    // If no valid data was found, return default values
    if (overall_min >= 3.402823466e+38 || overall_max <= -3.402823466e+38 || overall_min > overall_max) {
        output_min_max = vec2<f32>(0.0, 1.0);
    } else {
        output_min_max = vec2<f32>(overall_min, overall_max);
    }
}