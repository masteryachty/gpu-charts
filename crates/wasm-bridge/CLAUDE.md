# WASM Bridge Crate - Central Orchestration Layer

## Purpose

The `wasm-bridge` crate serves as the central orchestration layer that bridges the Rust/WebAssembly world with JavaScript/React. It coordinates all other crates in the workspace and provides the JavaScript API for web integration.

## Core Architecture

### Position in the Architecture Stack

```
JavaScript/React Frontend
        ↑
    [wasm-bindgen boundary]
        ↑
wasm-bridge (orchestrator)
        ↑
    ├── config-system (presets)
    ├── data-manager (data operations)
    ├── renderer (GPU rendering)
    └── shared-types (common types)
```

### Key Responsibilities

1. **JavaScript Bindings**: Exposes Rust functionality to JavaScript via wasm-bindgen
2. **Instance Management**: Manages multiple chart instances with thread-local storage
3. **Event Coordination**: Handles user interactions and updates all subsystems
4. **Async Operations**: Manages async data fetching and GPU operations
5. **State Synchronization**: Keeps Rust and JavaScript states in sync

## JavaScript/React Integration

### Main Chart API (`src/lib.rs`)

The `Chart` struct is the primary interface exposed to JavaScript:

```rust
#[wasm_bindgen]
pub struct Chart {
    instance_id: Uuid,  // Unique identifier for this chart instance
}
```

#### Key Methods Exposed to JavaScript:

- **`new()`**: Constructor that initializes logging on first use
- **`init(canvas_id, width, height, start_x, end_x)`**: Async initialization with canvas
- **`apply_preset_and_symbol(preset, symbol)`**: Applies configuration and fetches data
- **`render()`**: Async rendering that spawns local tasks
- **`handle_mouse_*`**: Mouse event handlers for interactions
- **`toggle_metric_visibility(metric_label)`**: Dynamic metric control
- **`get_all_preset_names()`**: Returns available presets
- **`get_metrics_for_preset()`**: Returns metrics and visibility states

### wasm-bindgen Usage Patterns

#### 1. **Async Operations with Promises**

```rust
#[wasm_bindgen]
pub fn apply_preset_and_symbol(&mut self, preset: &str, symbol: &str) -> js_sys::Promise {
    // Creates JavaScript Promise
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        wasm_bindgen_futures::spawn_local(async move {
            // Async operations here
            resolve.call1(&JsValue::undefined(), &JsValue::from_bool(true)).unwrap();
        });
    });
    promise
}
```

#### 2. **JavaScript Array Returns**

```rust
#[wasm_bindgen]
pub fn get_all_preset_names(&self) -> Result<js_sys::Array, JsValue> {
    let names = js_sys::Array::new();
    for preset in preset_manager.get_all_presets() {
        names.push(&JsValue::from_str(&preset.name));
    }
    Ok(names)
}
```

#### 3. **Error Handling**

All errors are converted to `JsValue` for JavaScript consumption:

```rust
.map_err(|e| JsValue::from_str(&format!("Error: {e:?}")))?
```

## Event Handling System

### Event Flow Architecture

1. **JavaScript Events** → Browser DOM events
2. **WASM Bridge** → `handle_mouse_*` methods receive events
3. **Window Events** → Converted to internal `WindowEvent` types
4. **Canvas Controller** → Processes drag/zoom interactions
5. **Data Store** → Updates view ranges and marks dirty
6. **Render Trigger** → Initiates GPU rendering

### Mouse Event Processing (`handle_cursor_event`)

#### Zoom Handling (Mouse Wheel)
- Calculates zoom factor based on scroll delta
- Centers zoom on mouse position
- Updates X range in data store
- Marks data store dirty for re-render

#### Drag Zoom
- **Mouse Down**: Starts drag via canvas controller
- **Mouse Move**: Updates position during drag
- **Mouse Up**: Calculates new view range from drag positions

### Canvas Controller (`src/controls/canvas_controller.rs`)

Simple state holder for interaction tracking:
- Tracks current mouse position
- Manages drag start/end positions
- Returns drag ranges for zoom operations

## State Synchronization

### Data Store Management

