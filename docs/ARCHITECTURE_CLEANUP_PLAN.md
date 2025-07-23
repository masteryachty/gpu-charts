# Architecture Cleanup Plan

## Current State vs Original Design

### Original Design (NEW_ARCHITECTURE.md)
- 3 main WASM components: data-manager, renderer, wasm-bridge
- Simple, focused architecture
- Zero JS boundary crossings
- Direct GPU buffer sharing

### Current State
We have added many extra crates that weren't in the original design:
- **config-system**: Phase 3 configuration management
- **system-integration**: Tries to bridge everything together
- **gpu-charts-unified**: Unknown purpose
- **wasm-fetch, wasm-storage, wasm-websocket**: Additional WASM utilities

## Why We're Off Track

1. **Phase 3 Optimizations**: Added complexity with config-system and system-integration
2. **Over-engineering**: system-integration tries to create bridges and APIs that duplicate wasm-bridge's purpose
3. **API Drift**: Crates were developed independently without coordinating APIs

## Cleanup Recommendations

### Option 1: Minimal Cleanup (Recommended)
Keep the original 3-component design but integrate useful Phase 3 features:

1. **Keep config-system** but integrate it directly into wasm-bridge
   - It's already WASM-compatible
   - Provides useful configuration management
   
2. **Remove system-integration**
   - It duplicates wasm-bridge's orchestration role
   - Has significant API mismatches
   - Would require major rewrite

3. **Move Phase 3 optimizations** into the core crates:
   - Vertex compression → renderer
   - Binary culling → renderer
   - SIMD optimizations → data-manager

### Option 2: Full Cleanup
Strictly follow NEW_ARCHITECTURE.md:

1. Remove all extra crates (config-system, system-integration, etc.)
2. Move any useful code into the 3 core crates
3. Simplify APIs to match original design

## Immediate Actions

### 1. Fix wasm-bridge to work without system-integration
```rust
// Instead of complex system-integration
// Direct integration in wasm-bridge:
pub struct ChartSystem {
    data_manager: DataManager,
    renderer: Renderer,
    config: GpuChartsConfig, // from config-system
}
```

### 2. Update wasm-bridge imports
Remove references to system-integration and use data-manager/renderer directly.

### 3. Test basic functionality
Ensure the 3-component system works before adding optimizations.

## Benefits of Cleanup

1. **Simpler Architecture**: Matches original design document
2. **Fewer Dependencies**: Reduces compilation time and complexity
3. **Clear Responsibilities**: Each crate has a focused purpose
4. **Easier Maintenance**: Less code to maintain and debug

## Migration Path

### Phase 1: Get Basic System Working (1-2 days)
- Fix wasm-bridge to use data-manager and renderer directly
- Remove system-integration dependency
- Test basic chart rendering

### Phase 2: Integrate Config System (1 day)
- Move config management into wasm-bridge
- Keep the config structures but simplify integration

### Phase 3: Add Optimizations (2-3 days)
- Cherry-pick useful Phase 3 optimizations
- Integrate them into renderer/data-manager
- Maintain simple API surface

## Decision Required

Should we:
1. **Option 1**: Keep config-system, remove system-integration, integrate optimizations
2. **Option 2**: Full cleanup to match NEW_ARCHITECTURE.md exactly

The key insight is that system-integration is redundant with wasm-bridge's purpose. The wasm-bridge should be the only orchestrator, directly using data-manager and renderer.