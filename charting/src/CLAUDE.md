# Src Directory - CLAUDE.md

This file provides specific guidance for working with the WebAssembly core rendering engine and GPU-accelerated visualization components of the graph application.

## Overview

The src directory contains a sophisticated WebAssembly-based real-time data visualization engine built in Rust. It features WebGPU-accelerated rendering, GPU compute shaders for data processing, and multiple integration modes for both standalone WASM and React frontend usage.

## Architecture Overview

### Core Design Philosophy
- **WebGPU-First**: GPU-accelerated rendering and compute operations
- **Modular Design**: Clear separation of concerns across specialized modules
- **Zero-Copy Performance**: Efficient memory management and data pipelines
- **WebAssembly Integration**: Seamless JavaScript interop with multiple bridge patterns
- **Real-time Visualization**: Optimized for high-frequency financial data rendering

### Module Organization
```
src/
├── lib.rs              # Core library exports and WASM bindings
├── lib_react.rs        # React-specific bridge with Chart class
├── react_bridge.rs     # Simple WASM bridge for minimal integration
├── line_graph.rs       # Main orchestrator and application logic
├── main.rs             # Standalone executable entry point
├── calcables/          # GPU compute operations (min/max calculations)
├── controls/           # User interaction handling (zoom, pan, mouse)
├── drawables/          # Rendering components (plot, axes, text)
├── renderer/           # Core rendering engine and data management
└── wrappers/           # JavaScript interop utilities
```

## Development Commands

### Build Commands
```bash
# Development WASM build (from web/ directory)
npm run dev:wasm

# Watch mode with auto-rebuild
npm run dev:watch

# Production WASM build
npm run build:wasm

# Native development build (for testing)
cargo build --target x86_64-unknown-linux-gnu

# WASM-specific build
wasm-pack build --target web --out-dir web/pkg
```

### Testing
```bash
# Native tests only (WASM doesn't support all test features)
cargo test --target x86_64-unknown-linux-gnu

# Individual module testing
cargo test --target x86_64-unknown-linux-gnu renderer
cargo test --target x86_64-unknown-linux-gnu calcables
```

**Critical**: All testing must use native target. WASM target doesn't support full test infrastructure.

## WebAssembly Integration Architecture

### Dual Integration Modes

#### 1. React Bridge (`lib_react.rs`)
Advanced integration for React applications with full lifecycle management:

```rust
#[wasm_bindgen]
pub struct Chart {
    line_graph: Option<LineGraph>,
    canvas_id: String,
}

#[wasm_bindgen]
impl Chart {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Chart { /* ... */ }
    
    #[wasm_bindgen]
    pub async fn init(&mut self, canvas_id: &str) -> Result<(), JsValue> { /* ... */ }
    
    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&mut self, delta: f64, x: f64, y: f64) { /* ... */ }
    
    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool { /* ... */ }
}
```

**Key Features:**
- Async initialization with error handling
- Mouse event bridging (wheel, move, click)
- Canvas size management and responsiveness
- Direct integration with React component lifecycle

#### 2. Simple Bridge (`react_bridge.rs`)
Minimal wrapper for basic WASM integration:
```rust
#[wasm_bindgen]
pub fn run() {
    // Simple winit-based initialization
    // Suitable for standalone or minimal integration
}
```

### WebAssembly Build Configuration
```toml
[lib]
crate-type = ["cdylib", "rlib"]  # WASM + native testing

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"    # Async support
web-sys = "0.3"                 # Web API bindings
js-sys = "0.3"                  # JavaScript type bindings
```

## WebGPU Rendering Engine (`renderer/`)

### Core Engine Architecture (`render_engine.rs`)

#### WebGPU Initialization
```rust
pub struct RenderEngine {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: Surface,
    config: SurfaceConfiguration,
    render_listeners: Vec<Box<dyn RenderListener>>,
}
```

**Initialization Flow:**
1. **Instance Creation**: WebGPU instance with BROWSER_WEBGPU backend
2. **Adapter Request**: Compatible hardware adapter selection
3. **Device Creation**: GPU device with required features and limits
4. **Surface Configuration**: Canvas surface with optimal present mode
5. **Render Pipeline**: Multi-pass rendering system setup

