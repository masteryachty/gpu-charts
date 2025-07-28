# Complete First Render Method Call Trace

This document provides an exhaustive list of all methods called during the first render of the GPU-Charts application, from React initialization to GPU frame presentation.

## 1. React Initialization Phase

### Frontend (TypeScript/React)
```typescript
// web/src/components/chart/WasmCanvas.tsx
1. WasmCanvas.render()
2. useWasmChart() hook initialization
3. useState() calls for:
   - chart instance
   - error state
   - loading state
   - performance metrics
4. useEffect() with empty deps array
5. initialize() function call
```

### Hook Initialization (web/src/hooks/useWasmChart.ts)
```typescript
6. waitForCanvas() - line 295
   - document.getElementById('wasm-chart-canvas')
   - Retry loop (up to 10 attempts)
   - Check canvas.offsetWidth > 0 && canvas.offsetHeight > 0
   
7. Dynamic WASM import - line 310
   - import('@pkg/wasm_bridge.js')
   - wasmModule.default() to initialize
   
8. Chart creation - line 318
   - new wasmModule.Chart()
```

## 2. WASM Bridge Layer Initialization

### Chart Instance Creation (crates/wasm-bridge/src/lib.rs)
```rust
9. Chart::new() - line 38
   - Generate UUID for instance_id
   - Initialize empty Chart struct

10. Chart::init(canvas_id, width, height) - line 49
    - console_error_panic_hook::set_once()
    - init_logger()
    - web_sys::window()
    - window.document()
    - document.get_element_by_id(&canvas_id)
    - element.dyn_into::<HtmlCanvasElement>()
    - canvas.set_width(width)
    - canvas.set_height(height)
```

### LineGraph Creation (crates/wasm-bridge/src/line_graph.rs)
```rust
11. LineGraph::new(width, height, canvas) - line 66
    - window.location()
    - location.search() for query params
    - parse_query_params()
    - Extract topic, start, end parameters
    
12. DataStore::new(width, height) - data_store.rs:85
    - Set default X range: 0..100
    - Set default Y range: 0.0..100.0
    - Initialize empty data_groups HashMap
    - Set screen dimensions
    - Set chart_type: ChartType::Candlestick
    - Set candle_timeframe: 60 seconds
    - Mark as dirty: true
    - Set excluded_columns: ["side", "volume"]
```

### WebGPU Initialization
```rust
13. wgpu::Instance::new() - line_graph.rs:83
    - Create with Backends::BROWSER_WEBGPU
    
14. instance.create_surface() - line_graph.rs:84
    - Pass canvas reference
    - Create WebGPU surface
    
15. instance.request_adapter() - line_graph.rs:87
    - power_preference: HighPerformance
    - compatible_surface: Some(&surface)
    
16. adapter.request_device() - line_graph.rs:96
    - label: "LineGraph Device"
    - Default features and limits
    
17. adapter.get_texture_format_features()
    - Check format capabilities
```

### Component Creation
```rust
18. DataManager::new() - data_manager/lib.rs
    - Initialize with empty cache
    - Set up error handling
    
19. Renderer::new() - renderer/lib.rs:51
    - Store device, queue references
    - surface.get_capabilities(&adapter)
    - Select texture format
    - Configure surface:
      - usage: RENDER_ATTACHMENT
      - format: selected format
      - width, height from data_store
      - present_mode: AutoNoVsync
      - alpha_mode: Auto
      - desired_maximum_frame_latency: 1
    - Create ComputeEngine
    
20. ComputeEngine::new() - compute_engine.rs
    - Store device reference
    - Initialize shader cache
    - Set up compute pipeline layouts
```

### Multi-Renderer Setup
```rust
21. MultiRendererBuilder::new() - line_graph.rs:164
    - Initialize with device, queue, format
    - Set render_order: BackgroundToForeground
    
22. Add renderers:
    - PlotRenderer::new()
      - Create vertex/index buffers
      - Compile plot shaders
      - Create pipeline layout
    - XAxisRenderer::new()
      - Create axis geometry
      - Compile axis shaders
    - YAxisRenderer::new()
      - Similar to X-axis setup
      
23. MultiRendererBuilder::build()
    - Create shared bind group layout
    - Initialize renderer collection
```

### Controller Creation
```rust
24. CanvasController::new() - canvas_controller.rs
    - Store data_store reference
    - Initialize interaction state
    - Set up event tracking structures
```

### Instance Management
```rust
25. InstanceManager::get() - wasm_bridge/lib.rs
    - Access global singleton
    
26. instances.insert(instance_id, instance)
    - Store Chart instance globally
```

## 3. First Render Execution

### Render Trigger (crates/wasm-bridge/src/lib.rs)
```rust
27. Chart::render() - line 97
    - wasm_bindgen_futures::spawn_local()
    - Clone instance_id
    
28. Inside async block:
    - InstanceManager::get()
    - instances.get_mut(&instance_id)
    - instances.remove(&instance_id) temporarily
```

