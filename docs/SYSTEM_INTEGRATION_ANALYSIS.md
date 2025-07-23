# System Integration Analysis and WASM Requirements

## Overview

The `system-integration` crate is designed to be the central orchestrator that connects all GPU Charts subsystems:
- **DataManager**: Handles data fetching and GPU buffer management
- **Renderer**: Manages WebGPU rendering pipeline
- **Config System**: Manages configuration and hot-reload
- **Error Recovery**: Handles errors gracefully
- **Lifecycle Management**: Coordinates initialization and shutdown

## Current State

### What's the Purpose
The system-integration crate serves as:
1. **Central Hub**: Single point of coordination for all subsystems
2. **Bridge Layer**: Adapters between different subsystem APIs
3. **Lifecycle Manager**: Handles initialization order and dependencies
4. **Error Recovery**: Graceful error handling and recovery
5. **Unified API**: Single API surface for the WASM bridge to interact with

### What's Broken/Incompatible
1. ❌ **Import Errors**: Types don't exist in the expected modules
   - `BufferHandle`, `BufferMetadata`, `DataSource` not exported from data-manager root
   - `Phase2Config`, `Phase2Renderer` don't exist in renderer
2. ❌ **API Mismatches**: Methods being called don't exist
   - `DataManager::new()` doesn't match actual constructor signature
3. ❌ **Missing Re-exports**: Types are in submodules but not re-exported
4. ❌ **Outdated References**: References to "Phase2" components that may have been renamed

## What Needs to Be Done

### 1. Fix Import Paths (Immediate)
```rust
// Current (broken):
use gpu_charts_data::{BufferHandle, BufferMetadata, DataManager, DataManagerConfig, DataSource};

// Should be:
use gpu_charts_data::{DataManager, WasmDataManager};
use gpu_charts_data::handle::{BufferHandle, BufferMetadata};
use gpu_charts_data::manager::{DataManagerConfig, DataSource};
```

### 2. Update Constructor Calls
```rust
// Current (broken):
DataManager::new(device, queue, dm_config.clone())

// Should be:
DataManager::new_with_device(device, queue, base_url)
```

### 3. Remove Phase2 References
The renderer seems to have moved away from "Phase2" naming. Need to:
- Find the actual renderer types
- Update imports accordingly
- Remove outdated Phase2 references

### 4. Add Missing Re-exports
In the respective crates, add:
```rust
// In data-manager/src/lib.rs
pub use handle::{BufferHandle, BufferMetadata};
pub use manager::{DataManagerConfig, DataSource};

// In renderer/src/lib.rs
pub use config::RendererConfig;
```

### 5. Make WASM-Compatible
- Ensure all async operations work in WASM
- Remove or conditionally compile any native-only features
- Add proper error handling for WASM constraints

## Module Structure

### Current Modules
- **api.rs**: External API surface
- **bridge.rs**: Bridges between subsystems
- **error_recovery.rs**: Error handling and recovery
- **lifecycle.rs**: Initialization and shutdown coordination
- **unified_api.rs**: Single unified API for all operations

### Integration Flow
```
WASM Bridge
     ↓
SystemIntegration (this crate)
     ↓
  ┌──────────────┬──────────────┬────────────┐
  │              │              │            │
DataManager   Renderer    ConfigSystem   Lifecycle
  │              │              │            │
  └──────────────┴──────────────┴────────────┘
                 ↓
            Unified API
```

## Priority Actions

1. **High Priority**: Fix all import errors by finding correct paths
2. **High Priority**: Update constructor calls to match actual APIs
3. **Medium Priority**: Add proper re-exports in dependency crates
4. **Low Priority**: Add comprehensive error recovery

## Expected Integration Points

### With WASM Bridge
```rust
// The WASM bridge will use system-integration like this:
let system = SystemIntegration::new(device, queue, config).await?;

// Single API for all operations
let chart_id = system.api().create_chart(chart_config).await?;
system.api().update_data(chart_id, data_request).await?;
system.api().render_frame(chart_id)?;
```

### With Data Manager
- Request data through unified API
- Manage buffer lifecycle
- Handle caching and prefetching

### With Renderer
- Create and manage render pipelines
- Update render configurations
- Handle viewport changes

## Testing Strategy

1. **Unit Tests**: Test each bridge in isolation
2. **Integration Tests**: Test full system flow
3. **WASM Tests**: Ensure all APIs work in browser
4. **Error Tests**: Verify error recovery works

## Success Criteria

The system-integration is ready when:
1. ✅ All imports resolve correctly
2. ✅ Compiles cleanly to WASM
3. ✅ All subsystems integrate properly
4. ✅ Unified API provides complete functionality
5. ✅ Error recovery handles common failures
6. ✅ Works seamlessly with WASM bridge

The main issue is that this crate was written against an older or different API surface than what currently exists. Need to update all references to match the actual implementations.