# WASM Bridge Crate - CLAUDE.md

This file provides guidance for working with the wasm-bridge crate, which serves as the central orchestration layer bridging JavaScript/React and the Rust/WebGPU implementation.

## Overview

The wasm-bridge crate provides:
- WebAssembly bindings for JavaScript integration
- React state synchronization
- Event handling and user interaction management
- Orchestration of all other crates
- Smart change detection for efficient updates
- Performance monitoring and metrics

## Architecture Position

```
shared-types
    ↑
├── config-system
├── data-manager
├── renderer
    ↑
wasm-bridge (this crate) → JavaScript/React
```

This crate is the top-level orchestrator that coordinates all other crates and exposes functionality to JavaScript.

## Key Components

### Chart Class (`src/lib.rs`)
Main WebAssembly interface exposed to JavaScript:

```rust
#[wasm_bindgen]
pub struct Chart {
    instance_id: u32,
}

#[wasm_bindgen]
impl Chart {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Chart;
    
    pub async fn init(&self, canvas_id: &str, width: u32, height: u32) -> Result<(), JsValue>;
    pub async fn render(&self) -> Result<(), JsValue>;
    pub fn update_chart_state(&self, store_state_json: &str) -> Result<String, JsValue>;
    
    // Mouse/interaction handlers
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue>;
    pub fn handle_mouse_move(&self, x: f64, y: f64) -> Result<(), JsValue>;
    pub fn handle_mouse_click(&self, x: f64, y: f64, pressed: bool) -> Result<(), JsValue>;
}
```

### LineGraph (`src/line_graph.rs`)
Core chart implementation managing the rendering pipeline:

```rust
pub struct LineGraph {
    pub data_store: Rc<RefCell<DataStore>>,
    pub engine: Rc<RefCell<RenderEngine>>,
    
    gpu_plotter: Option<GpuPlotter>,
    selected_columns: Vec<String>,
    chart_type: String,
}
```

### CanvasController (`src/controls/canvas_controller.rs`)
Handles user interactions and transforms them into data operations:

```rust
pub struct CanvasController {
    data_store: Rc<RefCell<DataStore>>,
    engine: Rc<RefCell<RenderEngine>>,
    
    // Interaction state
    is_dragging: bool,
    last_mouse_pos: Option<PhysicalPosition<f64>>,
    zoom_sensitivity: f32,
}
```

## React Integration

### Store State Synchronization

The bridge provides smart state synchronization with React:

```rust
// React sends state updates as JSON
pub fn update_chart_state(&self, store_state_json: &str) -> Result<String, JsValue> {
    // 1. Deserialize and validate
    let store_state = self.deserialize_and_validate_store_state(store_state_json)?;
    
    // 2. Detect changes
    let changes = store_state.detect_changes_from(&current_state, &config);
    
    // 3. Apply only necessary updates
    if changes.requires_data_fetch {
        self.trigger_data_fetch();
    }
    if changes.requires_render {
        self.request_render();
    }
    
    // 4. Return detailed response
    Ok(json!({
        "success": true,
        "changes": changes,
        "updated": true
    }).to_string())
}
```

### Change Detection Configuration

```rust
#[derive(Serialize, Deserialize)]
pub struct ChangeDetectionConfig {
    pub enable_symbol_change_detection: bool,
    pub enable_time_range_change_detection: bool,
    pub enable_timeframe_change_detection: bool,
    pub enable_indicator_change_detection: bool,
    pub symbol_change_triggers_fetch: bool,
    pub time_range_change_triggers_fetch: bool,
    pub minimum_time_range_change_seconds: u64,
}
```

## JavaScript API

### TypeScript Definitions
```typescript
// From wasm.d.ts
export class Chart {
    constructor();
    init(canvas_id: string, width: number, height: number): Promise<void>;
    update_chart_state(store_state_json: string): string;
    
    // Rendering
    render(): Promise<void>;
    needs_render(): boolean;
    resize(width: number, height: number): void;
    
    // Interactions
    handle_mouse_wheel(delta_y: number, x: number, y: number): void;
    handle_mouse_move(x: number, y: number): void;
    handle_mouse_click(x: number, y: number, pressed: boolean): void;
    
    // Configuration
    set_chart_type(chart_type: string): void;
    set_candle_timeframe(timeframe_seconds: number): void;
    configure_change_detection(config_json: string): string;
    get_change_detection_config(): string;
}
```

