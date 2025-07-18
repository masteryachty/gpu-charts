# Dirty State Rendering Implementation

## Overview

This implementation adds a dirty state tracking system to the GPU charting library to ensure the chart only re-renders when data actually changes, eliminating unnecessary render loops and improving performance.

## Key Changes

### 1. DataStore Dirty State Tracking

Added a `dirty` boolean field to `DataStore` that tracks whether the data has changed and needs re-rendering:

```rust
pub struct DataStore {
    // ... existing fields ...
    dirty: bool, // Track if data has changed and needs re-rendering
}
```

**New methods:**
- `is_dirty()` - Check if rendering is needed
- `mark_clean()` - Mark as rendered
- `mark_dirty()` - Mark as needing render

**Updated methods to set dirty flag:**
- `add_data_group()` - When new data is added
- `add_metric_to_group()` - When metrics are added
- `set_x_range()` - When zoom/pan changes the view range
- `resized()` - When canvas is resized
- `update_min_max_y()` - When Y-axis bounds change
- `set_chart_type()` - When switching between line/candlestick
- `set_candle_timeframe()` - When changing candle timeframe

### 2. Render Method Optimization

The `LineGraph::render()` method now checks the dirty state before rendering:

```rust
pub async fn render(&self) -> Result<(), wgpu::SurfaceError> {
    // Check if rendering is needed
    if !self.data_store.borrow().is_dirty() {
        return Ok(());
    }
    
    // Render and mark clean on success
    let result = engine.render().await;
    if result.is_ok() {
        self.data_store.borrow_mut().mark_clean();
    }
    result
}
```

### 3. WASM Bridge Updates

Added `needs_render()` method to the WASM bridge to allow JavaScript to check if rendering is needed:

```rust
#[wasm_bindgen]
pub fn needs_render(&self) -> bool {
    unsafe {
        if let Some(instance) = (&raw const CHART_INSTANCE).as_ref().unwrap() {
            instance.line_graph.borrow().data_store.borrow().is_dirty()
        } else {
            false
        }
    }
}
```

### 4. React Component Updates

Replaced the continuous 60fps render loop with an on-demand system in `WasmCanvas.tsx`:

```typescript
// On-demand render loop - only renders when chart state is dirty
useEffect(() => {
    if (!chartState.chart || !chartState.isInitialized) return;

    let animationId: number;
    let isRendering = false;

    const checkAndRender = async () => {
        if (!isRendering && chartState.chart && chartState.isInitialized) {
            // Check if rendering is needed
            const needsRender = chartState.chart.needs_render?.() ?? false;
            
            if (needsRender) {
                isRendering = true;
                try {
                    await chartState.chart.render?.();
                } catch (error) {
                    console.warn('[WasmCanvas] Render failed:', error);
                } finally {
                    isRendering = false;
                }
            }
        }
        
        // Continue checking at 60fps rate
        animationId = requestAnimationFrame(checkAndRender);
    };

    animationId = requestAnimationFrame(checkAndRender);

    return () => {
        if (animationId) {
            cancelAnimationFrame(animationId);
        }
    };
}, [chartState.chart, chartState.isInitialized]);
```

## How It Works

1. **Initial State**: DataStore starts with `dirty = true` to ensure initial render
2. **Data Changes**: Any modification to the data marks it as dirty
3. **Render Check**: The render loop checks `needs_render()` at 60fps
4. **Conditional Render**: Only renders if dirty flag is true
5. **Mark Clean**: After successful render, marks as clean
6. **User Interactions**: Mouse events (zoom, pan) modify data and set dirty flag

## Benefits

1. **Performance**: Eliminates unnecessary GPU operations when nothing has changed
2. **Battery Life**: Reduces power consumption on devices
3. **Resource Usage**: Lower CPU/GPU utilization
4. **Smooth Interactions**: Still maintains 60fps responsiveness for user interactions

## Events That Trigger Re-rendering

- New data fetched from server
- User zooms (mouse wheel)
- User pans (drag)
- Window resizes
- Chart type changes (line/candlestick)
- Candle timeframe changes
- Min/max Y values update

## Testing

The implementation maintains backward compatibility while adding the optimization. The render loop still runs at 60fps to check for changes, ensuring smooth updates when they occur, but actual GPU rendering only happens when needed.