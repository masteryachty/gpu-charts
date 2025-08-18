# Renderer Crate - GPU Rendering Engine Documentation

## Purpose and Architecture

The renderer crate is a high-performance, pure GPU rendering engine built on WebGPU/WGPU for real-time financial data visualization. It provides hardware-accelerated rendering of large datasets with multiple chart types, leveraging GPU compute shaders for data processing and transformation.

### Core Design Principles
- **GPU-First Architecture**: All heavy computation happens on the GPU via compute shaders
- **Zero-Copy Rendering**: Direct GPU buffer usage without CPU-GPU roundtrips
- **Modular Pipeline System**: Composable renderers that can be combined via MultiRenderer
- **WGSL Shader-Driven**: All rendering logic implemented in WebGPU Shading Language
- **Memory-Efficient**: Shared buffer strategies and GPU-resident data

## Architecture Components

### Render Context Management (`render_context.rs`)
Central WebGPU resource holder:
```rust
pub struct RenderContext {
    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
    pub surface: wgpu::Surface<'static>,  // 'static lifetime for WASM context
    pub config: wgpu::SurfaceConfiguration,
}
```
- Manages surface configuration and resizing
- Provides texture acquisition for render targets
- Shared device/queue references for all renderers

### Multi-Renderer System (`multi_renderer.rs`)
Orchestrates multiple chart renderers in a single view:
- **MultiRenderable Trait**: Common interface for all renderers
- **Render Order Strategies**:
  - `Sequential`: Render in insertion order
  - `BackgroundToForeground`: Background elements (bars, areas) first
  - `Priority`: Custom priority-based ordering (0-255)
- **Compute Pass Integration**: Runs compute shaders before rendering
- **Automatic Clear Management**: First renderer clears the frame

Key Features:
- Dynamic renderer composition
- Priority-based rendering (background: 0-50, midground: 100, foreground: 150+)
- Shared resource management
- Builder pattern for convenient setup

### Pipeline Builder (`pipeline_builder.rs`)
Currently commented out but provides structured pipeline creation patterns for:
- Vertex buffer layout configuration
- Shader module management
- Render pipeline state setup
- Blend mode configuration

## GPU Compute Infrastructure

### Compute Engine (`compute_engine.rs`)
Central orchestrator for GPU compute operations:
- **Dependency Resolution**: Topological sort for compute operation ordering
- **Compute Calculators**: Pluggable compute shader processors
- **Frame Caching**: Tracks computed metrics to avoid redundant calculations
- **Operations Supported**:
  - Average (Mid Price calculation)
  - Sum, Difference, Product, Ratio (planned)
  - Min/Max aggregations
  - Weighted averages

### Compute Processor Framework (`compute/`)
Generic infrastructure for GPU compute operations:

#### ComputeProcessor Trait (`compute_processor.rs`)
```rust
pub trait ComputeProcessor {
    fn compute(device, queue, encoder) -> Result<ComputeResult>;
    fn name() -> &str;
}
```

#### Mid Price Calculator (`mid_price_calculator.rs`)
GPU-accelerated bid/ask spread calculations:
- Computes `(bid + ask) / 2` for millions of data points
- Handles edge cases (missing bid/ask values)
- Additional functions: spread, spread percentage
- 256-thread workgroups for optimal GPU occupancy

## Shader Architecture (WGSL)

### Compute Shaders

#### Min/Max Calculation (`calcables/min_max_*.wgsl`)
Two-phase parallel reduction algorithm:
1. **First Phase** (`min_max_first.wgsl`):
   - 256-thread workgroups
   - Each thread processes multiple elements (configurable multiplier)
   - Thread-local min/max accumulation
   - Shared memory reduction tree
   - Handles NaN/Infinity filtering
   - Outputs partial results per workgroup

2. **Second Phase** (`min_max_second.wgsl`):
   - Reduces partial results from first phase
   - Final global min/max computation
   - Handles empty data ranges gracefully

3. **Overall Min/Max** (`overall_min_max.wgsl`):
   - Combines multiple min/max buffers
   - Used for multi-metric bounds calculation

