# GPU Charts - Comprehensive Project Documentation

This file provides comprehensive guidance for developers and AI assistants working with the GPU Charts codebase.

## Executive Summary

GPU Charts is a high-performance, real-time financial data visualization platform that leverages WebGPU for hardware-accelerated rendering. Built with a modular Rust/WebAssembly architecture, it delivers sub-millisecond latency charting capabilities for professional trading applications.

### Key Value Propositions
- **Ultra-Low Latency**: Sub-millisecond data-to-pixel rendering using GPU compute shaders
- **Massive Scale**: Visualize millions of data points without performance degradation
- **Professional Trading Features**: Order book visualization, candlestick charts, technical indicators
- **Real-Time Data**: Live market data ingestion from multiple exchanges
- **Production Ready**: Battle-tested with comprehensive testing and monitoring

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         External Data Sources                        │
│              (Coinbase, Kraken, Binance WebSocket Feeds)            │
└─────────────────────┬───────────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Market Data Collection Layer                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  Logger Service (Multi-Exchange WebSocket Connections)       │  │
│  │  - Real-time order book and trade data collection           │  │
│  │  - Binary file output (4-byte aligned records)              │  │
│  │  - Multi-threaded processing with 40+ concurrent streams    │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────┬───────────────────────────────────────────────┘
                      │ Binary Files
                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Data Serving Layer                              │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  High-Performance Data Server (HTTP/2 + TLS)                 │  │
│  │  - Memory-mapped file I/O for zero-copy serving             │  │
│  │  - Sub-millisecond query response times                     │  │
│  │  - RESTful API with binary streaming                        │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────┬───────────────────────────────────────────────┘
                      │ HTTPS API
                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    WebAssembly Rendering Engine                      │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │               Modular Crate Architecture                      │  │
│  │  ┌────────────────────────────────────────────────────────┐  │  │
│  │  │ shared-types: Foundation types and data structures     │  │  │
│  │  └────────────────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────────────────┐  │  │
│  │  │ config-system: Quality presets and performance tuning  │  │  │
│  │  └────────────────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────────────────┐  │  │
│  │  │ data-manager: Data fetching and GPU buffer management  │  │  │
│  │  └────────────────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────────────────┐  │  │
│  │  │ renderer: WebGPU pipelines and WGSL shader execution   │  │  │
│  │  └────────────────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────────────────┐  │  │
│  │  │ wasm-bridge: JavaScript bindings and orchestration     │  │  │
│  │  └────────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────┬───────────────────────────────────────────────┘
                      │ WASM Module
                      ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      React Frontend Application                      │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  - Modern React 18 with TypeScript and Vite                  │  │
│  │  - Zustand state management for real-time updates            │  │
│  │  - Professional trading UI components                        │  │
│  │  - WebGPU canvas integration                                 │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Data Flow Architecture

```
1. Market Data Collection
   WebSocket Feed → Logger → Binary Files
   - 40+ concurrent WebSocket connections
   - Multi-threaded data processing
   - 4-byte aligned binary records

2. Data Storage & Serving
   Binary Files → Memory Map → HTTP/2 Server
   - Zero-copy data access via mmap
   - Sub-millisecond query response
   - Multi-day range queries

3. Client Data Fetching
   React App → HTTPS API → Binary Stream
   - Efficient binary protocol
   - Streaming large datasets
   - Client-side caching

4. GPU Processing Pipeline
   Binary Data → GPU Buffers → Compute Shaders → Render Pipeline
   - Parallel data processing on GPU
   - Real-time min/max calculations
   - Hardware-accelerated rendering

5. User Interaction Loop
   Mouse/Keyboard → Event System → State Update → Re-render
   - <16ms interaction latency
   - Smooth 60 FPS updates
   - Responsive zoom/pan operations
```

## Component Documentation

Each component has its own detailed CLAUDE.md file:

