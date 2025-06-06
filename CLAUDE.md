# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a WebAssembly-based real-time data visualization application built in Rust that renders interactive line graphs using WebGPU for high-performance GPU-accelerated rendering. The application has both a standalone WASM module and a React web frontend for development.

## Development Commands

### Building and Development
```bash
# Build WASM module for development (generates files in web/pkg)
cd web && npm run dev:wasm

# Watch Rust files and auto-rebuild WASM with hot reload
cd web && npm run dev:watch

# Build and run the data server (port 8443)
cd web && npm run dev:server

# Full development server (WASM watch + React dev server)
cd web && npm run dev:full

# Complete development stack (WASM + server + React)
cd web && npm run dev:complete

# Set up SSL certificates for local development
cd web && npm run setup:ssl

# Production build
cd web && npm run build

# Build server for production
cd web && npm run build:server

# Lint TypeScript/React code
cd web && npm run lint
```

### Testing
```bash
# Run server unit and integration tests
cd web && npm run test:server

# Run server API integration tests (requires running server)
cd web && npm run test:server:api

# Run React/frontend tests
cd web && npm run test

# Run specific frontend test suites
cd web && npm run test:data
cd web && npm run test:basic
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

## Multi-Component Architecture

This project consists of three main components working together:

### 1. React Frontend Mode (Primary)
- **Frontend**: Modern React app with TypeScript, Tailwind CSS, and Vite
- **WASM Output**: `web/pkg/` for React integration
- **Development**: Hot reloading via `scripts/dev-build.sh` watching Rust changes
- **State Management**: Zustand store in `web/src/store/`
- **Components**: React components in `web/src/components/` with chart integration
- **Data Source**: Connects to local data server via HTTPS API

### 2. Data Server (Production Ready)
- **Backend**: High-performance Rust server with HTTP/2 and TLS
- **Purpose**: Serves financial time-series data with ultra-low latency
- **Location**: `/server` directory
- **API**: RESTful endpoints for data and symbol queries
- **Storage**: Memory-mapped binary files for zero-copy data access
- **Testing**: Comprehensive test suite with 26 total tests (18 unit + 8 integration)
- **Development**: Must use `--target x86_64-unknown-linux-gnu` for all cargo operations

### 3. Legacy File Server (Development Only)
- **File Server**: Simple Actix-web server in `file_server/`
- **Direct WASM**: Traditional web integration without React framework
- **Legacy Support**: Maintains original URL parameter-based interface

## File Structure Notes
- `web/pkg/`: Generated WASM modules for React integration
- `server/`: High-performance data server with SSL certificates
- `file_server/`: Simple Actix-web development server (legacy mode)
- `scripts/`: Build and development automation scripts
  - `dev-build.sh`: Automated WASM rebuilding with file watching
  - `setup-ssl.sh`: SSL certificate generation and management
- WGSL shaders co-located with respective Rust components
- Font files in `src/drawables/` for text rendering
- React bridge code in `src/lib_react.rs` and `src/react_bridge.rs`