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

    // We'll accumulate a local min/max in registers for each thread
    var thread_min = f32(3.402823466e+38);   // +Infinity
    var thread_max = f32(-3.402823466e+38);   // -Infinity

    // Each thread processes multiple elements
    // e.g., chunk_size = 256 * 8 => each thread does 8 elements
    let THREAD_MULT = chunk_size / 256u; // e.g. 2048/256 = 8

    // Base index for this thread
    let base = group_start + lid * THREAD_MULT;

    for (var i = 0u; i < THREAD_MULT; i += 4u) {
        let idx = base + i;
        if idx + 3u < group_end {
            let values = array<f32, 4>(
                input_data[idx],
                input_data[idx + 1u],
                input_data[idx + 2u],
                input_data[idx + 3u],
            );
            thread_min = min(thread_min, min(min(values[0], values[1]), min(values[2], values[3])));
            thread_max = max(thread_max, max(max(values[0], values[1]), max(values[2], values[3])));
        }
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
        partial_out[wg_id * 2u] = local_min[0];
        partial_out[wg_id * 2u + 1u] = local_max[0];
    }
}
