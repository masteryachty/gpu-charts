// Compute shader for calculating mid price from bid and ask data

struct ComputeParams {
    element_count: u32,
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
}

@group(0) @binding(0) var<storage, read> bid_data: array<f32>;
@group(0) @binding(1) var<storage, read> ask_data: array<f32>;
@group(0) @binding(2) var<storage, read_write> mid_data: array<f32>;
@group(0) @binding(3) var<uniform> params: ComputeParams;

// Helper function to find the nearest valid price looking backward
fn find_previous_valid(index: u32) -> f32 {
    var search_index = index;
    var max_search = min(index, 1000u); // Search up to 1000 points back
    
    for (var i = 0u; i < max_search; i++) {
        if (search_index == 0u) {
            break;
        }
        search_index = search_index - 1u;
        
        let bid = bid_data[search_index];
        let ask = ask_data[search_index];
        
        if (bid > 0.0 && ask > 0.0) {
            return (bid + ask) / 2.0;
        } else if (bid > 0.0) {
            return bid;
        } else if (ask > 0.0) {
            return ask;
        }
    }
    
    return 0.0;
}

// Helper function to find the nearest valid price looking forward
fn find_next_valid(index: u32, max_index: u32) -> f32 {
    var search_index = index;
    var max_search = min(max_index - index, 1000u); // Search up to 1000 points forward
    
    for (var i = 0u; i < max_search; i++) {
        search_index = search_index + 1u;
        if (search_index >= max_index) {
            break;
        }
        
        let bid = bid_data[search_index];
        let ask = ask_data[search_index];
        
        if (bid > 0.0 && ask > 0.0) {
            return (bid + ask) / 2.0;
        } else if (bid > 0.0) {
            return bid;
        } else if (ask > 0.0) {
            return ask;
        }
    }
    
    return 0.0;
}

@compute @workgroup_size(256, 1, 1)
fn compute_mid_price(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    // Check bounds
    if (index >= params.element_count) {
        return;
    }
    
    // Read bid and ask values
    let bid = bid_data[index];
    let ask = ask_data[index];
    
    // Calculate mid price
    var mid: f32;
    
    if (bid > 0.0 && ask > 0.0) {
        // Both valid - compute average
        mid = (bid + ask) / 2.0;
    } else if (bid > 0.0) {
        // Only bid is valid - use bid
        mid = bid;
    } else if (ask > 0.0) {
        // Only ask is valid - use ask
        mid = ask;
    } else {
        // Both invalid - interpolate between nearest valid values
        let prev_valid = find_previous_valid(index);
        let next_valid = find_next_valid(index, params.element_count);
        
        if (prev_valid > 0.0 && next_valid > 0.0) {
            // Interpolate between previous and next valid values
            // Simple average for now (could do linear interpolation based on distance)
            mid = (prev_valid + next_valid) / 2.0;
        } else if (prev_valid > 0.0) {
            // Only previous is valid - carry it forward
            mid = prev_valid;
        } else if (next_valid > 0.0) {
            // Only next is valid - use it
            mid = next_valid;
        } else {
            // No valid data found anywhere - last resort
            // Use a small positive value to avoid exactly 0
            mid = 1.0; // Use 1.0 instead of 0 to keep the line visible
        }
    }
    
    // Write result
    mid_data[index] = mid;
}

// Additional compute functions for future use

@compute @workgroup_size(256, 1, 1)
fn compute_spread(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    if (index >= params.element_count) {
        return;
    }
    
    let bid = bid_data[index];
    let ask = ask_data[index];
    
    // Calculate spread (ask - bid)
    var spread: f32;
    
    if (bid > 0.0 && ask > 0.0) {
        spread = ask - bid;
    } else {
        spread = 0.0;
    }
    
    mid_data[index] = spread;
}

@compute @workgroup_size(256, 1, 1)
fn compute_spread_percentage(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    if (index >= params.element_count) {
        return;
    }
    
    let bid = bid_data[index];
    let ask = ask_data[index];
    
    // Calculate spread percentage: (ask - bid) / mid * 100
    var spread_pct: f32;
    
    if (bid > 0.0 && ask > 0.0) {
        let mid = (bid + ask) / 2.0;
        spread_pct = ((ask - bid) / mid) * 100.0;
    } else {
        spread_pct = 0.0;
    }
    
    mid_data[index] = spread_pct;
}