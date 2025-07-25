// Compute shader for calculating mid price from bid and ask data

struct ComputeParams {
    element_count: u32,
    _padding: array<u32, 3>, // Align to 16 bytes
}

@group(0) @binding(0) var<storage, read> bid_data: array<f32>;
@group(0) @binding(1) var<storage, read> ask_data: array<f32>;
@group(0) @binding(2) var<storage, read_write> mid_data: array<f32>;
@group(0) @binding(3) var<uniform> params: ComputeParams;

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
    // Handle edge cases where bid or ask might be 0 or invalid
    var mid: f32;
    
    if (bid > 0.0 && ask > 0.0) {
        mid = (bid + ask) / 2.0;
    } else if (bid > 0.0) {
        // Only bid is valid
        mid = bid;
    } else if (ask > 0.0) {
        // Only ask is valid
        mid = ask;
    } else {
        // Both invalid, use 0
        mid = 0.0;
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