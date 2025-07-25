# GPU Charts Project Compact Snapshot

Generated: 2025-07-25

## Project Overview

**GPU Charts** is a WebAssembly-based real-time data visualization application built in Rust that renders interactive charts using WebGPU for high-performance GPU-accelerated rendering. The application uses a modular architecture with separate crates for different concerns and includes a React web frontend.

## Architecture Summary

### Core Components

1. **WASM Bridge and Core Libraries** (`/crates/`)
   - Modular WebAssembly-based charting system
   - WebGPU for GPU-accelerated rendering
   - 5 specialized crates with clear separation of concerns

2. **React Frontend** (`/web`)
   - Modern React app with TypeScript, Tailwind CSS, and Vite
   - Zustand state management
   - Consumes WASM module from `web/pkg/`

3. **Data Server** (`/server`)
   - High-performance Rust server with HTTP/2 and TLS
   - Memory-mapped binary files for zero-copy data access
   - Ultra-low latency financial time-series data serving

4. **Market Data Loggers**
   - **Coinbase Logger** (`/coinbase-logger`) - Real-time Coinbase WebSocket feed
   - **Multi-Exchange Logger** (`/logger`) - Unified logger for multiple exchanges

5. **Legacy File Server** (`/file_server`)
   - Simple Actix-web server for development

## Workspace Structure

```toml
[workspace]
resolver = "2"
members = [
    "server", 
    "coinbase-logger",
    "logger",
    "file_server",
    "crates/shared-types",
    "crates/config-system",
    "crates/data-manager",
    "crates/renderer",
    "crates/wasm-bridge"
]
```

## Crate Architecture

```
shared-types (foundation - no internal deps)
    ↑
├── config-system (depends on: shared-types)
├── data-manager (depends on: shared-types)
├── renderer (depends on: shared-types, config-system)
    ↑
└── wasm-bridge (depends on: all above crates)
    ↑
    JavaScript/React
```

### Crate Details

#### 1. `shared-types` - Common types and data structures
- Store state types for React integration
- Event system types
- Error definitions
- Zero dependencies on other workspace crates

#### 2. `config-system` - Configuration and quality presets
- Low/Medium/High/Ultra quality settings
- Performance tuning parameters
- Chart appearance configuration

#### 3. `data-manager` - Data operations
- HTTP data fetching with caching
- Binary data parsing
- GPU buffer creation and management
- Screen-space coordinate transformations

#### 4. `renderer` - Pure GPU rendering engine
- WebGPU pipeline management
- Specialized renderers (plot, candlestick, axes)
- WGSL shader management
- Surface and texture handling

#### 5. `wasm-bridge` - JavaScript/React integration
- Central orchestration layer
- JavaScript/React bindings
- Event handling and user interactions
- State synchronization

## Key Development Commands

```bash
# From project root
npm run dev:suite        # Full stack: WASM + Server + React
npm run dev:suite:full   # Full stack + data logger
npm run dev:watch        # Watch Rust files and auto-rebuild WASM
npm run build           # Production build (WASM + React)
npm run test            # Run server tests
npm run test:all        # Run all tests including web
npm run setup:ssl       # Set up SSL certificates
```

## API Endpoints

### Production
- Base URL: `https://api.rednax.io/api/`
- Endpoints: `/data`, `/symbols`

### Local Development
- Base URL: `https://localhost:8443/api/`
- Same endpoints as production

### Data API Example
```
/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid
```

## Testing Infrastructure

- **Pre-commit hooks** automatically run:
  - Rust formatting (`cargo fmt`)
  - Clippy linting
  - Security audit (`cargo audit`)
  - Dependency check (`cargo deny`)
  - Build verification
  - Full test suite

- **Test Commands**:
  ```bash
  npm run test:server      # Server unit and integration tests
  npm run test:server:api  # Live API tests
  npm run test:logger      # Logger tests
  npm run test:web         # Frontend tests
  ```

## File Structure

```
gpu-charts/
├── crates/              # Modular Rust crates
│   ├── shared-types/    # Common types and data structures
│   ├── config-system/   # Configuration and quality presets
│   ├── data-manager/    # Data fetching, parsing, and GPU buffers
│   ├── renderer/        # Pure GPU rendering engine
│   └── wasm-bridge/     # JavaScript/React integration layer
├── web/                 # React frontend application
│   ├── src/            # Source code
│   ├── tests/          # Test files
│   └── pkg/            # Generated WASM modules
├── server/             # High-performance data server
├── coinbase-logger/    # Coinbase market data collector
├── logger/             # Multi-exchange data logger
├── file_server/        # Legacy development server
├── scripts/            # Build and development scripts
├── package.json        # Top-level orchestration
└── Cargo.toml         # Workspace configuration
```

## Technology Stack

- **Backend**: Rust, WebAssembly, WebGPU
- **Frontend**: React, TypeScript, Tailwind CSS, Vite, Zustand
- **Server**: Hyper, Tokio, TLS, Memory-mapped I/O
- **Data**: Binary format, WebSocket feeds, HTTP/2
- **Testing**: Jest, Playwright, Cargo test
- **Build**: wasm-pack, npm scripts, cargo workspace

## Performance Features

- GPU-accelerated rendering via WebGPU
- Zero-copy data serving with memory-mapped files
- Efficient binary data format
- WebAssembly for near-native performance
- Compute shaders for data processing
- Multi-threaded data collection

## Development Guidelines

- Use Linux line endings (LF) for all files
- Dependencies only flow upward in the architecture
- Each crate should have comprehensive unit tests
- Pre-commit hooks ensure code quality
- Always use absolute paths in commands

## Quick Start

```bash
# Clone and setup
git clone <repo>
cd gpu-charts
npm install
npm run setup:ssl

# Start development
npm run dev:suite

# Make changes (auto-rebuild)
# - Rust changes in /crates/ auto-rebuild
# - React changes hot-reload

# Test and commit
npm run test:server
npm run test:web
git commit -m "feat: your feature"  # Pre-commit hooks run automatically
```