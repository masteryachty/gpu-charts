// reduce_subsequent_pass.wgsl

// We assume the uniform block passes in:
//   - element_count: how many [min,max] pairs are in the input
//   - chunk_size:    how many pairs each group should process
struct Params {
    element_count: u32,
    chunk_size: u32,
};

@group(0) @binding(0)
var<storage, read>  partial_in  : array<f32>;
@group(0) @binding(1)
var<storage, read_write> partial_out : array<f32>;
@group(0) @binding(2)
var<uniform> params : Params;

var<workgroup> local_min: array<f32, 256>;
var<workgroup> local_max: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>) {
    let lid = local_id.x;
    let wg_id = workgroup_id.x;
    let chunk_size = params.chunk_size;

    // Each pair is 2 floats: partial_in[2*i] = min, partial_in[2*i+1] = max
    let start_pair = wg_id * chunk_size;
    let end_pair = min((wg_id + 1u) * chunk_size, params.element_count);
    let count = end_pair - start_pair;

    // Initialize with +inf/-inf
    var my_min = f32(3.402823466e+38);
    var my_max = f32(-3.402823466e+38);

    // We'll do the same approach: each thread picks one pair if in range
    if lid < count {
        let pair_index = start_pair + lid;
        let v_min = partial_in[pair_index * 2u];
        let v_max = partial_in[pair_index * 2u + 1u];
        my_min = v_min;
        my_max = v_max;
    }

    // Put them in shared memory
    local_min[lid] = my_min;
    local_max[lid] = my_max;
    workgroupBarrier();

    // Parallel reduction
    var stride = 256u / 2u;
    while stride > 0u {
        if lid < stride && (lid + stride) < 256u {
            local_min[lid] = min(local_min[lid], local_min[lid + stride]);
            local_max[lid] = max(local_max[lid], local_max[lid + stride]);
        }
        workgroupBarrier();
        stride = stride / 2u;
    }

    // Output result
    if lid == 0u {
        partial_out[wg_id * 2u] = local_min[0];
        partial_out[wg_id * 2u + 1u] = local_max[0];
    }
}