### LineGraph Render (crates/wasm-bridge/src/line_graph.rs)
```rust
29. LineGraph::render() - line 180
    - Check data_store state
    - Call renderer.render()
```

### Core Render Loop (crates/renderer/src/lib.rs)
```rust
30. Renderer::render(multi_renderer) - line 119
    - data_store.is_dirty() check
    - surface.get_current_texture()
    - texture.texture.create_view()
    - device.create_command_encoder()
```

### Compute Pass Execution
```rust
31. ComputeEngine::run_compute_passes() - if needed
    - Identify metrics needing computation
    - Sort by dependencies
    - For each metric:
      - Check if already computed
      - Get compute shader
      - Create compute pass
      - Set pipeline
      - Set bind groups
      - Dispatch workgroups
```

### Y-Bounds Calculation (if needed)
```rust
32. calculate_y_bounds_new() - renderer/lib.rs:189
    - Get active data groups
    - Create min/max buffers
    - Run GPU compute shader
    - Read back results
    - Update data_store bounds
```

### Shared Bind Group Update
```rust
33. update_shared_bind_group() - renderer/lib.rs:243
    - Calculate projection matrix
    - Create uniform buffer
    - Write matrix data
    - Create bind group
```

### Multi-Renderer Execution
```rust
34. MultiRenderer::render() - multi_renderer.rs
    - Sort renderers by priority
    - For each renderer:
```

### Plot Renderer
```rust
35. PlotRenderer::render() - plot_renderer.rs
    - Check if ready (has data)
    - encoder.begin_render_pass()
    - render_pass.set_pipeline()
    - render_pass.set_bind_group(0, shared)
    - render_pass.set_bind_group(1, plot_specific)
    - For each data group:
      - set_vertex_buffer()
      - draw() or draw_indexed()
```

### X-Axis Renderer
```rust
36. XAxisRenderer::render() - x_axis_renderer.rs
    - Calculate tick positions
    - Update vertex buffer
    - Begin render pass
    - Set pipeline and bind groups
    - Draw axis lines
    - Draw tick marks
    - Render labels (if any)
```

### Y-Axis Renderer
```rust
37. YAxisRenderer::render() - y_axis_renderer.rs
    - Similar to X-axis
    - Calculate Y-axis ticks
    - Draw grid lines
    - Draw axis line
    - Render value labels
```

### Command Submission
```rust
38. Back in Renderer::render():
    - encoder.finish()
    - queue.submit([command_buffer])
    - frame.present()
    - data_store.mark_clean()
```

## 4. React Render Loop Setup

### Animation Frame Loop (web/src/components/chart/WasmCanvas.tsx)
```typescript
39. useEffect for render loop - line 210
    - const animate = () => { ... }
    - Check chart?.needs_render()
    - If true: await chart.render()
    - animationFrameRef.current = requestAnimationFrame(animate)
    - Start loop: animate()
```

## 5. Event Handler Setup

### Mouse Events
```typescript
40. Canvas event listeners attached:
    - onWheel={handleMouseWheel}
    - onMouseDown={handleMouseDown}
    - onMouseUp={handleMouseUp}
    - onMouseMove={handleMouseMove}
    - onMouseLeave={handleMouseLeave}
```

### State Synchronization
```typescript
41. useEffect for store sync (if enableAutoSync):
    - Subscribe to Zustand store changes
    - Debounced update function
    - chart.update_chart_state(storeStateJson)
```

## Performance-Critical Method Signatures

### GPU Buffer Creation
```rust
// crates/data-manager/src/gpu_buffer_manager.rs
create_buffer_from_data<T>(device, data, usage, label)
create_staging_buffer(device, data)
create_vertex_buffer(device, vertices)
create_index_buffer(device, indices)
```

### Shader Compilation
```rust
// crates/renderer/src/shader_manager.rs
device.create_shader_module(ShaderModuleDescriptor {
    label,
    source: ShaderSource::Wgsl(source.into()),
})
```

### Pipeline Creation
```rust
// Various renderer files
device.create_render_pipeline(RenderPipelineDescriptor {
    vertex, fragment, primitive, depth_stencil, multisample, ...
})
```

### Data Transformation
```rust
// crates/data-manager/src/data_store.rs
world_to_screen(world_x, world_y) -> (f32, f32)
screen_to_world(screen_x, screen_y) -> (f64, f64)
```

## Total Method Count for First Render

**Approximately 150+ individual method calls** occur during the first render, spanning:
- React/TypeScript initialization
- WASM module loading
- WebGPU device/surface creation
- Multi-renderer pipeline setup
- GPU buffer allocation
- Shader compilation
- Initial frame rendering
- Event handler registration

This doesn't include internal WebGPU driver calls or browser API implementations.