#### Candle Aggregation (`calcables/candle_aggregation.wgsl`)
Parallel OHLC candle generation from tick data:
- **64-thread workgroups** (one workgroup per candle)
- **Binary search optimization** for sorted timestamps
- **Parallel tick scanning** with thread-local accumulation
- **Shared memory reduction** for OHLC values
- **Empty candle handling** (uses previous close)
- Optimizations:
  - Early exit for out-of-range threads
  - Efficient first/last tick tracking
  - Sentinel values for uninitialized data

#### Mid Price Compute (`compute/mid_price_compute.wgsl`)
Calculates derived metrics from bid/ask data:
- **256-thread workgroups** for maximum throughput
- Three compute entry points:
  - `compute_mid_price`: Average of bid/ask
  - `compute_spread`: Ask - Bid difference
  - `compute_spread_percentage`: Percentage spread
- Robust edge case handling for missing data

### Rendering Shaders

#### Plot Renderer (`drawables/plot.wgsl`)
Line chart rendering with GPU transformation:
- **Vertex Shader**:
  - Handles u32 timestamps with precision preservation
  - World-to-screen matrix transformation
  - 10% Y-axis margin for visual padding
  - Unsigned integer underflow protection
- **Fragment Shader**:
  - Per-metric color application
  - Simple pass-through for efficiency

#### Candlestick Renderer (`drawables/candlestick.wgsl`)
Financial candlestick visualization:
- **Two-pass rendering**:
  1. Bodies: 6 vertices per candle (2 triangles)
  2. Wicks: 4 vertices per candle (2 lines)
- **Features**:
  - 80% timeframe width for visual gaps
  - Minimum body height enforcement (0.5% of range)
  - Bullish (green), bearish (red), doji (yellow) coloring
  - Centered candle positioning
- **GPU-optimized vertex generation** from compute shader output

#### Triangle Renderer (`charts/triangle.wgsl`)
Trade marker visualization:
- **Instance-based rendering** (3 vertices per triangle)
- **Fixed pixel-size triangles** (screen-space consistent)
- **Direction-based shapes**:
  - Upward triangles for buy trades (green)
  - Downward triangles for sell trades (red)
- **Screen-space calculations** for pixel-perfect rendering
- **NDC transformation** with proper Y-axis inversion

#### Axis Renderers (`drawables/x_axis.wgsl`, `y_axis.wgsl`)
Dynamic axis label generation:
- GPU-based text positioning
- Automatic label scaling
- Grid line integration
- Time formatting for X-axis
- Scientific notation support for Y-axis

## Rendering Components (Drawables)

### PlotRenderer (`drawables/plot.rs`)
Multi-line chart renderer:
- **Data Filtering**: Selective column rendering
- **Per-Metric Bind Groups**: Individual color/style per line
- **Multi-Buffer Support**: Handles segmented data
- **Dynamic vertex generation** from data buffers

### CandlestickRenderer (`drawables/candlestick.rs`)
OHLC financial chart renderer:
- **GPU Compute Integration**: Reads directly from aggregated candle buffer
- **Two-stage rendering**: Bodies and wicks separately
- **Dynamic timeframe adjustment**
- **Volume overlay support** (planned)

### XAxisRenderer & YAxisRenderer (`drawables/x_axis.rs`, `y_axis.rs`)
Axis label and grid rendering:
- **Dynamic label generation** based on viewport
- **Automatic formatting** (time, scientific notation)
- **GPU-accelerated text rendering** via wgpu_text
- **Responsive to zoom/pan operations**

## Chart Types

### Line Chart (`charts/line.rs`)
- Standard time-series visualization
- Anti-aliased line rendering
- Multiple series support

### Area Chart (`charts/area.rs`)
- Filled area under curve
- Gradient fill support
- Transparency handling

### Bar Chart (`charts/bar.rs`)
- Vertical/horizontal bars
- Grouped/stacked variants
- Dynamic width calculation

### Candlestick Chart (`charts/candlestick.rs`)
- OHLC data visualization
- Volume integration
- Multiple timeframe support

### Triangle Renderer (`charts/triangle_renderer.rs`)
- Trade execution markers
- Buy/sell differentiation
- Fixed screen-space size
- Instanced rendering for efficiency

## Performance Optimizations