### Core Rendering Engine
- [`crates/shared-types/CLAUDE.md`](/home/xander/projects/gpu-charts/crates/shared-types/CLAUDE.md) - Foundation types and data structures
- [`crates/config-system/CLAUDE.md`](/home/xander/projects/gpu-charts/crates/config-system/CLAUDE.md) - Configuration and quality presets
- [`crates/data-manager/CLAUDE.md`](/home/xander/projects/gpu-charts/crates/data-manager/CLAUDE.md) - Data operations and GPU buffers
- [`crates/renderer/CLAUDE.md`](/home/xander/projects/gpu-charts/crates/renderer/CLAUDE.md) - WebGPU rendering engine
- [`crates/wasm-bridge/CLAUDE.md`](/home/xander/projects/gpu-charts/crates/wasm-bridge/CLAUDE.md) - JavaScript/React integration

### Infrastructure Components
- [`logger/CLAUDE.md`](/home/xander/projects/gpu-charts/logger/CLAUDE.md) - Multi-exchange market data collection
- [`server/CLAUDE.md`](/home/xander/projects/gpu-charts/server/CLAUDE.md) - Ultra-low latency data server
- [`web/CLAUDE.md`](/home/xander/projects/gpu-charts/web/CLAUDE.md) - React frontend application

## Modular Crate Architecture

### Dependency Hierarchy
```
shared-types (foundation - zero dependencies)
    ↑
    ├── config-system (configuration management)
    ├── data-manager (data operations)
    └── renderer (GPU rendering)
            ↑
        wasm-bridge (orchestration layer)
            ↑
        JavaScript/React
```

### Architectural Principles
1. **Single Responsibility**: Each crate has one clear purpose
2. **Upward Dependencies**: Dependencies only flow up the hierarchy
3. **Interface Stability**: shared-types provides stable contracts
4. **Testability**: Crates can be tested in isolation
5. **Parallel Development**: Teams can work independently on crates

## Development Commands

### Quick Start
```bash
# Initial setup
git clone https://github.com/masteryachty/gpu-charts.git
cd gpu-charts
npm install
npm run setup:ssl  # Generate SSL certificates for local HTTPS

# Start full development stack
npm run dev:suite  # WASM + Server + React
```

### Development Workflows
```bash
# Core Development Commands
npm run dev                  # React dev server only
npm run dev:web             # WASM watch + React dev server
npm run dev:suite           # Full stack: WASM + Server + React
npm run dev:suite:full      # Full stack + Market data logger

# Component-Specific Development
npm run dev:wasm            # Build WASM module once
npm run dev:watch           # Watch and auto-rebuild WASM
npm run dev:server          # Run data server (port 8443)
npm run dev:logger          # Run market data logger

# Build Commands
npm run build               # Production build (WASM + React)
npm run build:wasm          # Build WASM module only
npm run build:server        # Build server binary
npm run build:logger        # Build logger binary

# Docker Deployment
npm run docker:build:server  # Build server Docker image
npm run docker:deploy:server # Deploy server container
npm run docker:logs:server   # View server logs
npm run docker:shell:server  # Access server shell
```

### Testing Commands
```bash
# Test Suites
npm run test                # Default tests (server only)
npm run test:all           # All tests (server + web)
npm run test:server        # Server unit and integration tests
npm run test:logger        # Logger tests
npm run test:web           # React/frontend tests

# Specific Frontend Tests
npm run test:data          # Data handling tests
npm run test:basic         # Basic functionality tests

# Quality Checks
npm run lint               # Lint TypeScript/React code
npm run clean              # Clean all build artifacts
```

## Important Development Standards

### Critical Requirements
- **ALWAYS use Linux line endings (LF) for all files**. Never use Windows line endings (CRLF).
- **All Rust cargo commands must use `--target x86_64-unknown-linux-gnu`** to avoid WASM compilation issues
- **Pre-commit hooks automatically enforce code quality** - commits are blocked if checks fail

### Pre-commit Hook Validation
The pre-commit hook (`/.git/hooks/pre-commit`) automatically runs:

```bash
# Automatic checks on commit
- Rust formatting (cargo fmt)
- Clippy linting with zero warnings
- Security vulnerability scanning (cargo audit)
- Dependency and license auditing (cargo deny)
- Build verification
- Full test suite execution
- Frontend linting (if web/ exists)
- Server code quality checks
```

### Code Quality Standards
1. **Rust Code**
   - Must pass `cargo fmt --check`
   - Zero warnings from `cargo clippy`
   - No security vulnerabilities (cargo audit)
   - All tests must pass

2. **TypeScript/React**
   - Must pass ESLint checks
   - Type-safe with strict TypeScript
   - React best practices enforced

3. **Performance Standards**
   - Sub-millisecond data query response
   - 60 FPS rendering minimum
   - <16ms interaction latency

## Testing Strategy

### Unit Testing
- Each crate has comprehensive unit tests
- Server: 18 unit tests covering core functionality
- Logger: 6 test modules with 49 total tests
- Frontend: Component and hook testing with Vitest

### Integration Testing
- Server: 8 integration tests for API endpoints
- End-to-end WebSocket data flow testing
- React integration tests with Playwright

### Performance Testing
- GPU benchmark suite for rendering performance
- Load testing for server scalability
- Memory profiling for leak detection

### Test Execution
```bash
# Must use native target for Rust tests
cd server && cargo test --target x86_64-unknown-linux-gnu
cd logger && cargo test --target x86_64-unknown-linux-gnu

# Or use npm scripts from root
npm run test:server
npm run test:logger
npm run test:web
```

## API Documentation

### Production Endpoints
Base URL: `https://api.rednax.io/api/`

### Local Development
Base URL: `https://localhost:8443/api/` (requires SSL certificates)

### Endpoints

#### `GET /api/data`
Serves time-series financial data with binary streaming.

**Query Parameters:**
- `symbol`: Trading pair (e.g., "BTC-USD")
- `type`: Data type (e.g., "MD" for market data)
- `start`: Unix timestamp for range start
- `end`: Unix timestamp for range end
- `columns`: Comma-separated column names (time,best_bid,best_ask,price,volume,side)

**Response Format:**
```
JSON Header (metadata) + Binary Data Stream (4-byte records)
```

**Example:**
```bash
curl "https://api.rednax.io/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid"
```

#### `GET /api/symbols`
Returns available trading symbols.

**Response:**
```json
{
  "symbols": ["BTC-USD", "ETH-USD", "SOL-USD", ...]
}
```

### Environment Configuration
```bash
# Production deployment
REACT_APP_API_BASE_URL=https://api.rednax.io

# Local development
REACT_APP_API_BASE_URL=https://localhost:8443
```

## Performance Optimizations

### GPU Acceleration
- WebGPU compute shaders for parallel data processing
- Hardware-accelerated line rendering
- GPU-based min/max calculations
- Efficient buffer management

### Data Serving
- Memory-mapped files for zero-copy access
- Binary protocol for minimal overhead
- HTTP/2 for multiplexed streams
- Client-side caching strategies

### Rendering Pipeline
- Separate render passes for different elements
- Instanced rendering for repeated elements
- Texture atlases for text rendering
- Frame rate limiting for power efficiency

### Memory Management
- Rust ownership for guaranteed memory safety
- Efficient buffer recycling
- Streaming large datasets
- Automatic garbage collection in WASM

## Deployment Considerations

### Production Requirements
- Linux server with 16+ GB RAM
- SSD storage for binary data files
- SSL certificates for HTTPS
- Docker or systemd for process management

### Scaling Strategy
- Horizontal scaling via load balancer
- CDN for static assets
- Read replicas for data serving
- WebSocket connection pooling

### Monitoring
- Prometheus metrics integration
- Custom performance dashboards
- Error tracking with Sentry
- Real-time alerting system

### Security
- TLS 1.3 for all connections
- CORS configuration for API access
- Rate limiting per client
- Input validation and sanitization