#### Render Listener Pattern
Observer pattern enabling modular rendering components:
```rust
pub trait RenderListener {
    fn on_render(&mut self, encoder: &mut CommandEncoder, view: &TextureView, device: &Device, queue: &Queue);
    fn on_resize(&mut self, width: u32, height: u32, device: &Device, queue: &Queue);
}
```

**Registered Listeners:**
- `PlotRenderer`: Main data line visualization
- `XAxisRenderer`: Time-based X-axis with intelligent label spacing
- `YAxisRenderer`: Value-based Y-axis with automatic scaling

### Data Management (`data_store.rs`)

#### Multi-Series Data Architecture
```rust
pub struct DataStore {
    data_groups: Vec<DataGroup>,
    world_bounds: Option<WorldBounds>,
    screen_bounds: ScreenBounds,
    margin_percent: f32,  // Default 10% Y-axis margin
}

pub struct DataGroup {
    x_data_chunks: Vec<Buffer>,  // GPU buffers for X data (u32 timestamps)
    y_data_chunks: Vec<Buffer>,  // GPU buffers for Y data (f32 values)
    vertex_count_per_chunk: Vec<u32>,
    color: [f32; 3],
}
```

**Key Features:**
- **Chunked GPU Buffers**: Large datasets split into manageable 128MB chunks
- **Coordinate Transformations**: World-to-screen and screen-to-world conversions
- **Automatic Margins**: 10% Y-axis padding for visual clarity
- **Multi-Series Support**: Multiple data groups with independent coloring

#### Coordinate System
```rust
// World coordinates (data space)
pub struct WorldBounds {
    min_x: f64,  // Earliest timestamp
    max_x: f64,  // Latest timestamp
    min_y: f64,  // Minimum value
    max_y: f64,  // Maximum value
}

// Screen coordinates (pixel space)
pub struct ScreenBounds {
    width: f32,
    height: f32,
}
```

### Data Retrieval (`data_retriever.rs`)

#### HTTP/2 Binary Protocol
```rust
pub struct DataRetriever {
    base_url: String,  // HTTPS data server endpoint
}

impl DataRetriever {
    pub async fn fetch_data(&self, symbol: &str, start: u64, end: u64, columns: &[&str]) -> Result<ParsedData, JsValue> {
        // 1. HTTP/2 request to data server
        // 2. Parse JSON header + binary data stream
        // 3. Convert to GPU-ready format
        // 4. Create chunked buffers for large datasets
    }
}
```

**Data Flow:**
1. **HTTPS Request**: Fetch data from high-performance server
2. **Protocol Parsing**: JSON header + binary data stream
3. **GPU Buffer Creation**: Direct upload to GPU memory
4. **Chunk Management**: Automatic splitting for memory efficiency

## GPU Compute Architecture (`calcables/`)

### Min/Max Calculation System (`min_max.rs`)

#### Two-Pass Reduction Algorithm
Highly optimized GPU compute pipeline for dataset bounds calculation:

```rust
pub struct MinMax {
    first_pass_pipeline: ComputePipeline,
    subsequent_pass_pipeline: ComputePipeline,
    staging_buffer: Buffer,
}
```

#### First Pass Shader (`min_max_first.wgsl`)
```wgsl
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Vectorized processing: 4 elements per thread
    // Local memory reduction using workgroup shared memory
    // Tree reduction pattern for optimal parallelism
}
```

**Key Features:**
- **Workgroup Size**: 256 threads per workgroup (optimal for most GPUs)
- **Vectorized Processing**: SIMD-style operations processing 4 elements simultaneously
- **Shared Memory**: Workgroup-local reduction using shared memory
- **Configurable Chunking**: Default 256×32 = 8,192 elements per workgroup

#### Subsequent Pass Shader (`min_max_second.wgsl`)
```wgsl
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Multi-level reduction of partial results
    // Continues until single min/max pair remains
    // Memory-efficient staging buffer usage
}
```

#### Binary Search Optimization
```rust
// JavaScript-interop binary search for large ArrayBuffers
pub fn binary_search_start_index(data: &[u8], target: u32) -> usize {
    // Optimized for 4-byte timestamp arrays
    // O(log n) time complexity
    // Handles edge cases and boundary conditions
}
```

