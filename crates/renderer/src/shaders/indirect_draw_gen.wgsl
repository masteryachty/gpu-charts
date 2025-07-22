// GPU-based indirect draw call generation
// This shader analyzes data and generates optimal draw calls without CPU intervention

struct DrawGenParams {
    total_vertices: u32,
    viewport_start: f32,
    viewport_end: f32,
    batch_size: u32,
    max_draw_calls: u32,
    render_mode: u32,
    enable_culling: u32,
    _padding: u32,
}

struct DataInfo {
    offset: u32,
    stride: u32,
    count: u32,
    buffer_index: u32,
    column_type: u32,
    _padding: array<u32, 3>,
}

struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

// Input data information
@group(0) @binding(0) var<storage, read> data_infos: array<DataInfo>;

// Output indirect commands
@group(0) @binding(1) var<storage, read_write> indirect_commands: array<DrawIndirectArgs>;

// Draw count
@group(0) @binding(2) var<storage, read_write> draw_count: atomic<u32>;

// Parameters
@group(0) @binding(3) var<uniform> params: DrawGenParams;

// Shared memory for workgroup coordination
var<workgroup> workgroup_vertex_count: atomic<u32>;
var<workgroup> workgroup_draw_index: atomic<u32>;

@compute @workgroup_size(64)
fn generate_draw_calls(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let thread_id = global_id.x;
    
    // Initialize workgroup shared memory
    if (thread_id % 64u == 0u) {
        atomicStore(&workgroup_vertex_count, 0u);
        atomicStore(&workgroup_draw_index, 0u);
    }
    workgroupBarrier();
    
    // Check if this thread should process vertices
    if (thread_id >= params.total_vertices) {
        return;
    }
    
    // Determine if this vertex should be included based on viewport
    // In a real implementation, we'd read actual data here
    let vertex_in_viewport = should_include_vertex(thread_id);
    
    if (vertex_in_viewport) {
        // Increment vertex count for this workgroup
        let local_vertex_index = atomicAdd(&workgroup_vertex_count, 1u);
        
        // Check if we've filled a batch
        if (local_vertex_index > 0u && local_vertex_index % params.batch_size == 0u) {
            // Create a new draw call
            create_draw_call();
        }
    }
    
    // Synchronize before finalizing
    workgroupBarrier();
    
    // Last thread in workgroup creates final draw call if needed
    if (thread_id % 64u == 63u || thread_id == params.total_vertices - 1u) {
        let remaining = atomicLoad(&workgroup_vertex_count);
        if (remaining > 0u) {
            create_final_draw_call(remaining);
        }
    }
}

fn should_include_vertex(vertex_index: u32) -> bool {
    // Simple viewport culling
    // In practice, this would read actual vertex data
    
    // Simulate reading vertex timestamp
    let normalized_position = f32(vertex_index) / f32(params.total_vertices);
    let vertex_time = mix(0.0, 1000000.0, normalized_position);
    
    // Check if within viewport
    return vertex_time >= params.viewport_start && vertex_time <= params.viewport_end;
}

fn create_draw_call() {
    // Get next draw call index
    let draw_index = atomicAdd(&draw_count, 1u);
    
    // Safety check
    if (draw_index >= params.max_draw_calls) {
        atomicSub(&draw_count, 1u);
        return;
    }
    
    // Create draw command
    var command: DrawIndirectArgs;
    command.vertex_count = params.batch_size;
    command.instance_count = 1u;
    command.first_vertex = draw_index * params.batch_size;
    command.first_instance = 0u;
    
    // Store command
    indirect_commands[draw_index] = command;
}

fn create_final_draw_call(vertex_count: u32) {
    let draw_index = atomicAdd(&draw_count, 1u);
    
    if (draw_index >= params.max_draw_calls) {
        atomicSub(&draw_count, 1u);
        return;
    }
    
    var command: DrawIndirectArgs;
    command.vertex_count = vertex_count;
    command.instance_count = 1u;
    command.first_vertex = draw_index * params.batch_size;
    command.first_instance = 0u;
    
    indirect_commands[draw_index] = command;
}

