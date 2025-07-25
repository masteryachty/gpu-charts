# TriangleRenderer Documentation

The `TriangleRenderer` is a specialized renderer for displaying trade markers on charts. It renders fixed-size triangles at trade positions, with upward triangles for buy trades and downward triangles for sell trades.

## Features

- **Fixed-size triangles**: Triangles maintain consistent pixel size regardless of zoom level
- **Trade direction visualization**: Upward triangles for buy trades, downward for sell trades
- **Color coding**: Green for buy trades, red for sell trades
- **Instanced rendering**: Efficient rendering of many trade markers
- **Screen-space rendering**: Pixel-perfect triangles that don't distort with data scaling

## Usage

### Basic Usage

```rust
use renderer::charts::TriangleRenderer;
use shared_types::{TradeData, TradeSide};

// Create the renderer
let mut triangle_renderer = TriangleRenderer::new(
    device.clone(),
    queue.clone(),
    format,
);

// Create trade data
let trades = vec![
    TradeData {
        timestamp: 1234567890,
        price: 100.5,
        volume: 1.5,
        side: TradeSide::Buy,
    },
    TradeData {
        timestamp: 1234567900,
        price: 99.8,
        volume: 2.0,
        side: TradeSide::Sell,
    },
];

// Update the renderer with trade data
triangle_renderer.update_trades(&trades);

// Optionally set triangle size (default is 8 pixels)
triangle_renderer.set_triangle_size(10.0);
```

### Integration with MultiRenderer

The TriangleRenderer implements the `MultiRenderable` trait, making it compatible with the MultiRenderer system:

```rust
use renderer::{MultiRendererBuilder, RenderOrder};

let mut multi_renderer = MultiRendererBuilder::new(device, queue, format)
    .with_render_order(RenderOrder::BackgroundToForeground)
    .build();

// Add candlesticks first (background)
multi_renderer.add_renderer(Box::new(candlestick_renderer));

// Add trade triangles (foreground)
multi_renderer.add_renderer(Box::new(triangle_renderer));

// Add axes on top
multi_renderer.add_renderer(Box::new(x_axis_renderer));
multi_renderer.add_renderer(Box::new(y_axis_renderer));
```

## Technical Details

### Vertex Generation

The renderer uses instanced rendering with vertex generation in the shader:
- Each trade is an instance with position (timestamp, price) and side
- The vertex shader generates 3 vertices per instance to form a triangle
- Triangles are positioned in screen space for consistent sizing

### Shader Implementation

The `triangle.wgsl` shader:
1. Transforms trade positions from data space to NDC coordinates
2. Converts to screen space for pixel-perfect positioning
3. Generates triangle vertices based on trade side
4. Colors triangles based on buy/sell direction

### Performance Considerations

- **Instancing**: One draw call renders all triangles
- **No vertex buffer**: Vertices generated in shader reduce memory usage
- **Fixed size**: Screen-space sizing avoids recalculation on zoom
- **Priority**: Renders with priority 150 (on top of most elements)

## Example Integration

See `triangle_example.rs` for complete examples of:
- Creating a candlestick chart with trade markers
- Creating a line chart with trade markers
- Generating sample trade data for testing