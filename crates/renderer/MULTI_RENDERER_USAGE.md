# MultiRenderer Pipeline System

The MultiRenderer system provides a flexible way to combine multiple render types (lines, triangles, bars, etc.) in a single chart. This document explains how to use and extend the system.

## Overview

The MultiRenderer manages multiple chart renderers and executes them in a coordinated manner, sharing the same surface and device resources. It handles:

- Render order management
- Resource sharing between renderers
- Viewport and transform coordination
- Initialization and cleanup

## Basic Usage

### Creating a MultiRenderer

```rust
use renderer::{MultiRenderer, MultiRendererBuilder, RenderOrder};

// Using the builder pattern
let multi_renderer = MultiRendererBuilder::new(device, queue, format)
    .with_render_order(RenderOrder::BackgroundToForeground)
    .add_candlestick_renderer()
    .add_plot_renderer()
    .add_x_axis_renderer(width, height)
    .add_y_axis_renderer(width, height)
    .build();

// Or create manually
let mut multi_renderer = MultiRenderer::new(device, queue, format);
multi_renderer.add_renderer(Box::new(my_custom_renderer));
```

### Render Order Strategies

The system supports three render order strategies:

1. **Sequential**: Renders in the order renderers were added
2. **BackgroundToForeground**: Automatically sorts by priority
3. **Priority**: Custom priority-based ordering

```rust
// Background elements render first
multi_renderer.with_render_order(RenderOrder::BackgroundToForeground);
```

### Rendering

```rust
// In your render loop
multi_renderer.render(&mut encoder, &view, &data_store)?;

// Or use the convenience method
renderer.render_with_multi(&mut multi_renderer).await?;
```

## Creating Custom Renderers

To create a renderer compatible with MultiRenderer, implement the `MultiRenderable` trait:

```rust
use renderer::MultiRenderable;

struct MyCustomRenderer {
    // Your fields
}

impl MultiRenderable for MyCustomRenderer {
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        device: &Device,
        queue: &Queue,
    ) {
        // Your rendering logic
    }

    fn name(&self) -> &str {
        "MyCustomRenderer"
    }

    fn priority(&self) -> u32 {
        100 // Default priority
    }

    fn resize(&mut self, width: u32, height: u32) {
        // Handle resize if needed
    }
}
```

### Priority Guidelines

- **0-50**: Background elements (volume bars, grid, areas)
- **50-100**: Main chart elements (candles, lines)
- **100-150**: Foreground elements (overlays, indicators)
- **150+**: UI elements (axes, labels, tooltips)

## Example: Candlestick Chart with Volume

```rust
// Create a multi-renderer for candles + volume
let multi_renderer = renderer.create_multi_renderer()
    .with_render_order(RenderOrder::BackgroundToForeground)
    .build();

// Add volume bars (background)
let volume_renderer = VolumeBarRenderer::new(device, queue, format);
multi_renderer.add_renderer(Box::new(volume_renderer));

// Add candlesticks
let candle_renderer = CandlestickRenderer::new(device, queue, format);
multi_renderer.add_renderer(Box::new(candle_renderer));

// Add axes (foreground)
multi_renderer.add_renderer(Box::new(x_axis));
multi_renderer.add_renderer(Box::new(y_axis));
```

## Example: Multiple Line Plots

```rust
// Create multiple line renderers with different data/colors
let mut multi_renderer = MultiRenderer::new(device, queue, format);

for (i, metric) in metrics.iter().enumerate() {
    let mut plot_renderer = PlotRenderer::new(device, queue, format);
    plot_renderer.set_data_source(metric);
    plot_renderer.set_color(colors[i]);
    
    multi_renderer.add_renderer(Box::new(plot_renderer));
}

// Add shared axes
multi_renderer.add_renderer(Box::new(x_axis));
multi_renderer.add_renderer(Box::new(y_axis));
```

## Advanced Usage

### Using the Adapter Pattern

For existing renderers that don't implement `MultiRenderable`:

```rust
use renderer::RendererAdapter;

let my_renderer = MyOldRenderer::new();
let adapted = RendererAdapter::new(my_renderer, "MyRenderer")
    .with_priority(75)
    .with_clear(); // This renderer should clear the screen

multi_renderer.add_renderer(Box::new(adapted));
```

### Dynamic Renderer Management

```rust
// Clear all renderers
multi_renderer.clear_renderers();

// Check renderer count
let count = multi_renderer.renderer_count();

// Get renderer names for debugging
let names = multi_renderer.get_renderer_names();
println!("Active renderers: {:?}", names);
```

## Integration with Existing Code

The MultiRenderer system is designed to work alongside the existing rendering architecture. You can:

1. Use it to replace the manual render orchestration in `Renderer::render()`
2. Create specialized multi-renderers for specific chart types
3. Mix and match renderers dynamically based on user preferences

## Performance Considerations

1. **Render Pass Batching**: MultiRenderer executes all renderers in a single command encoder submission
2. **Resource Sharing**: All renderers share the same device and queue
3. **Clear Operations**: Only the first renderer (or one marked with `should_clear`) clears the screen
4. **Priority Sorting**: Sorting is stable and only happens when renderers are added

## Future Enhancements

The MultiRenderer system can be extended with:

- Render pass dependencies and ordering constraints
- Shared resource pools for common uniforms
- Automatic layout conflict resolution
- Performance profiling per renderer
- Conditional rendering based on viewport or zoom level