# Renderer Crate Cleanup Plan

## Overview
The renderer crate is already mostly WASM-compatible and well-structured. It only needs minor fixes to be fully ready for WASM integration.

## Issues to Fix

### 1. **WASM Compatibility**
- Replace `std::time::Instant` with WASM-compatible timing
- Add feature flags for WASM vs native builds
- Ensure wgpu backends are set correctly for WASM

### 2. **Compilation Warnings**
- Fix `RenderTarget` visibility in multi_resolution.rs
- Remove unused fields:
  - `vertex_count` in LineChartRenderer
  - `device` and `cull_pipeline` in CullingSystem
  - `surface_texture` in RenderEngine

### 3. **API Cleanup**
- Ensure all public methods are WASM-safe
- Add proper error handling for WASM context
- Document which features work in WASM

## Implementation Steps

### Step 1: Add WASM Feature Flags
Update Cargo.toml:
```toml
[features]
default = ["wasm"]
wasm = []
native = ["timestamps"]
timestamps = []
```

### Step 2: Fix Timing for WASM
Create a cross-platform timing abstraction:
```rust
#[cfg(target_arch = "wasm32")]
use web_sys::window;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

pub struct Timer {
    #[cfg(target_arch = "wasm32")]
    start: f64,
    #[cfg(not(target_arch = "wasm32"))]
    start: Instant,
}
```

### Step 3: Fix Compilation Warnings
- Make `RenderTarget` public or adjust method visibility
- Remove or use the unused fields
- Add `#[allow(dead_code)]` where appropriate

### Step 4: Update WebGPU Backend Selection
```rust
// In gpu_context.rs
let backends = if cfg!(target_arch = "wasm32") {
    wgpu::Backends::BROWSER_WEBGPU
} else {
    wgpu::Backends::all()
};
```

### Step 5: Create Public API Module
Create `src/api.rs` that exposes only WASM-safe interfaces:
```rust
pub use crate::{
    Renderer,
    Viewport,
    PerformanceMetrics,
    GpuBufferSet,
};
```

## Benefits

1. **Full WASM Compatibility**: Renderer will compile and run in browser
2. **Clean API**: Clear separation between internal and public interfaces
3. **No Warnings**: Clean compilation output
4. **Better Performance**: Proper backend selection for each platform

## Testing

After implementation:
1. Test WASM compilation: `cargo build --target wasm32-unknown-unknown`
2. Test native compilation: `cargo build`
3. Run benchmarks to ensure no performance regression
4. Verify all features work in both environments