## Quick Reference for AI Assistants

### When Working on Rendering
1. Check `/home/xander/projects/gpu-charts/crates/renderer/CLAUDE.md`
2. GPU shaders are in `crates/renderer/src/shaders/`
3. Test with `cargo test -p renderer --target x86_64-unknown-linux-gnu`

### When Working on Data
1. Check `/home/xander/projects/gpu-charts/crates/data-manager/CLAUDE.md`
2. Binary parsing in `crates/data-manager/src/binary_parser.rs`
3. Test with `cargo test -p data-manager --target x86_64-unknown-linux-gnu`

### When Working on Frontend
1. Check `/home/xander/projects/gpu-charts/web/CLAUDE.md`
2. Components in `web/src/components/`
3. State management in `web/src/store/`
4. Test with `npm run test:web`

### When Working on Server
1. Check `/home/xander/projects/gpu-charts/server/CLAUDE.md`
2. Handlers in `server/src/handlers/`
3. Test with `npm run test:server`

### When Working on Logger
1. Check `/home/xander/projects/gpu-charts/logger/CLAUDE.md`
2. Exchange modules in `logger/src/exchanges/`
3. Test with `npm run test:logger`

## Common Development Scenarios

### Adding a New Chart Type
1. Define types in `crates/shared-types`
2. Add configuration in `crates/config-system`
3. Implement renderer in `crates/renderer`
4. Add JavaScript bindings in `crates/wasm-bridge`
5. Create React component in `web/src/components`

### Adding a New Data Source
1. Implement collector in `logger/src/exchanges/`
2. Define binary format in `logger/src/writers/`
3. Update server to handle new data type
4. Add parsing logic in `crates/data-manager`

### Performance Optimization
1. Profile with Chrome DevTools Performance tab
2. Check GPU utilization with WebGPU profiling
3. Analyze server response times
4. Monitor memory usage patterns

### Debugging Issues
1. Enable debug logging: `RUST_LOG=debug`
2. Use Chrome DevTools for frontend
3. Check server logs: `npm run docker:logs:server`
4. Validate data integrity with test scripts

## Troubleshooting Guide

### WASM Build Failures
```bash
# Clean and rebuild
npm run clean
npm run dev:wasm

# Check wasm-pack version
wasm-pack --version  # Should be 0.12.0+
```

### Server Connection Issues
```bash
# Regenerate SSL certificates
npm run setup:ssl

# Check server is running
curl -k https://localhost:8443/api/symbols

# Verify firewall rules
sudo ufw status
```

### Performance Issues
1. Check GPU acceleration is enabled in browser
2. Verify quality preset (Low/Medium/High/Ultra)
3. Monitor data transfer sizes
4. Profile rendering pipeline

### Test Failures
```bash
# Always use native target for Rust tests
cargo test --target x86_64-unknown-linux-gnu

# Run specific test
cargo test test_name --target x86_64-unknown-linux-gnu

# Verbose output
cargo test --target x86_64-unknown-linux-gnu -- --nocapture
```

## Contributing Guidelines

1. **Code Style**: Follow Rust and TypeScript conventions
2. **Testing**: Add tests for new functionality
3. **Documentation**: Update relevant CLAUDE.md files
4. **Performance**: Benchmark changes that might impact speed
5. **Security**: Review for vulnerabilities before committing

## Project Metadata

- **Repository**: https://github.com/masteryachty/gpu-charts
- **License**: MIT
- **Primary Language**: Rust (70%), TypeScript (30%)
- **Key Technologies**: WebGPU, WebAssembly, React, HTTP/2
- **Target Audience**: Professional traders and financial institutions

## Support and Resources

- **Documentation**: Component-specific CLAUDE.md files
- **Issues**: GitHub Issues for bug reports
- **Performance**: Benchmarks in `/benchmarks`
- **Examples**: Sample data in `/test-data`

---

For detailed component-specific information, refer to the individual CLAUDE.md files in each directory.