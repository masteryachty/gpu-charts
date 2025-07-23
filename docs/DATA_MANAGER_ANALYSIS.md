# Data Manager Analysis and WASM Requirements

## Overview

The `data-manager` crate is the high-performance data fetching and GPU buffer management system for GPU Charts. It handles:
- Fetching financial time-series data from HTTP endpoints
- Parsing binary data directly into GPU buffers (zero-copy)
- Memory pooling and caching for optimal performance
- SIMD-optimized data processing
- Progressive streaming for large datasets

## Current State

### What's Working
1. ✅ Core data structures (DataManager, BufferPool, Cache)
2. ✅ Basic HTTP fetching via reqwest (WASM-compatible)
3. ✅ Binary data parsing logic
4. ✅ GPU buffer creation with wgpu
5. ✅ WASM API module exists

### What's Broken/Incompatible with WASM
1. ❌ **File System Operations**: `tokio::fs` used in chunked.rs
2. ❌ **Memory Mapping**: `memmap2` in direct_gpu_parser.rs (not available in WASM)
3. ❌ **Async Runtime Issues**: Using tokio features not available in WASM
   - `tokio::time` (timers/sleep)
   - `tokio::sync::broadcast`, `mpsc`, `RwLock`, `Semaphore`
   - `tokio::io` async traits
4. ❌ **HTTP/2 Client**: Custom hyper-based client (native only)
5. ❌ **WebSocket Client**: tokio-tungstenite (native only)
6. ❌ **Compression**: zstd not properly configured for WASM
7. ❌ **Missing WASM Features**: wasm-bindgen not enabled in build

## Purpose in the System

The data-manager serves as the critical data pipeline:
```
User Request → Data Manager → GPU Buffers → Renderer
                    ↓
              - Fetch from API
              - Parse binary data
              - Create GPU buffers
              - Manage memory
              - Cache results
```

### Key Responsibilities
1. **Zero-Copy Performance**: Parse server data directly into GPU buffers without JavaScript intermediaries
2. **Memory Management**: Pool and reuse GPU buffers to avoid allocation overhead
3. **Caching**: LRU cache to avoid redundant fetches
4. **SIMD Optimization**: Use WASM SIMD for fast data processing
5. **Progressive Loading**: Stream large datasets in chunks

## What Needs to Be Done

### 1. Fix Cargo.toml Features (Immediate)
```toml
[features]
default = ["wasm"]
native = ["tokio/full", "hyper", "hyper-tls", "tokio-tungstenite", "memmap2", "zstd"]
wasm = ["wasm-bindgen", "wasm-bindgen-futures", "getrandom/js", "tokio/sync", "tokio/time"]
```

### 2. Replace Incompatible Async Operations
- **File System**: Remove or make conditional all `tokio::fs` usage
- **Timers**: Use `wasm-bindgen-futures` for delays instead of `tokio::time`
- **Channels**: Replace `tokio::sync` channels with `futures::channel`
- **Locks**: Use `parking_lot` (already in use) instead of async locks

### 3. Fix Module Conditionals
```rust
// chunked.rs - make entire module conditional
#[cfg(feature = "native")]
pub mod chunked;

// direct_gpu_parser.rs - fix memmap2 usage
#[cfg(feature = "native")]
use memmap2::Mmap;
#[cfg(target_arch = "wasm32")]
// Use Vec<u8> instead of memory mapping
```

### 4. Implement WASM-Specific Features
- **WASM Timer**: Create browser-compatible timer for delays
- **WASM Channels**: Use JavaScript Promise-based channels
- **Compression**: Use browser-native compression APIs or WASM-compatible libraries

### 5. Add WASM SIMD Implementation
```rust
// simd.rs
#[cfg(target_arch = "wasm32")]
pub fn process_f32_simd(data: &[f32]) -> Vec<f32> {
    use std::arch::wasm32::*;
    // Implement v128 SIMD operations
}
```

### 6. Fix Request Batching
The `request_batching.rs` imports non-existent `http2_client`. Either:
- Remove request batching for WASM (simpler)
- Implement using `reqwest` batching (already available)

## Integration with WASM Bridge

Once fixed, the data-manager will integrate with wasm-bridge like this:

```rust
// In wasm-bridge lib.rs
let data_manager = gpu_charts_data::DataManager::new_with_device(
    device.clone(),
    queue.clone(),
    base_url
);

// Fetch data
let handle = data_manager.fetch_data(&request_json).await?;

// Pass GPU buffers to renderer
renderer.set_data_buffers(&handle)?;
```

## Priority Actions

1. **High Priority**: Fix Cargo.toml and module conditionals
2. **High Priority**: Replace tokio async operations with WASM-compatible alternatives  
3. **Medium Priority**: Implement WASM SIMD optimizations
4. **Low Priority**: Add progressive streaming support

## Testing Strategy

1. **Conditional Compilation**: Ensure all tests use `#[cfg(not(target_arch = "wasm32"))]`
2. **WASM Tests**: Create browser-based tests using `wasm-bindgen-test`
3. **Mock Data**: Use mock HTTP responses for WASM testing
4. **Performance Tests**: Benchmark SIMD vs non-SIMD paths

The data-manager is critical infrastructure - getting it WASM-compatible is essential for the entire GPU Charts system to function in the browser.