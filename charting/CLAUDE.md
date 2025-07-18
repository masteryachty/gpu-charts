# Charting Library - CLAUDE.md

This file provides specific guidance for working with the core WebAssembly charting library component.

## Overview

The charting directory contains the core WebAssembly-based charting library built in Rust that renders interactive line graphs using WebGPU for high-performance GPU-accelerated rendering. This library is designed to be embedded in web applications via WebAssembly and provides real-time data visualization capabilities.

## Development Commands

### Building the Library
```bash
# Development WASM build (from project root)
npm run dev:wasm

# Production WASM build (from project root)  
npm run build:wasm

# Direct build from charting directory
cd charting && wasm-pack build --target web --out-dir ../web/pkg --dev

# Production build from charting directory
cd charting && wasm-pack build --target web --out-dir ../web/pkg
```

### Development Workflow
```bash
# Watch for changes and auto-rebuild (from project root)
npm run dev:watch

# Full development pipeline (from project root)
npm run dev:suite
```

## Architecture Overview

### Core Components
- **LineGraph** (`src/line_graph.rs`): Main orchestrator that manages data fetching, rendering, and user interactions
- **RenderEngine** (`src/renderer/render_engine.rs`): WebGPU rendering system with surface management
- **DataStore** (`src/renderer/data_store.rs`): Manages time-series data buffers and screen transformations
- **DataRetriever** (`src/renderer/data_retriever.rs`): HTTP-based data fetching from external APIs

### Rendering Pipeline
The application uses separate render passes for different components:
- **PlotRenderer** (`src/drawables/plot.rs`): Main data line visualization
- **XAxisRenderer** (`src/drawables/x_axis.rs`): Time-based X-axis with labels
- **YAxisRenderer** (`src/drawables/y_axis.rs`): Value-based Y-axis with labels

Each renderer has corresponding WGSL compute/vertex/fragment shaders.

### GPU Compute
- **MinMax** (`src/calcables/min_max.rs`): Uses compute shaders to efficiently calculate dataset bounds on GPU
- All shaders located in respective component directories as `.wgsl` files

### User Interaction
- **CanvasController** (`src/controls/canvas_controller.rs`): Handles mouse wheel zoom, cursor panning, and triggers data refetching for new time ranges

## WebAssembly Integration