## Rendering Pipeline Components (`drawables/`)

### Plot Renderer (`plot.rs`)

#### Vertex Shader (`plot.wgsl`)
```wgsl
struct VertexInput {
    @location(0) x_data: u32,    // Timestamp (u32)
    @location(1) y_data: f32,    // Value (f32)
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    // World-to-screen coordinate transformation
    // Automatic margin calculation (10% Y-axis padding)
    // Orthographic projection to clip space
}
```

**Rendering Features:**
- **Line Strip Topology**: Efficient line rendering with `PrimitiveTopology::LineStrip`
- **Dynamic Vertex Buffers**: Multiple chunked buffers per data series
- **Automatic Margins**: Built-in 10% Y-axis padding for visual clarity
- **Multi-Series Support**: Different colors per data group

### Axis Renderers

#### X-Axis Renderer (`x_axis.rs`)
Time-based axis with intelligent label spacing:

```rust
impl XAxisRenderer {
    fn calculate_time_intervals(&self, start_time: u64, end_time: u64, width: f32) -> Vec<TimeLabel> {
        // Adaptive time interval selection
        // Supports: seconds, minutes, hours, days
        // Optimal label density based on zoom level
    }
}
```

**Features:**
- **Adaptive Time Intervals**: Automatic selection of appropriate time units
- **Multi-line Labels**: Date and time on separate lines
- **Performance Optimization**: Only recalculates when data range changes
- **Font Rendering**: Uses `wgpu_text` with embedded Roboto font

#### Y-Axis Renderer (`y_axis.rs`)
Value-based axis with automatic scaling:

```rust
impl YAxisRenderer {
    fn calculate_nice_bounds(&self, min_val: f64, max_val: f64) -> (f64, f64, f64) {
        // "Nice numbers" algorithm for clean intervals
        // Returns: (start_value, end_value, step_size)
        // Handles various magnitude ranges appropriately
    }
}
```

**Features:**
- **Nice Numbers Algorithm**: Clean, human-readable interval selection
- **Scientific Notation**: Appropriate formatting for different value ranges
- **Dynamic Range**: Optimal start/end values based on data bounds
- **Margin Integration**: Works with DataStore's 10% margin system

### Shader Architecture
All shaders implement consistent patterns:
- **Vertex Input**: Structured data from GPU buffers
- **Uniform Buffers**: Transformation matrices and viewport parameters
- **Fragment Output**: Standard color output to framebuffer

## User Interaction System (`controls/`)

### Canvas Controller (`canvas_controller.rs`)

#### Mouse Event Handling
```rust
pub struct CanvasController {
    data_retriever: DataRetriever,
    zoom_factor: f64,
    drag_start: Option<(f64, f64)>,
}

impl CanvasController {
    pub async fn handle_mouse_wheel(&mut self, delta: f64, mouse_x: f64, mouse_y: f64) {
        // 1. Calculate zoom factor and new time range
        // 2. Trigger async data fetching for expanded range
        // 3. Update DataStore with new data
        // 4. Trigger re-render with new bounds
    }
    
    pub fn handle_mouse_drag(&mut self, start_x: f64, start_y: f64, end_x: f64, end_y: f64) {
        // 1. Convert screen coordinates to data coordinates
        // 2. Create zoom rectangle for precise range selection
        // 3. Trigger data fetching for selected range
    }
}
```

**Interaction Patterns:**
- **Zoom Operations**: Mouse wheel triggers data fetching for expanded ranges
- **Pan/Drag**: Click-and-drag creates zoom rectangles
- **Coordinate Mapping**: Screen coordinates converted to data coordinates
- **Async Loading**: Non-blocking data fetching during interactions

## Integration Patterns

### React Integration Flow
1. **Component Mount**: React component calls `Chart.new()` and `Chart.init()`
2. **Canvas Setup**: WebGPU surface attached to HTML canvas element
3. **Event Bridging**: React mouse events converted to Rust handler calls
4. **State Sync**: Chart state synchronized with React component state
5. **Cleanup**: Proper resource cleanup on component unmount

