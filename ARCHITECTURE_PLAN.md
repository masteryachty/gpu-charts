# GPU Charts Architecture Overhaul Plan

## Overview

This document outlines the plan to transform our monolithic WASM charting library into a high-performance, modular architecture with clean separation of concerns.

### Goals
1. **Maximum Performance** - Handle billions of data points without JS boundary crossings
2. **Clean Architecture** - Separate data management, rendering, and configuration
3. **User-Friendly** - Automatic data selection based on chart type
4. **Extensible** - Easy to add new chart types and overlays

### Architecture Overview

```
┌─────────────────────────┐
│   React UI Layer        │
│  - Chart Selection      │
│  - Configuration Builder│
└───────────┬─────────────┘
            │ Configuration
            ▼
┌─────────────────────────┐     ┌──────────────────────┐
│  Data Manager (WASM)    │────▶│  Renderer (WASM)     │
│  - HTTP/2 Fetching      │     │  - Pure Rendering    │
│  - Binary Parsing       │     │  - GPU Pipelines     │
│  - GPU Buffer Creation  │     │  - Chart Drawing     │
│  - Memory Management    │     │  - Overlay System    │
└─────────────────────────┘     └──────────────────────┘
         Direct GPU Buffer Transfer (no JS boundary)
```

## Core Components

### 1. Data Manager (WASM)
- Handles all data fetching and parsing
- Creates GPU buffers directly from binary data
- Manages memory and caching
- Zero-copy path from network to GPU

### 2. Renderer (WASM)
- Pure rendering engine
- Accepts configuration and GPU buffer handles
- No knowledge of data sources
- Extensible renderer system

### 3. Configuration Layer (TypeScript)
- Chart type registry
- Automatic data requirement resolution
- Configuration building
- User preference management

### 4. Shared Types
- Common types between Rust and TypeScript
- Configuration schemas
- Data handle types

## Performance Considerations

### Critical Performance Requirements
1. **Zero JS Boundary Crossings** for data
2. **Direct GPU Buffer Sharing** between WASM modules
3. **Memory-Mapped I/O** where possible
4. **Efficient Binary Parsing** with SIMD
5. **GPU-Based Aggregation** for OHLC
6. **Intelligent Caching** to avoid re-fetching

### Performance Targets
- Handle 1B+ data points without degradation
- Sub-16ms render times (60 FPS)
- Sub-100ms data fetch and parse for 100M points
- Memory usage linear with data size

## Implementation Phases

See individual phase documents:
- [Phase 1: Foundation](./docs/PHASE_1_FOUNDATION.md)
- [Phase 2: Data Manager](./docs/PHASE_2_DATA_MANAGER.md)
- [Phase 3: Renderer Refactor](./docs/PHASE_3_RENDERER.md)
- [Phase 4: Configuration Layer](./docs/PHASE_4_CONFIGURATION.md)
- [Phase 5: Integration](./docs/PHASE_5_INTEGRATION.md)

## Success Criteria

### Performance Metrics
- [ ] 1 billion points rendered at 60 FPS
- [ ] Data fetching under 100ms for 100M points
- [ ] Memory usage under 2x raw data size
- [ ] Zero JS GC pressure from data operations

### Architecture Goals
- [ ] Complete separation of data and rendering
- [ ] Configuration-driven rendering
- [ ] Extensible chart type system
- [ ] Clean module boundaries

### User Experience
- [ ] Automatic data selection for chart types
- [ ] Seamless chart type switching
- [ ] Responsive interaction with large datasets
- [ ] Clear feedback during data loading