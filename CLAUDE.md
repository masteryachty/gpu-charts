# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a WebAssembly-based real-time data visualization application built in Rust that renders interactive line graphs using WebGPU for high-performance GPU-accelerated rendering. The application has both a standalone WASM module and a React web frontend for development.

## Development Commands

### Primary Development Workflow (from project root)
```bash
# Build WASM module for development (generates files in web/pkg)
npm run dev:wasm

# Watch Rust files and auto-rebuild WASM with hot reload
npm run dev:watch

# Build and run the data server (port 8443)
npm run dev:server

# Full development server (WASM watch + React dev server)
npm run dev:full

# Complete development stack (WASM + server + React)
npm run dev:complete

# Complete development stack with data logger
npm run dev:complete:full

# Set up SSL certificates for local development
npm run setup:ssl

# Production build (WASM + React)
npm run build

# Build all components for production
npm run build:server
npm run build:logger

# Lint TypeScript/React code
npm run lint

# Clean all build artifacts
npm run clean
```

### Testing (from project root)
```bash
# Run all tests (server + web frontend)
npm run test

# Run server unit and integration tests
npm run test:server

# Run server API integration tests (requires running server)
npm run test:server:api

# Run coinbase logger tests
npm run test:logger

# Run React/frontend tests
npm run test:web

# Run specific frontend test suites
npm run test:data
npm run test:basic
```

### Alternative File Server (Legacy)
```bash
# Build and run simple file server (serves on port 8080)
cd file_server && cargo build && cargo run
```

### Testing the Application
- **React dev server**: `http://localhost:3000/app?topic=BTC-usd&start=1745322750&end=1745691150`
- **Data server API**: `https://localhost:8443/api/` (requires SSL certificates)
  - `/api/data` - Time-series data endpoint
  - `/api/symbols` - Available symbols endpoint
- **Legacy file server**: `http://localhost:8080` with query parameters:
  - `topic`: data source identifier
  - `start`: start timestamp  
  - `end`: end timestamp

Example data server request: `https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid`

Example legacy server: `http://localhost:8080?topic=sensor_data&start=1234567890&end=1234567900`

## Server Architecture

### Data Server (`/server`)
A high-performance HTTP/2 TLS server built for ultra-low latency financial data serving:

- **Technology**: Rust with `hyper`, `tokio-rustls`, and `memmap2`
- **Port**: 8443 (HTTPS only)
- **Data Storage**: Memory-mapped binary files for zero-copy serving
- **File Format**: `{column}.{DD}.{MM}.{YY}.bin` (e.g., `best_bid.01.03.25.bin`)
- **Path Structure**: `/mnt/md/data/{symbol}/{type}/{column}.{DD}.{MM}.{YY}.bin`

#### Endpoints
- **`GET /api/data`**: Serves time-series data
  - Query params: `symbol`, `type`, `start`, `end`, `columns`
  - Returns: JSON header + binary data stream
  - Columns: `time`, `best_bid`, `best_ask`, `price`, `volume`, `side` (4-byte records each)
- **`GET /api/symbols`**: Lists available trading symbols

#### Features
- Memory-mapped file I/O for zero-copy data access
- Multi-day query support with automatic date range handling
- TLS encryption with local SSL certificates
- Memory locking (`mlock`) for ultra-low latency
- CORS enabled for web frontend integration
- Comprehensive test coverage with unit and integration tests

#### Testing Infrastructure
The server includes extensive testing capabilities:

- **Unit Tests** (`server/tests/unit_tests.rs`): 18 tests covering:
  - Query parameter parsing and validation
  - Data indexing and binary search algorithms
  - File I/O and memory-mapped file operations
  - Edge cases and error handling
  
- **Integration Tests** (`server/tests/data_tests.rs`): 8 tests covering:
  - End-to-end API request handling
  - Mock data generation and validation
  - Multi-column data serving scenarios
  
- **API Tests** (`server/test_api.sh`): Bash script testing:
  - Live server endpoints (`/api/data`, `/api/symbols`)
  - Error handling and HTTP status codes
  - CORS headers and OPTIONS preflight requests
  - SSL/TLS connectivity

