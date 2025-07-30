# GPU Charts Architecture Deep Dive

## Table of Contents
1. [Executive Overview](#executive-overview)
2. [The State Management System](#the-state-management-system)
3. [The Rendering Pipeline](#the-rendering-pipeline)
4. [The GPU Compute System](#the-gpu-compute-system)
5. [Data Flow and Coordination](#data-flow-and-coordination)
6. [Performance Optimizations](#performance-optimizations)
7. [Real-World Example: User Zooms In](#real-world-example-user-zooms-in)

## Executive Overview

GPU Charts is a high-performance, WebGPU-based charting system designed for real-time financial data visualization. The architecture consists of five main components:

1. **Unified State System** - Centralized state management with change detection
2. **Simplified Render Loop** - 3-state render loop controller (Idle → Updating → Rendering)
3. **Multi-Renderer Pipeline** - Composable rendering system for complex visualizations
4. **GPU Compute Engine** - Pre-render computations on GPU (min/max, averages, etc.)
5. **Resource Pooling** - Efficient GPU resource management

## The State Management System

### Overview
The state management system is the brain of GPU Charts. It tracks all application state and efficiently detects what changed, triggering only the necessary updates.

### Unified State Structure
```rust
UnifiedState {
    generation: u64,  // Global version number
    sections: {
        Data → { symbol, time_range, data_version }
        View → { zoom_level, pan_offset, viewport }
        Config → { preset, quality, chart_type }
        GPU → { buffers_valid, pipelines_valid }
        UI → { visible_metrics, theme, layout }
    }
}
```

### How State Changes Work

1. **State Update Request**
   ```rust
   // React sends updated state
   chart.update_unified_state(store_state_json)
   ```

2. **Change Detection**
   ```rust
   // The system detects what changed
   StateDiff {
       changed_sections: [Data, View],
       section_changes: {
           Data: { symbol_changed: true, time_range_changed: false },
           View: { zoom_changed: true, pan_changed: false }
       }
   }
   ```

3. **Action Determination**
   ```rust
   StateChangeActions {
       needs_data_fetch: true,      // Symbol changed
       needs_preprocessing: true,   // New data needs bounds calculation
       needs_render: true,          // Visual update required
       needs_pipeline_rebuild: false // No config changes
   }
   ```

### Generation-Based Tracking
Each state section has its own generation number. This allows efficient change detection:
```rust
// Check if view changed since last render
if state.get_section(View).generation > last_render_generation {
    // Re-render needed
}
```

## The Rendering Pipeline

### Multi-Renderer Architecture
The rendering system uses a composable pipeline where multiple renderers work together:

```
MultiRenderer
├── CandlestickRenderer (priority: 50)
├── PlotRenderer (priority: 100)
├── XAxisRenderer (priority: 150)
└── YAxisRenderer (priority: 150)
```

### Render Execution Flow

1. **Clear Pass** - Background clear (once per frame)
2. **Compute Pass** - GPU computations (mid price, etc.)
3. **Bounds Calculation** - Find min/max Y values using GPU
4. **Render Passes** - Execute each renderer in priority order

### How Renderers Work

Each renderer implements the `MultiRenderable` trait:

```rust
trait MultiRenderable {
    fn render(&mut self, encoder, view, data_store, device, queue);
    fn priority(&self) -> u32;  // Lower = renders first
    fn has_compute(&self) -> bool;
    fn compute(&mut self, encoder, data_store, device, queue);
}
```

### Example: Candlestick Renderer

The candlestick renderer demonstrates the full pipeline:

1. **Compute Phase** (if needed)
   ```rust
   // Aggregate tick data into OHLC candles using GPU
   candle_aggregator.aggregate_candles(
       device, queue, encoder,
       time_buffer, price_buffer,
       tick_count, timeframe
   )
   ```

2. **Render Phase**
   ```rust
   // Render candle bodies (filled rectangles)
   render_pass.set_pipeline(&body_pipeline);
   render_pass.draw(0..num_candles * 6, 0..1);
   
   // Render wicks (lines)
   render_pass.set_pipeline(&wick_pipeline);
   render_pass.draw(0..num_candles * 4, 0..1);
   ```

### Pipeline Caching
Renderers cache their render pipelines and only rebuild when configuration changes:
```rust
if config_changed {
    self.pipeline = create_render_pipeline(device, new_config);
}
```

## The GPU Compute System

### Overview
The compute system runs GPU shaders for data processing before rendering. This includes:
- Min/max bounds calculation
- Moving averages
- Mid price from bid/ask
- Technical indicators

### Compute Engine Architecture

```rust
ComputeEngine {
    calculators: {
        mid_price: MidPriceCalculator,
        min_max: MinMaxCalculator,
        // ... other calculators
    },
    computed_metrics: HashMap<MetricRef, version>
}
```

### How Compute Works

1. **Dependency Resolution**
   ```rust
   // Mid price depends on bid and ask
   Metric {
       name: "mid_price",
       compute_type: Average,
       dependencies: [bid_metric, ask_metric]
   }
   ```

2. **Topological Sort**
   ```rust
   // Ensure dependencies computed first
   sorted_metrics = [bid, ask, mid_price, bollinger_bands]
   ```

3. **GPU Execution**
   ```rust
   // Compute shader for mid price
   @compute @workgroup_size(256)
   fn compute_mid_price(
       @builtin(global_invocation_id) id: vec3<u32>
   ) {
       let index = id.x;
       let bid = bid_buffer[index];
       let ask = ask_buffer[index];
       output_buffer[index] = (bid + ask) / 2.0;
   }
   ```

### Min/Max Calculation

The min/max calculation is critical for Y-axis scaling:

1. **Parallel Reduction**
   ```wgsl
   // First pass: Each thread finds local min/max
   let local_min = min(data[i*4], data[i*4+1], data[i*4+2], data[i*4+3]);
   
   // Second pass: Reduce across workgroups
   workgroup_mins[local_id] = local_min;
   workgroupBarrier();
   
   // Final: Single thread writes result
   if (local_id == 0) {
       output[workgroup_id] = workgroup_min;
   }
   ```

2. **Staging Buffer**
   ```rust
   // GPU → CPU readback for axis labels
   let staging_buffer = device.create_buffer({
       usage: COPY_DST | MAP_READ,
       size: 8, // min + max
   });
   ```

## Data Flow and Coordination

### The Simplified Render Loop

The render loop has only 3 states:

```
┌─────┐ trigger  ┌──────────┐ complete ┌───────────┐ complete ┌─────┐
│Idle │ -------> │Updating  │ -------> │Rendering  │ -------> │Idle │
└─────┘          └──────────┘          └───────────┘          └─────┘
```

### State Transitions

1. **Idle → Updating**
   - Triggered by: Data request, view change, config change
   - Actions: Fetch data, calculate bounds, prepare GPU resources

2. **Updating → Rendering**
   - Triggered by: Update completion
   - Actions: Execute render pipeline, present frame

3. **Rendering → Idle**
   - Triggered by: Render completion
   - Actions: Check for pending updates, clean up

### Update Types

```rust
enum UpdateType {
    Data,   // New data requested (fetch + preprocess + render)
    View,   // Pan/zoom changed (render only)
    Config, // Settings changed (rebuild pipeline + render)
}
```

### Batching and Priority

Updates are batched and prioritized:
```rust
// If multiple updates pending, take highest priority
Config > Data > View
```

### Frame Pacing

The system includes adaptive frame rate control:
```rust
FrameRateTarget {
    Smooth: 60 FPS,      // Trading/active interaction
    Balanced: 30 FPS,    // Normal viewing
    PowerSaver: 15 FPS,  // Background/idle
    Adaptive: Auto       // Adjust based on performance
}
```

## Performance Optimizations

### 1. Resource Pooling
```rust
ResourcePoolManager {
    buffer_pool: Vec<ReusableBuffer>,
    texture_pool: Vec<ReusableTexture>,
}

// Reuse buffers instead of creating new ones
let buffer = pool.get_buffer(size, usage);
```

### 2. Incremental Updates
- Only update changed state sections
- Skip rendering if nothing changed
- Reuse GPU resources across frames

### 3. GPU-Based Calculations
- All heavy computation on GPU
- Parallel processing of millions of data points
- Minimal CPU-GPU data transfer

### 4. Smart Change Detection
```rust
// Only fetch data if symbol or time range changed
if state_diff.section_changes.get(&Data).symbol_changed {
    fetch_new_data().await;
}
```

### 5. Pipeline State Caching
- Render pipelines compiled once
- Bind groups cached and reused
- Shader modules shared across renderers

## Real-World Example: User Zooms In

Let's trace what happens when a user scrolls to zoom:

### 1. **Browser Event**
```javascript
canvas.addEventListener('wheel', (e) => {
    chart.handle_mouse_wheel(e.deltaY, e.clientX, e.clientY);
});
```

### 2. **WASM Bridge**
```rust
pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, y: f64) {
    // Convert to internal event
    let event = WindowEvent::MouseWheel {
        delta: MouseScrollDelta::PixelDelta(x, delta_y),
        phase: TouchPhase::Moved,
    };
    
    // Pass to canvas controller
    canvas_controller.handle_cursor_event(event, renderer);
}
```

### 3. **Canvas Controller**
```rust
// Calculate zoom factor
let zoom_delta = delta_y * 0.001;
let zoom_factor = 1.0 + zoom_delta;

// Update data store
data_store.zoom(zoom_factor, mouse_x);
```

### 4. **Data Store Update**
```rust
pub fn zoom(&mut self, factor: f32, center_x: f32) {
    // Calculate new visible range
    let range = self.end_x - self.start_x;
    let new_range = range / factor;
    
    // Zoom around mouse position
    let pivot = self.start_x + (center_x / width) * range;
    self.start_x = pivot - (center_x / width) * new_range;
    self.end_x = pivot + (1.0 - center_x / width) * new_range;
    
    // Mark dirty for re-render
    self.mark_dirty();
}
```

### 5. **State Update**
```rust
// Update unified state
state.update_section(StateSection::View, StateData::View {
    zoom_level: new_zoom,
    pan_offset: new_pan,
    viewport_width, viewport_height
});
```

### 6. **Render Loop Trigger**
```rust
// State change triggers render loop
render_loop.trigger(UpdateTrigger::ViewChanged, instance_id);

// Transitions: Idle → Updating(View) → Rendering → Idle
```

### 7. **GPU Bounds Recalculation**
```rust
// Clear cached bounds
data_store.gpu_min_y = None;
data_store.gpu_max_y = None;

// Recalculate for new visible range
let (min_max_buffer, staging) = calculate_min_max_y(
    device, queue, encoder, data_store,
    new_start_x, new_end_x
);
```

### 8. **Render Execution**
```rust
// Each renderer updates for new view
for renderer in &mut multi_renderer.renderers {
    renderer.render(encoder, view, data_store, device, queue);
}
```

### 9. **Frame Presentation**
```rust
// Submit all GPU commands
queue.submit(encoder.finish());

// Present the frame
surface_texture.present();
```

### 10. **State Cleanup**
```rust
// Mark data store as clean
data_store.mark_clean();

// Render loop returns to Idle
render_loop.transition_to(RenderState::Idle);
```

## Key Architectural Principles

1. **Separation of Concerns**
   - State management separate from rendering
   - Each renderer independent and composable
   - Clear boundaries between crates

2. **GPU-First Design**
   - Heavy computation on GPU
   - Minimal CPU-GPU data transfer
   - Efficient resource management

3. **Change-Driven Updates**
   - Only update what changed
   - Smart change detection
   - Batched updates

4. **Predictable State Machine**
   - Simple 3-state design
   - Clear transitions
   - No complex state combinations

5. **Performance by Default**
   - Resource pooling
   - Pipeline caching
   - Adaptive frame rates

## Debugging and Monitoring

### Logging
```rust
log::debug!("[Renderer] GPU bounds: min={}, max={}", min, max);
log::info!("State transition: {:?} -> {:?}", old_state, new_state);
```

### Performance Metrics
```rust
FrameStats {
    avg_frame_time: 16.2ms,
    min_frame_time: 15.8ms,
    max_frame_time: 18.1ms,
    current_fps: 58.7,
    dropped_frames: 2,
}
```

### State History
```rust
// Recent state changes for debugging
state.get_history(10) // Last 10 state diffs
```

## Conclusion

The GPU Charts architecture achieves high performance through:
- Efficient state management with change detection
- GPU-accelerated computations
- Composable rendering pipeline
- Smart resource management
- Simple, predictable control flow

This design enables smooth 60 FPS rendering of millions of data points while maintaining code clarity and extensibility.