### GPU Memory Management
- **Buffer Pooling**: Reuse allocated buffers across frames
- **Shared Buffers**: Single time buffer for multiple metrics
- **GPU-Resident Data**: Minimize CPU-GPU transfers
- **Memory-Mapped Buffers**: Direct data access patterns

### Parallel Computation Strategies
- **Workgroup Optimization**: 256 threads for compute, 64 for specialized tasks
- **Thread Multipliers**: Each thread processes multiple elements
- **Shared Memory Usage**: Reduce global memory access
- **Coalesced Memory Access**: Sequential access patterns

### Rendering Optimizations
- **Instanced Rendering**: For repeated geometry (triangles, candles)
- **Early Z-Testing**: Depth-based culling
- **Viewport Culling**: Skip off-screen elements
- **LOD System**: Reduced detail at low zoom levels
- **Batch Rendering**: Minimize draw calls

### Compute Shader Optimizations
- **Binary Search**: For sorted data lookups
- **Parallel Reduction**: Tree-based aggregation
- **Early Exit**: Skip unnecessary computation
- **Sentinel Values**: Efficient uninitialized data handling
- **Cache-Friendly Access**: Optimize for GPU cache lines

## Multi-Renderer Usage

### Basic Setup
```rust
let multi_renderer = MultiRendererBuilder::new(device, queue, format)
    .with_render_order(RenderOrder::Priority)
    .add_candlestick_renderer()
    .add_plot_renderer()
    .add_x_axis_renderer(width, height)
    .add_y_axis_renderer(width, height)
    .build();
```

### Custom Renderer Integration
```rust
impl MultiRenderable for CustomRenderer {
    fn render(&mut self, encoder, view, data_store, device, queue) {
        // Custom rendering logic
    }
    
    fn priority(&self) -> u32 {
        75  // Render between background (50) and foreground (100)
    }
    
    fn has_compute(&self) -> bool {
        true  // Enable compute pass
    }
    
    fn compute(&mut self, encoder, data_store, device, queue) {
        // Pre-render compute operations
    }
}
```

## Dependencies and Integration

### Internal Dependencies
- `shared-types`: Common types and error definitions
- `config-system`: Quality presets and configuration
- `data-manager`: Data store and GPU buffer management

### External Dependencies
- `wgpu 24.0.5`: WebGPU implementation
- `bytemuck`: Zero-copy buffer casting
- `wgpu_text`: GPU text rendering
- `nalgebra-glm`: Matrix operations
- `js-sys/web-sys`: WASM browser integration

## WebGPU Best Practices Implemented

1. **Buffer Usage Flags**: Minimal, specific usage flags for optimization
2. **Bind Group Caching**: Reuse bind groups across frames
3. **Pipeline State Objects**: Pre-compiled, cached pipelines
4. **Async Pipeline Compilation**: Non-blocking shader compilation
5. **Proper Synchronization**: Barriers and fences where needed
6. **Error Recovery**: Graceful handling of device loss
7. **Format Compatibility**: Automatic format detection and adaptation

## Performance Profiling Points

### GPU Metrics to Monitor
- **GPU Time**: Via timestamp queries
- **Memory Usage**: Buffer and texture allocation
- **Draw Call Count**: Minimize for efficiency
- **Vertex Count**: Track per-frame vertices
- **Compute Dispatch Count**: Optimize workgroup sizes

### Optimization Targets
- < 16ms frame time (60 FPS)
- < 100MB GPU memory for 1M data points
- < 10 draw calls per frame
- < 5ms compute shader execution
- Zero CPU-GPU sync stalls

## Future Enhancements

### Planned Features
- **Adaptive Quality**: Dynamic LOD based on performance
- **GPU-Based Culling**: Frustum and occlusion culling
- **Texture Atlasing**: For improved text rendering
- **Indirect Drawing**: GPU-driven render commands
- **Mesh Shaders**: Next-gen geometry pipeline
- **Ray Tracing**: Advanced visualization effects

### Optimization Opportunities
- **Persistent Mapped Buffers**: Reduce allocation overhead
- **Multi-Queue Submission**: Parallel command execution
- **Subgroup Operations**: Utilize GPU wave/warp intrinsics
- **Variable Rate Shading**: Adaptive quality per region
- **GPU Timeline Profiling**: Detailed performance analysis
- **Shader Hot-Reload**: Development productivity