**Running Tests**: All tests must be run with the native target to avoid WASM compilation issues:
```bash
# From project root
cargo test --target x86_64-unknown-linux-gnu

# Or using npm scripts from web directory
npm run test:server          # Unit and integration tests
npm run test:server:api      # Live API tests (requires running server)
```

## Architecture Overview

### Core Components (Charting Library)
- **LineGraph** (`charting/src/line_graph.rs`): Main orchestrator that manages data fetching, rendering, and user interactions
- **RenderEngine** (`charting/src/renderer/render_engine.rs`): WebGPU rendering system with surface management
- **DataStore** (`charting/src/renderer/data_store.rs`): Manages time-series data buffers and screen transformations
- **DataRetriever** (`charting/src/renderer/data_retriever.rs`): HTTP-based data fetching from external APIs

### Rendering Pipeline
The application uses separate render passes for different components:
- **PlotRenderer** (`charting/src/drawables/plot.rs`): Main data line visualization
- **XAxisRenderer** (`charting/src/drawables/x_axis.rs`): Time-based X-axis with labels
- **YAxisRenderer** (`charting/src/drawables/y_axis.rs`): Value-based Y-axis with labels

Each renderer has corresponding WGSL compute/vertex/fragment shaders.

### GPU Compute
- **MinMax** (`charting/src/calcables/min_max.rs`): Uses compute shaders to efficiently calculate dataset bounds on GPU
- All shaders located in respective component directories as `.wgsl` files

### User Interaction
- **CanvasController** (`charting/src/controls/canvas_controller.rs`): Handles mouse wheel zoom, cursor panning, and triggers data refetching for new time ranges

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

## Multi-Component Architecture

This project consists of four main components working together:

### 1. Charting Library (`/charting`)
- **Core Engine**: WebAssembly-based charting library built in Rust
- **Technology**: WebGPU for GPU-accelerated rendering, WASM for web integration
- **Output**: Built to `web/pkg/` for React consumption
- **Features**: Real-time data visualization, interactive controls, high-performance rendering
- **Development**: Hot reloading via `scripts/dev-build.sh` watching Rust changes

### 2. React Frontend (`/web`)
- **Frontend**: Modern React app with TypeScript, Tailwind CSS, and Vite
- **Integration**: Consumes WASM charting library from `web/pkg/`
- **State Management**: Zustand store in `web/src/store/`
- **Components**: React components in `web/src/components/` with chart integration
- **Data Source**: Connects to local data server via HTTPS API

### 3. Data Server (`/server`)
- **Backend**: High-performance Rust server with HTTP/2 and TLS
- **Purpose**: Serves financial time-series data with ultra-low latency
- **API**: RESTful endpoints for data and symbol queries
- **Storage**: Memory-mapped binary files for zero-copy data access
- **Testing**: Comprehensive test suite with 26 total tests (18 unit + 8 integration)
- **Development**: Must use `--target x86_64-unknown-linux-gnu` for all cargo operations

### 4. Coinbase Logger (`/coinbase-logger`)
- **Purpose**: Real-time market data collection from Coinbase WebSocket feed
- **Output**: Writes binary data files that the server memory-maps
- **Technology**: Multi-threaded Rust application with WebSocket connections
- **Integration**: Feeds data directly to server for live visualization

### 5. Legacy File Server (`/file_server`)
- **File Server**: Simple Actix-web server (development only)
- **Direct WASM**: Traditional web integration without React framework
- **Legacy Support**: Maintains original URL parameter-based interface

## File Structure Notes
- `charting/`: Core WebAssembly charting library (moved from root `src/`)
  - WGSL shaders co-located with respective Rust components
  - Font files in `charting/src/drawables/` for text rendering
  - React bridge code in `charting/src/lib_react.rs` and `charting/src/react_bridge.rs`
- `web/`: React frontend application
  - `web/pkg/`: Generated WASM modules from charting library
- `server/`: High-performance data server with SSL certificates
- `coinbase-logger/`: Real-time market data collection service
- `file_server/`: Simple Actix-web development server (legacy mode)
- `scripts/`: Build and development automation scripts
  - `dev-build.sh`: Automated WASM rebuilding with file watching (updated paths)
  - `setup-ssl.sh`: SSL certificate generation and management
- `package.json`: Top-level orchestration scripts for all components
- `Cargo.toml`: Workspace configuration for all Rust components