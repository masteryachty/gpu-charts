// Compute shader for calculating Exponential Moving Averages (EMA)
// Processes raw trade data to calculate EMAs with different periods

struct EmaParams {
    element_count: u32,        // Number of input price points
    period: u32,                // EMA period (9, 20, 50, 100, or 200)
    alpha_numerator: u32,       // Numerator for alpha calculation (2)
    alpha_denominator: u32,     // Denominator for alpha calculation (period + 1)
}

@group(0) @binding(0) var<storage, read> price_data: array<f32>;
@group(0) @binding(1) var<storage, read_write> ema_output: array<f32>;
@group(0) @binding(2) var<uniform> params: EmaParams;

// Sequential EMA calculation - must be done in order due to dependency on previous values
@compute @workgroup_size(1, 1, 1)
fn compute_ema(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // EMA calculation must be sequential, so we only use one workgroup
    if (global_id.x != 0u) {
        return;
    }
    
    let count = params.element_count;
    let period = params.period;
    
    if (count == 0u || period == 0u) {
        return;
    }
    
    // Calculate alpha = 2 / (period + 1)
    let alpha = f32(params.alpha_numerator) / f32(params.alpha_denominator);
    let one_minus_alpha = 1.0 - alpha;
    
    // Initialize EMA with simple average of first 'period' values
    var initial_sum: f32 = 0.0;
    let initial_count = min(period, count);
    
    for (var i = 0u; i < initial_count; i++) {
        initial_sum += price_data[i];
    }
    
    var ema: f32 = initial_sum / f32(initial_count);
    
    // Store initial EMA values (all same until we have enough data)
    for (var i = 0u; i < min(period, count); i++) {
        ema_output[i] = ema;
    }
    
    // Calculate EMA for remaining values
    // EMA = alpha * price + (1 - alpha) * previous_EMA
    for (var i = period; i < count; i++) {
        let price = price_data[i];
        ema = alpha * price + one_minus_alpha * ema;
        ema_output[i] = ema;
    }
}

// Parallel EMA initialization for multiple periods
// Each workgroup handles one EMA period
@compute @workgroup_size(1, 1, 1)
fn compute_ema_multi(@builtin(workgroup_id) wg_id: vec3<u32>) {
    let ema_index = wg_id.x; // Which EMA period (0-4 for periods 9, 20, 50, 100, 200)
    
    // Map index to actual period
    var period: u32;
    if (ema_index == 0u) {
        period = 9u;
    } else if (ema_index == 1u) {
        period = 20u;
    } else if (ema_index == 2u) {
        period = 50u;
    } else if (ema_index == 3u) {
        period = 100u;
    } else if (ema_index == 4u) {
        period = 200u;
    } else {
        return; // Invalid index
    }
    
    let count = params.element_count;
    if (count == 0u) {
        return;
    }
    
    // Calculate alpha for this period
    let alpha = 2.0 / (f32(period) + 1.0);
    let one_minus_alpha = 1.0 - alpha;
    
    // Calculate initial SMA
    var initial_sum: f32 = 0.0;
    let initial_count = min(period, count);
    
    for (var i = 0u; i < initial_count; i++) {
        initial_sum += price_data[i];
    }
    
    var ema: f32 = initial_sum / f32(initial_count);
    
    // Calculate offset in output buffer for this EMA
    let buffer_offset = ema_index * count;
    
    // Store initial values
    for (var i = 0u; i < min(period, count); i++) {
        ema_output[buffer_offset + i] = ema;
    }
    
    // Calculate EMA for remaining values
    for (var i = period; i < count; i++) {
        let price = price_data[i];
        ema = alpha * price + one_minus_alpha * ema;
        ema_output[buffer_offset + i] = ema;
    }
}