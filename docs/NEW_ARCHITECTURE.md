# New GPU Charts Architecture

## Overview

The GPU Charts project has been restructured into a high-performance, modular architecture with three main WASM components:

1. **Data Manager** - Handles all data fetching, parsing, and GPU buffer management
2. **Renderer** - Pure rendering engine driven by configuration
3. **WASM Bridge** - Orchestrates the data manager and renderer

## Key Performance Features

- **Zero JS Boundary Crossings**: Data never crosses the JavaScript/WASM boundary
- **Direct GPU Buffer Sharing**: Buffers are shared between WASM modules without copying
- **GPU Buffer Pooling**: Reuse allocations to minimize memory pressure
- **LRU Cache**: Intelligent caching of frequently accessed data
- **Binary Parsing**: Direct network-to-GPU data pipeline

## Project Structure

```
gpu-charts/
├── crates/
│   ├── shared-types/     # Shared types between all crates
│   ├── data-manager/     # Data fetching and GPU buffer management
│   ├── renderer/         # Pure rendering engine
│   └── wasm-bridge/      # JS/WASM interop layer
├── charting/             # Legacy monolithic WASM (to be migrated)
├── server/               # High-performance data server
├── coinbase-logger/      # Real-time data collection
└── scripts/              # Build and performance scripts
```

## Building

### Development Build
```bash
npm run dev:wasm
```

### Production Build
```bash
npm run build:wasm
```

### Legacy Build (for existing charting)
```bash
npm run dev:wasm:legacy
```

## Performance Benchmarking

### Generate Test Data
```bash
npm run perf:generate-data
```

### Run Benchmarks
```bash
# All benchmarks
npm run bench

# Data manager benchmarks
npm run bench:data

# Renderer benchmarks  
npm run bench:renderer
```

### Performance Monitoring
```bash
npm run perf:monitor
```

## Testing

### Run All Tests
```bash
npm run test:all
```

### Test New Crates
```bash
npm run test:crates
```

## Development Workflow

1. Make changes to the appropriate crate
2. Run `npm run dev:wasm` to rebuild
3. Test with `npm run test:crates`
4. Benchmark with `npm run bench`
5. Monitor performance with `npm run perf:monitor`

## Migration Status

- [x] Workspace structure created
- [x] Shared types defined
- [x] Data manager structure created
- [x] Renderer structure created
- [x] WASM bridge created
- [x] Performance benchmarking infrastructure
- [ ] Migrate existing code from charting/
- [ ] Implement data fetching
- [ ] Implement GPU buffer sharing
- [ ] Complete renderer implementation
- [ ] Integration testing

## Performance Targets

- 1B+ points rendered at 60 FPS
- <100ms data fetch for 100M points
- <500KB WASM size (gzipped)
- Zero JS GC pressure
- 80%+ cache hit rate