### Build Configuration
The library is configured to build for both WASM and native targets:

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "GPU-charting"
path = "src/main.rs"
```

### JavaScript Bridge
- **React Bridge** (`src/lib_react.rs`): React-specific WASM exports
- **Main Bridge** (`src/react_bridge.rs`): Core JavaScript interop functions
- **WASM Bindings**: Uses `wasm-bindgen` for JavaScript interop

### Memory Management
- Built with Rust ownership patterns for memory safety
- Uses `wasm-bindgen-futures` for async operations
- Efficient buffer management for large time-series datasets

## Data Flow Architecture

### Input Sources
1. URL parameters determine initial dataset (topic, time range)
2. DataRetriever fetches data via HTTP requests to local server
3. Real-time updates via WebSocket connections (future feature)

### Processing Pipeline
1. Raw data ingestion and validation
2. GPU compute shaders calculate min/max bounds
3. Data transformation for screen coordinates
4. Separate render passes for plot lines, axes, and labels
5. User interactions trigger new data fetches and re-rendering

### Output Integration
- WASM module exports for web integration
- Canvas-based rendering via WebGPU
- Event handling for mouse and keyboard interactions

## Performance Optimizations

### GPU Acceleration
- WebGPU compute shaders for mathematical operations
- Efficient vertex buffer management
- Parallel processing for large datasets

### Memory Efficiency
- Zero-copy data transfers where possible
- Efficient buffer reuse and pooling
- Streaming data processing for large time series

### Async Architecture
- Non-blocking data fetching
- Asynchronous rendering pipeline
- Concurrent processing of user interactions

## Development Guidelines

### Code Organization
- Each major component has its own module directory
- Shaders co-located with corresponding Rust code
- Clear separation between rendering, data, and control logic

### WebGPU Integration
- Modern GPU API for cross-platform compatibility
- Efficient resource management and cleanup
- Proper error handling for GPU operations

### Testing Considerations
- Library must be tested with native target: `cargo test --target x86_64-unknown-linux-gnu`
- WASM-specific testing requires browser environment
- Integration testing via web frontend

## Integration with Web Frontend

### WASM Package Output
- Built to `../web/pkg/` directory
- Package includes TypeScript definitions
- Optimized for web bundle sizes

### React Integration
- Custom React components consume WASM exports
- Event bridging between DOM and WASM
- State management integration with web store

### Build Pipeline
- Automated rebuilding on file changes
- Hot reload integration with Vite
- Production optimization for deployment

## File Structure
```
charting/
├── Cargo.toml              # Package configuration
├── Cargo.lock              # Dependency lock file
├── src/
│   ├── main.rs             # Native binary entry point
│   ├── lib.rs              # Library root and WASM exports
│   ├── lib_react.rs        # React-specific WASM bridge
│   ├── react_bridge.rs     # JavaScript interop functions
│   ├── line_graph.rs       # Main chart orchestrator
│   ├── calcables/          # GPU compute operations
│   │   ├── mod.rs
│   │   ├── min_max.rs      # Min/max calculation
│   │   ├── min_max_first.wgsl
│   │   └── min_max_second.wgsl
│   ├── controls/           # User interaction handling
│   │   ├── mod.rs
│   │   └── canvas_controller.rs
│   ├── drawables/          # Rendering components
│   │   ├── mod.rs
│   │   ├── plot.rs         # Main data visualization
│   │   ├── plot.wgsl
│   │   ├── x_axis.rs       # X-axis rendering
│   │   ├── x_axis.wgsl
│   │   ├── y_axis.rs       # Y-axis rendering
│   │   ├── y_axis.wgsl
│   │   └── Roboto.ttf      # Font for text rendering
│   ├── renderer/           # Core rendering engine
│   │   ├── mod.rs
│   │   ├── render_engine.rs    # WebGPU management
│   │   ├── data_store.rs       # Data buffer management
│   │   ├── data_retriever.rs   # HTTP data fetching
│   │   ├── mesh_builder.rs     # Geometry generation
│   │   ├── pipeline_builder.rs # GPU pipeline management
│   │   ├── web_socket.rs       # WebSocket communication
│   │   └── shaders/
│   │       └── shader.wgsl     # Common shader utilities
│   └── wrappers/           # Platform abstraction
│       ├── mod.rs
│       └── js.rs           # JavaScript platform layer
└── CLAUDE.md               # This documentation file
```

## Common Development Tasks

### Adding New Chart Types
1. Create new renderer in `src/drawables/`
2. Implement corresponding WGSL shaders
3. Add to rendering pipeline in `render_engine.rs`
4. Export new functionality via WASM bridge
5. Update TypeScript definitions for web integration

### Performance Optimization
1. Profile GPU operations using WebGPU debugging tools
2. Optimize shader code for target GPU architectures
3. Monitor memory usage in both WASM and GPU contexts
4. Benchmark data processing pipelines

### Adding New Data Sources
1. Extend `DataRetriever` with new connection types
2. Add data format parsing and validation
3. Update data transformation pipelines
4. Test with various data scenarios

### Debugging WASM Issues
```bash
# Build with debug symbols
wasm-pack build --target web --out-dir ../web/pkg --dev

# Use browser dev tools for WASM debugging
# Enable WebGPU validation layers
# Monitor memory usage and performance
```

This charting library serves as the core rendering engine for the entire visualization system, providing high-performance, GPU-accelerated data visualization capabilities that can be seamlessly integrated into modern web applications.