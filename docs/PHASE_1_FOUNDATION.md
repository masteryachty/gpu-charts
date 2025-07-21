# Phase 1: Foundation

## Overview
Set up the foundational structure for the new architecture, including project organization, shared types, and performance benchmarking infrastructure.

## Duration: 3-4 days

## Tasks

### 1.1 Project Restructuring
- [ ] Create new Rust workspace structure
  ```
  gpu-charts/
  ├── Cargo.toml (workspace)
  ├── crates/
  │   ├── shared-types/     # Shared types between all crates
  │   ├── data-manager/     # Data fetching and GPU buffer management
  │   ├── renderer/         # Pure rendering engine
  │   └── wasm-bridge/      # JS/WASM interop layer
  ```
- [ ] Move existing code to appropriate crates
- [ ] Set up workspace dependencies
- [ ] Configure wasm-pack for multi-crate builds

### 1.2 Performance Benchmarking Infrastructure
- [ ] Create benchmark harness for data operations
- [ ] Create benchmark harness for rendering operations
- [ ] Set up performance regression testing
- [ ] Add memory usage tracking
- [ ] Create test datasets of various sizes (1M, 10M, 100M, 1B points)

### 1.3 Shared Types Definition
- [ ] Define core data structures in Rust
  ```rust
  // shared-types/src/lib.rs
  pub struct TimeRange {
      pub start: u64,
      pub end: u64,
  }
  
  pub struct DataHandle {
      pub id: Uuid,
      pub metadata: DataMetadata,
  }
  
  pub struct ChartConfiguration {
      pub chart_type: ChartType,
      pub data_handles: Vec<DataHandle>,
      pub visual_config: VisualConfig,
  }
  ```
- [ ] Create TypeScript type definitions
- [ ] Set up automatic TypeScript generation from Rust types
- [ ] Define serialization strategy (bincode for performance)

### 1.4 GPU Resource Management
- [ ] Design GPU buffer pooling system
- [ ] Implement buffer lifecycle management
- [ ] Create buffer sharing mechanism between modules
- [ ] Add GPU memory pressure monitoring

### 1.5 Build System Updates
- [ ] Update build scripts for multi-crate WASM
- [ ] Configure release builds with maximum optimization
  ```toml
  [profile.release]
  opt-level = 3
  lto = true
  codegen-units = 1
  ```
- [ ] Set up size tracking for WASM modules
- [ ] Add performance-focused lint rules

## Performance Checkpoints

### Memory Management
- [ ] GPU buffer pool reduces allocations by >90%
- [ ] Zero-copy buffer sharing verified between modules
- [ ] Memory usage stays under 2x data size

### Build Performance
- [ ] WASM module size under 500KB (gzipped)
- [ ] Build time under 30 seconds for release
- [ ] Hot reload working for development

### Type Safety
- [ ] TypeScript types auto-generated from Rust
- [ ] Serialization round-trip tests passing
- [ ] No any types in TypeScript code

## Success Criteria
- [ ] Workspace structure created and building
- [ ] Performance benchmarks running and baseline established
- [ ] Shared types compiled and accessible from all crates
- [ ] GPU buffer sharing proof-of-concept working
- [ ] CI/CD updated for new structure

## Risks & Mitigations
- **Risk**: WebGPU buffer sharing between WASM modules
  - **Mitigation**: Use buffer indices/handles instead of direct references
- **Risk**: Build complexity with multiple WASM modules
  - **Mitigation**: Create unified build script, consider single WASM with multiple entry points

## Dependencies
- wasm-bindgen 0.2
- wgpu 0.20
- bincode 1.3
- uuid 1.0
- ts-rs (for TypeScript generation)

## Next Phase
[Phase 2: Data Manager](./PHASE_2_DATA_MANAGER.md) - Build the high-performance data management module