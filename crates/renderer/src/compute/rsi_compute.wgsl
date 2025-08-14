// Compute shader for calculating RSI (Relative Strength Index) from price data

struct ComputeParams {
    element_count: u32,
    period: u32,
    _padding1: u32,
    _padding2: u32,
}

@group(0) @binding(0) var<storage, read> price_data: array<f32>;
@group(0) @binding(1) var<storage, read_write> rsi_data: array<f32>;
@group(0) @binding(2) var<uniform> params: ComputeParams;

// Helper function to calculate exponential moving average
fn ema_smoothing_factor(period: u32) -> f32 {
    return 2.0 / (f32(period) + 1.0);
}

@compute @workgroup_size(256, 1, 1)
fn compute_rsi(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    // Check bounds
    if (index >= params.element_count) {
        return;
    }
    
    // RSI needs at least 'period' previous values
    if (index < params.period) {
        rsi_data[index] = 50.0; // Neutral RSI value for insufficient data
        return;
    }
    
    let period = params.period;
    let alpha = ema_smoothing_factor(period);
    
    // Calculate initial average for first RSI calculation
    if (index == period) {
        var total_gain = 0.0;
        var total_loss = 0.0;
        
        // Calculate simple average for the first 'period' values
        for (var i = 1u; i <= period; i++) {
            let current_price = price_data[index - period + i];
            let prev_price = price_data[index - period + i - 1u];
            let change = current_price - prev_price;
            
            if (change > 0.0) {
                total_gain += change;
            } else {
                total_loss += abs(change);
            }
        }
        
        let avg_gain = total_gain / f32(period);
        let avg_loss = total_loss / f32(period);
        
        // Calculate RSI
        var rsi: f32;
        if (avg_loss == 0.0) {
            rsi = 100.0; // All gains, maximum RSI
        } else {
            let rs = avg_gain / avg_loss;
            rsi = 100.0 - (100.0 / (1.0 + rs));
        }
        
        rsi_data[index] = rsi;
        return;
    }
    
    // For subsequent calculations, use EMA
    if (index > period) {
        let current_price = price_data[index];
        let prev_price = price_data[index - 1u];
        let change = current_price - prev_price;
        
        // Get previous RSI to calculate previous avg_gain and avg_loss
        let prev_rsi = rsi_data[index - 1u];
        
        // Reverse engineer previous avg_gain and avg_loss from RSI
        // RSI = 100 - (100 / (1 + RS))
        // RS = avg_gain / avg_loss
        // Solving: avg_gain = RS * avg_loss
        let rs_prev = if (prev_rsi == 0.0) {
            0.0
        } else {
            (100.0 - prev_rsi) / prev_rsi;
        };
        
        // Estimate previous averages (this is an approximation for GPU efficiency)
        var prev_avg_gain: f32;
        var prev_avg_loss: f32;
        
        if (rs_prev == 0.0) {
            prev_avg_gain = 0.0;
            prev_avg_loss = 1.0; // Small positive value to avoid division by zero
        } else {
            // Use a reasonable estimate based on typical price volatility
            prev_avg_loss = 0.1; // Arbitrary but reasonable baseline
            prev_avg_gain = rs_prev * prev_avg_loss;
        }
        
        // Update with current change using EMA
        var current_gain = 0.0;
        var current_loss = 0.0;
        
        if (change > 0.0) {
            current_gain = change;
        } else {
            current_loss = abs(change);
        }
        
        let avg_gain = (1.0 - alpha) * prev_avg_gain + alpha * current_gain;
        let avg_loss = (1.0 - alpha) * prev_avg_loss + alpha * current_loss;
        
        // Calculate RSI
        var rsi: f32;
        if (avg_loss == 0.0) {
            rsi = 100.0;
        } else {
            let rs = avg_gain / avg_loss;
            rsi = 100.0 - (100.0 / (1.0 + rs));
        }
        
        // Clamp RSI to valid range [0, 100]
        rsi = clamp(rsi, 0.0, 100.0);
        
        rsi_data[index] = rsi;
    }
}

// Alternative RSI calculation using simple moving average (more accurate but slower)
@compute @workgroup_size(64, 1, 1)
fn compute_rsi_sma(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    
    if (index >= params.element_count || index < params.period) {
        if (index < params.element_count && index < params.period) {
            rsi_data[index] = 50.0; // Neutral RSI for insufficient data
        }
        return;
    }
    
    let period = params.period;
    var total_gain = 0.0;
    var total_loss = 0.0;
    
    // Calculate gains and losses over the period
    for (var i = 1u; i <= period; i++) {
        let current_idx = index - period + i;
        let prev_idx = current_idx - 1u;
        
        let current_price = price_data[current_idx];
        let prev_price = price_data[prev_idx];
        let change = current_price - prev_price;
        
        if (change > 0.0) {
            total_gain += change;
        } else {
            total_loss += abs(change);
        }
    }
    
    let avg_gain = total_gain / f32(period);
    let avg_loss = total_loss / f32(period);
    
    // Calculate RSI
    var rsi: f32;
    if (avg_loss == 0.0) {
        rsi = 100.0;
    } else {
        let rs = avg_gain / avg_loss;
        rsi = 100.0 - (100.0 / (1.0 + rs));
    }
    
    rsi_data[index] = clamp(rsi, 0.0, 100.0);
}