// Advanced draw call generation with multiple strategies
@compute @workgroup_size(256)
fn generate_adaptive_draws(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let thread_id = global_id.x;
    let local_thread = local_id.x;
    
    // Shared memory for data analysis
    var<workgroup> data_density: array<f32, 256>;
    var<workgroup> importance_scores: array<f32, 256>;
    
    // Analyze data chunk
    if (thread_id < params.total_vertices) {
        let density = analyze_local_density(thread_id);
        let importance = calculate_importance(thread_id, density);
        
        data_density[local_thread] = density;
        importance_scores[local_thread] = importance;
    }
    workgroupBarrier();
    
    // Collaborative decision making within workgroup
    if (local_thread == 0u) {
        // Analyze workgroup data
        var total_importance = 0.0;
        var high_importance_count = 0u;
        
        for (var i = 0u; i < 256u; i = i + 1u) {
            total_importance += importance_scores[i];
            if (importance_scores[i] > 0.7) {
                high_importance_count += 1u;
            }
        }
        
        // Decide draw strategy based on analysis
        let avg_importance = total_importance / 256.0;
        
        if (avg_importance > 0.8) {
            // High detail area - create multiple small draw calls
            create_detailed_draws(workgroup_id.x, high_importance_count);
        } else if (avg_importance > 0.4) {
            // Medium detail - standard draw call
            create_standard_draw(workgroup_id.x);
        } else {
            // Low detail - can be batched with others
            register_for_batching(workgroup_id.x);
        }
    }
}

fn analyze_local_density(vertex_index: u32) -> f32 {
    // Analyze how densely packed data points are
    // This helps determine rendering strategy
    
    let position_in_dataset = f32(vertex_index) / f32(params.total_vertices);
    
    // Simulate density calculation
    // High density at certain regions (e.g., market open/close)
    let market_open = 0.3;
    let market_close = 0.7;
    
    let open_distance = abs(position_in_dataset - market_open);
    let close_distance = abs(position_in_dataset - market_close);
    
    let density = 1.0 - min(open_distance, close_distance) * 3.0;
    return clamp(density, 0.0, 1.0);
}

fn calculate_importance(vertex_index: u32, density: f32) -> f32 {
    // Calculate rendering importance based on multiple factors
    
    // Factor 1: Data density (dense areas are important)
    let density_factor = density * 0.4;
    
    // Factor 2: Position in viewport (center is more important)
    let viewport_position = f32(vertex_index) / f32(params.total_vertices);
    let center_distance = abs(viewport_position - 0.5) * 2.0;
    let position_factor = (1.0 - center_distance) * 0.3;
    
    // Factor 3: Volatility (simulated - would read actual data)
    let volatility = sin(f32(vertex_index) * 0.1) * 0.5 + 0.5;
    let volatility_factor = volatility * 0.3;
    
    return density_factor + position_factor + volatility_factor;
}

fn create_detailed_draws(workgroup_index: u32, vertex_count: u32) {
    // Create multiple small draw calls for high detail rendering
    let draws_needed = (vertex_count + 15u) / 16u; // 16 vertices per draw
    
    for (var i = 0u; i < draws_needed; i = i + 1u) {
        let draw_index = atomicAdd(&draw_count, 1u);
        if (draw_index >= params.max_draw_calls) {
            break;
        }
        
        var command: DrawIndirectArgs;
        command.vertex_count = min(16u, vertex_count - i * 16u);
        command.instance_count = 1u;
        command.first_vertex = workgroup_index * 256u + i * 16u;
        command.first_instance = 0u;
        
        indirect_commands[draw_index] = command;
    }
}

fn create_standard_draw(workgroup_index: u32) {
    let draw_index = atomicAdd(&draw_count, 1u);
    if (draw_index >= params.max_draw_calls) {
        return;
    }
    
    var command: DrawIndirectArgs;
    command.vertex_count = 256u; // Full workgroup
    command.instance_count = 1u;
    command.first_vertex = workgroup_index * 256u;
    command.first_instance = 0u;
    
    indirect_commands[draw_index] = command;
}

fn register_for_batching(workgroup_index: u32) {
    // In a real implementation, this would add to a batch list
    // For now, create a standard draw with instance flag
    let draw_index = atomicAdd(&draw_count, 1u);
    if (draw_index >= params.max_draw_calls) {
        return;
    }
    
    var command: DrawIndirectArgs;
    command.vertex_count = 256u;
    command.instance_count = 1u;
    command.first_vertex = workgroup_index * 256u;
    command.first_instance = 1u; // Flag for low priority
    
    indirect_commands[draw_index] = command;
}