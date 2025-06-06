# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a WebAssembly-based real-time data visualization application built in Rust that renders interactive line graphs using WebGPU for high-performance GPU-accelerated rendering. The application has both a standalone WASM module and a React web frontend for development.

## Development Commands

### Building and Development
```bash
# Build WASM module for development (generates files in web/public/pkg)
cd web && npm run dev:wasm

# Watch Rust files and auto-rebuild WASM with hot reload
cd web && npm run dev:watch

# Full development server (WASM watch + React dev server)
cd web && npm run dev:full

# Production build
cd web && npm run build

# Lint TypeScript/React code
cd web && npm run lint
```

### Alternative File Server (Legacy)
```bash
# Build and run simple file server (serves on port 8080)
cd file_server && cargo build && cargo run
```

### Testing the Application
- React dev server: `http://localhost:3000/app?topic=BTC-usd&start=1745322750&end=1745691150`
- File server: `http://localhost:8080` with query parameters:
  - `topic`: data source identifier
  - `start`: start timestamp  
  - `end`: end timestamp

Example: `http://localhost:8080?topic=sensor_data&start=1234567890&end=1234567900`

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

## Key Technical Considerations

### WebAssembly Integration
- Built as both `cdylib` (for WASM) and `rlib` (for testing)
- Uses `wasm-bindgen` for JavaScript interop
- Async operations handled via `wasm-bindgen-futures`
- Memory management follows Rust ownership patterns

### Data Flow
1. URL parameters determine initial dataset (topic, time range)
2. DataRetriever fetches data via HTTP requests
3. GPU compute shaders calculate min/max bounds
4. Separate render passes draw plot lines, axes, and labels
5. User interactions trigger new data fetches and re-rendering

### Performance Optimizations
- GPU-accelerated calculations using WebGPU compute shaders
- Efficient buffer management for large time-series datasets
- Asynchronous data loading and rendering pipeline

## Dual Architecture

This project has two operational modes:

### 1. React Integration Mode
- **Frontend**: Modern React app with TypeScript, Tailwind CSS, and Vite
- **WASM Output**: `web/public/pkg/` for React integration
- **Development**: Hot reloading via `scripts/dev-build.sh` watching Rust changes
- **State Management**: Zustand store in `web/src/store/`
- **Components**: React components in `web/src/components/` with chart integration

### 2. Standalone WASM Mode  
- **File Server**: Simple Actix-web server in `file_server/`
- **Direct WASM**: Traditional web integration without React framework
- **Legacy Support**: Maintains original URL parameter-based interface

## File Structure Notes
- `web/public/pkg/`: Generated WASM modules for React integration
- `file_server/`: Simple Actix-web development server (legacy mode)
- `scripts/dev-build.sh`: Automated WASM rebuilding with file watching
- WGSL shaders co-located with respective Rust components
- Font files in `src/drawables/` for text rendering
- React bridge code in `src/lib_react.rs` and `src/react_bridge.rs`