### Data Flow Pipeline
1. **URL Parameters**: Initial dataset determined by query parameters
2. **HTTP Fetching**: DataRetriever fetches data from server via HTTPS
3. **GPU Upload**: Binary data uploaded directly to GPU buffers
4. **Compute Shaders**: Min/max bounds calculated on GPU
5. **Render Pipeline**: Multi-pass rendering (plot lines, axes, text)
6. **User Interaction**: Mouse events trigger new data fetches and re-rendering

## Performance Optimizations

### Memory Management
- **Zero-Copy Operations**: Direct GPU buffer uploads from binary data
- **Chunked Buffers**: Large datasets split into manageable chunks
- **Staging Buffers**: Separate buffers for CPU-GPU data transfer
- **Resource Cleanup**: Proper WebGPU resource lifecycle management

### Rendering Optimizations
- **Viewport Culling**: Only render data within visible range
- **Dirty State Tracking**: Axis renderers cache calculations
- **GPU Compute**: Min/max calculations run entirely on GPU
- **Batch Operations**: Multiple render passes batched efficiently

### WebAssembly Optimizations
- **Async Architecture**: Heavy operations use `wasm-bindgen-futures`
- **Memory Safety**: Rust ownership system prevents memory leaks
- **Minimal Boundaries**: Efficient JavaScript-Rust data marshaling
- **Feature Flags**: Conditional compilation for different integration modes

## Common Development Tasks

### Adding New Data Columns
1. Update DataRetriever to handle new column types
2. Modify DataStore to support new data formats
3. Update plot shader if needed for new data types
4. Add client-side parsing for new binary formats

### Implementing New Visualizations
1. Create new drawable component implementing `RenderListener`
2. Add corresponding WGSL shaders for vertex/fragment processing
3. Register component with RenderEngine listener system
4. Implement resize and render event handling

### Debugging WebGPU Issues
```rust
// Enable WebGPU validation (development only)
let instance = Instance::new(InstanceDescriptor {
    backends: Backends::BROWSER_WEBGPU,
    dx12_shader_compiler: Dx12Compiler::default(),
    flags: InstanceFlags::DEBUG | InstanceFlags::VALIDATION,
});
```

### Performance Profiling
- Use browser DevTools for WebGPU command inspection
- Monitor memory usage with `performance.measureUserAgentSpecificMemory()`
- Profile JavaScript-Rust boundary crossings
- Use GPU timing queries for render pass optimization

## Build Configuration

### Cargo.toml Dependencies
```toml
[dependencies]
# WebGPU and rendering
wgpu = "0.20.1"
wgpu-text = "0.10.2"

# WebAssembly integration
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = "0.3"
js-sys = "0.3"

# Data processing
bytemuck = { version = "1.12", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde-wasm-bindgen = "0.4"

# Async and utilities
futures = "0.3"
console_error_panic_hook = "0.1.6"
console_log = "1.0"
```

### Feature Flags
```toml
[features]
default = []
react-mode = ["wasm-bindgen", "web-sys"]
```

### WASM-Pack Configuration
```bash
# Target web for ES module output
wasm-pack build --target web --out-dir web/pkg

# Development with debug info
wasm-pack build --dev --target web --out-dir web/pkg

# Production optimization
wasm-pack build --release --target web --out-dir web/pkg
```

## Debugging and Development

### Console Logging
```rust
use console_log::init_with_level;
use log::Level;

// Initialize logging for WASM
init_with_level(Level::Debug).expect("error initializing log");
log::info!("WebGPU initialization complete");
```

### Error Handling Patterns
```rust
// Convert WebGPU errors to JavaScript errors
impl From<CreateSurfaceError> for JsValue {
    fn from(err: CreateSurfaceError) -> JsValue {
        JsValue::from_str(&format!("WebGPU surface creation failed: {:?}", err))
    }
}
```

### Common Issues
- **WebGPU Support**: Check browser compatibility and enable experimental features
- **Memory Limits**: Monitor WASM memory usage, especially with large datasets
- **Async Operations**: Ensure proper error handling in async chains
- **Resource Cleanup**: Prevent WebGPU resource leaks in React components

This WebAssembly core represents a sophisticated, high-performance rendering engine optimized for real-time financial data visualization with seamless web integration.