The bridge coordinates state between:
- **Preset Configuration**: Chart types, metrics, visibility
- **View State**: X/Y ranges, zoom level
- **GPU State**: Computed bounds, buffers
- **Render State**: Dirty flags, pending operations

### GPU Bounds Synchronization

```rust
// Clear GPU bounds when data changes
data_store.gpu_min_y = None;
data_store.gpu_max_y = None;

// Force recalculation on next render
if data_store.is_dirty() {
    // GPU compute will recalculate bounds
}
```

## Chart Engine (`src/chart_engine.rs`)

The `ChartEngine` manages the complete rendering pipeline:

### Core Components

- **`render_context`**: WebGPU device, queue, surface management
- **`data_store`**: Centralized data and state
- **`compute_engine`**: GPU compute shader operations
- **`multi_renderer`**: Composite renderer for all chart types
- **`data_manager`**: HTTP data fetching and parsing
- **`canvas_controller`**: User interaction state

### Rendering Pipeline

1. **Initialization**: Creates WebGPU instance, surface, device
2. **Data Fetching**: Async HTTP requests for binary data
3. **GPU Compute**: Calculate min/max bounds via compute shaders
4. **Rendering**: Multi-pass rendering (plots, axes, labels)
5. **Readback**: Async GPU buffer readback for bounds

### Multi-Renderer Architecture

Dynamic renderer composition based on presets:

```rust
fn rebuild_multi_renderer_for_preset(&mut self, preset: &ChartPreset) {
    let mut builder = self.create_multi_renderer();
    
    for chart_type in &preset.chart_types {
        if !chart_type.visible {
            continue;
        }
        
        match chart_type.render_type {
            RenderType::Line => builder.add_plot_renderer(),
            RenderType::Candlestick => builder.add_candlestick_renderer(),
            RenderType::Triangle => builder.add_triangle_renderer(),
            // etc...
        }
    }
    
    builder.add_axes_renderers().build()
}
```

## Instance Manager (`src/instance_manager.rs`)

Thread-local storage pattern for managing multiple chart instances:

### Design Pattern

```rust
thread_local! {
    static CHART_INSTANCES: RefCell<HashMap<Uuid, ChartInstance>> = RefCell::new(HashMap::new());
}
```

### Key Operations

- **`create_instance`**: Creates new chart with unique UUID
- **`with_instance`**: Borrows instance for read operations
- **`with_instance_mut`**: Borrows instance for mutations
- **`take_instance`/`put_instance`**: For async operations requiring ownership

### Async Operation Pattern

```rust
// Take instance for async work
let instance_opt = InstanceManager::take_instance(&instance_id);

if let Some(mut instance) = instance_opt {
    // Perform async operations
    let result = async_operation(&mut instance).await;
    
    // Put instance back
    InstanceManager::put_instance(instance_id, instance);
}
```

## Memory Management Patterns

### Rc/RefCell Usage

Not used in current implementation - instances managed via thread-local storage.

### GPU Buffer Management

- Buffers created in data-manager crate
- Staging buffers for GPU readback
- Proper cleanup in Drop implementations

### Async Memory Safety

```rust
// Use raw pointers for async operations to avoid borrow checker issues
let data_store_ptr = instance.chart_engine.data_store_mut() as *mut DataStore;
let data_store = unsafe { &mut *data_store_ptr };

// Safe because we control the lifetime via instance manager
```

## WASM-Specific Optimizations

### 1. **Lazy Initialization**

Logger initialized once on first Chart construction:

```rust
static LOGGER_INIT: std::sync::Once = std::sync::Once::new();
LOGGER_INIT.call_once(|| {
    console_log::init_with_level(log::Level::Debug);
});
```

### 2. **Async Rendering**

Non-blocking renders via spawn_local:

```rust
wasm_bindgen_futures::spawn_local(async move {
    InstanceManager::with_instance_mut(&instance_id, |instance| {
        let _ = instance.chart_engine.render();
    });
});
```

### 3. **Efficient Event Handling**

Events processed synchronously but trigger async renders:

```rust
pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, _y: f64) -> Result<(), JsValue> {
    // Synchronous event processing
    instance.chart_engine.handle_cursor_event(window_event);
    
    // Trigger async render if needed
    let _ = instance.chart_engine.render();
}
```

