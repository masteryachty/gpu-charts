// min_max_first.wgsl

struct Params {
    start_index: u32,
    end_index: u32,     // exclusive
    chunk_size: u32,     // how many total elements each workgroup processes
};

@group(0) @binding(0)
var<storage, read>  input_data : array<f32>;
@group(0) @binding(1)
var<storage, read_write> partial_out : array<f32>;
@group(0) @binding(2)
var<uniform> params : Params;

var<workgroup> local_min: array<f32, 256>;
var<workgroup> local_max: array<f32, 256>;

// We define each group to have 256 threads:
@compute @workgroup_size(256)
fn main(@builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>) {
    let lid = local_id.x;
    let wg_id = workgroup_id.x;

    let start_index = params.start_index;
    let end_index = params.end_index;
    let chunk_size = params.chunk_size;  // e.g. 2048

    // Where this workgroup starts in the global array
    let group_start = start_index + wg_id * chunk_size;
    let group_end = min(group_start + chunk_size, end_index);
    
    // Also clamp to actual data size
    let data_size = arrayLength(&input_data);
    let safe_group_end = min(group_end, data_size);

    // We'll accumulate a local min/max in registers for each thread
    var thread_min = f32(3.402823466e+38);   // +Infinity
    var thread_max = f32(-3.402823466e+38);   // -Infinity

    // Each thread processes multiple elements
    // e.g., chunk_size = 256 * 8 => each thread does 8 elements
    let THREAD_MULT = chunk_size / 256u; // e.g. 2048/256 = 8

    // Base index for this thread
    let base = group_start + lid * THREAD_MULT;

    var data_count = 0u;
    
    // Process all elements for this thread
    for (var i = 0u; i < THREAD_MULT; i += 1u) {
        let idx = base + i;
        if (idx < safe_group_end) {
            let value = input_data[idx];
            
            // Accept all finite values that are not zero or our sentinel values
            // In WGSL, we check if a value is finite by comparing it to itself (NaN != NaN)
            // and checking if it's not infinity
            // Skip zero values and our sentinel value (1.0) which indicate missing data
            // Only accept values > 1.0 for proper price data
            if (value == value && value > 1.0 && value < 3.402823466e+38) {
                thread_min = min(thread_min, value);
                thread_max = max(thread_max, value);
                data_count += 1u;
            }
        }
    }
    
    // If no valid data found, keep infinity values
    // This ensures they don't interfere with valid data during reduction
    if data_count == 0u {
        // Keep thread_min as +infinity and thread_max as -infinity
        // These will be ignored during the min/max reduction
    }

    // Store these per-thread partial results into shared memory
    local_min[lid] = thread_min;
    local_max[lid] = thread_max;

    workgroupBarrier();

    // Now do the standard tree reduction in shared memory
    var stride = 256u / 2u;
    while stride > 0u {
        if lid < stride {
            local_min[lid] = min(local_min[lid], local_min[lid + stride]);
            local_max[lid] = max(local_max[lid], local_max[lid + stride]);
        }
        workgroupBarrier();
        stride = stride / 2u;
    }

    // After reduction, local_min[0] and local_max[0] hold the group's min/max
    if lid == 0u {
        // Write the workgroup's min/max to the output buffer
        partial_out[wg_id * 2u] = local_min[0];
        partial_out[wg_id * 2u + 1u] = local_max[0];
    }
}