### Usage from React
```typescript
// Initialize
const chart = new Chart();
await chart.init('canvas-id', 800, 600);

// Update state
const storeState = {
    currentSymbol: 'BTC-USD',
    ChartStateConfig: {
        symbol: 'BTC-USD',
        timeframe: '1h',
        startTime: Date.now() - 86400000,
        endTime: Date.now(),
        selectedMetrics: ['price', 'volume']
    },
    isConnected: true
};

const result = chart.update_chart_state(JSON.stringify(storeState));
const response = JSON.parse(result);

if (response.success) {
    console.log('Applied changes:', response.changes);
}
```

## Event Handling

### Mouse Events Flow
1. React captures DOM events
2. Calls WASM bridge methods
3. Bridge transforms to internal events
4. CanvasController processes events
5. Updates DataStore (zoom/pan)
6. Triggers re-render if needed

### Example Event Processing
```rust
pub fn handle_cursor_event(&mut self, event: WindowEvent) {
    match event {
        WindowEvent::MouseWheel { delta, .. } => {
            // Calculate zoom
            let zoom_delta = match delta {
                MouseScrollDelta::LineDelta(_, y) => y * 0.1,
                MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.001,
            };
            
            // Apply zoom to data store
            self.data_store.borrow_mut().zoom(1.0 + zoom_delta, mouse_x);
        }
        WindowEvent::CursorMoved { position } => {
            if self.is_dragging {
                // Calculate pan delta
                let delta_x = position.x - self.last_mouse_pos.x;
                self.data_store.borrow_mut().pan(delta_x, 0.0);
            }
            self.last_mouse_pos = Some(position);
        }
        _ => {}
    }
}
```

## Performance Considerations

### Minimizing JS-WASM Overhead
1. **Batch Updates**: Group multiple state changes
2. **Smart Detection**: Only update what changed
3. **Avoid Frequent Calls**: Use debouncing in React
4. **Efficient Serialization**: Use JSON for simplicity, consider binary for performance

### Memory Management
```rust
// Use Rc<RefCell<>> for shared ownership within WASM
let data_store = Rc::new(RefCell::new(DataStore::new()));

// Clone Rc before async operations to avoid borrow conflicts
let data_store_clone = data_store.clone();
spawn_local(async move {
    fetch_data(&data_store_clone).await;
});
```

## Error Handling

### WASM-Friendly Errors
```rust
// Return errors as JSON strings for JavaScript
match operation() {
    Ok(result) => Ok(json!({
        "success": true,
        "data": result
    }).to_string()),
    Err(e) => Ok(json!({
        "success": false,
        "error": e.to_string()
    }).to_string())
}
```

### JavaScript Error Handling
```typescript
try {
    const result = JSON.parse(chart.update_chart_state(stateJson));
    if (!result.success) {
        console.error('Chart update failed:', result.error);
    }
} catch (e) {
    console.error('WASM call failed:', e);
}
```

## Best Practices

1. **Keep State in Sync**: Always validate state before applying
2. **Handle Async Properly**: Use `spawn_local` for async operations
3. **Avoid RefCell Panics**: Use `try_borrow` when possible
4. **Profile Performance**: Use browser DevTools
5. **Test Edge Cases**: Network failures, invalid data, etc.

## Debugging Tips

### Enable Logging
```rust
cfg_if::cfg_if! {
    if  {
        console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    }
}
```

### Debug State Changes
```rust
log::info!("State change detected: {:?}", change_detection);
log::debug!("Current state: {:?}", self.current_store_state);
```

### Browser Console
```javascript
// Enable verbose logging
localStorage.debug = 'wasm:*';

// Check WASM module status
console.log(wasmModule);
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_state_validation() {
        let invalid_state = r#"{"ChartStateConfig": {"startTime": 100, "endTime": 50}}"#;
        let result = deserialize_and_validate_store_state(invalid_state);
        assert!(result.is_err());
    }
}
```

### Integration Tests
```rust
#[wasm_bindgen_test]
async fn test_chart_initialization() {
    let chart = Chart::new();
    let result = chart.init("test-canvas", 800, 600).await;
    assert!(result.is_ok());
}
```

## Future Enhancements

- Binary protocol for state updates
- SharedArrayBuffer for zero-copy data
- Web Workers for parallel processing
- WebTransport for streaming data
- Multiple chart instances support
- Offline rendering capabilities
- WebGL fallback option