## Async Operation Handling

### Data Fetching Pattern

```rust
async fn fetch_and_process_data(instance_id: Uuid) -> Result<(), GpuChartsError> {
    // Take instance ownership temporarily
    let instance_opt = InstanceManager::take_instance(&instance_id);
    
    if let Some(mut instance) = instance_opt {
        // Perform async fetch
        let result = instance.chart_engine.data_manager
            .fetch_data_for_preset(data_store)
            .await;
        
        // Return instance
        InstanceManager::put_instance(instance_id, instance);
        result
    } else {
        Err(GpuChartsError::DataNotFound)
    }
}
```

### GPU Readback Handling

```rust
fn process_pending_readback(&mut self) -> bool {
    // Check for pending readback
    if let Some(pending) = &mut self.pending_readback {
        // Initiate async mapping
        buffer_slice.map_async(wgpu::MapMode::Read, callback);
        
        // Poll device to make progress
        self.render_context.device.poll(wgpu::Maintain::Poll);
        
        // Check completion and process data
        if mapping_complete {
            // Read buffer data and update bounds
        }
    }
}
```

## Coordination with Other Crates

### Config System Integration

- Fetches presets via `PresetManager`
- Rebuilds renderers when presets change
- Updates data store with preset configuration

### Data Manager Integration

- Delegates data fetching to `DataManager`
- Passes data store for direct updates
- Handles async fetch completion

### Renderer Integration

- Creates and manages `MultiRenderer`
- Configures render pipeline based on presets
- Triggers render passes and manages surface

### Shared Types Usage

- Uses common event types (`WindowEvent`, `MouseButton`, etc.)
- Error types (`GpuChartsError`)
- Store state types for configuration

## JavaScript Wrappers (`src/wrappers/js.rs`)

### Query Parameter Extraction

```rust
pub fn get_query_params() -> HashMap<String, String> {
    // Access browser window
    let location = window().expect("should have a Window").location();
    
    // Parse URL search params
    let params = UrlSearchParams::new_with_str(&search)?;
    
    // Convert to HashMap
    for entry in js_sys::try_iter(&params.entries()) {
        map.insert(key, value);
    }
}
```

## Error Handling Patterns

### JavaScript-Friendly Errors

All public methods return `Result<T, JsValue>`:

```rust
#[wasm_bindgen]
pub fn method(&self) -> Result<(), JsValue> {
    operation()
        .map_err(|e| JsValue::from_str(&format!("Error: {e:?}")))?;
    Ok(())
}
```

### Graceful Degradation

```rust
// Return default values on error
.unwrap_or(false)
.unwrap_or_default()

// Skip operations if instance not found
.ok_or_else(|| JsValue::from_str("Chart instance not found"))?;
```

## Performance Considerations

### Minimizing JS-WASM Overhead

1. **Batch Operations**: Process multiple changes in single call
2. **Async Rendering**: Non-blocking render operations
3. **Lazy Updates**: Only update when data is dirty
4. **Direct Memory Access**: Use typed arrays where possible

### Efficient State Updates

```rust
// Only render if needed
if !self.data_store.is_dirty() {
    return Ok(());
}

// Check for pending GPU operations
if self.pending_readback.is_some() {
    // Wait for completion
}
```

## Testing Considerations

### WASM Testing Challenges

- Cannot use standard Rust tests directly
- Use `wasm-bindgen-test` for WASM-specific tests
- Mock browser APIs for unit testing
- Integration tests require browser environment

### Debug Logging

```rust
// Use console_log for debugging
log::debug!("[bridge] Operation completed");
log::error!("[BRIDGE] Render failed: {e:?}");
```

## Best Practices

1. **Always use UUIDs** for instance identification
2. **Handle async operations** with proper ownership transfers
3. **Clear GPU state** when data changes significantly
4. **Validate input** from JavaScript before processing
5. **Use thread-local storage** instead of global state
6. **Spawn local tasks** for async operations
7. **Poll GPU device** to progress async operations
8. **Clean up resources** in Drop implementations
9. **Log errors** for debugging in browser console
10. **Return detailed errors** to JavaScript for handling