# Phase 1 Decision Framework: GPU Optimizations

## Current Situation

We have 4 GPU optimizations ready in `/crates/renderer/src/`:
1. **Binary Search Culling** (`culling.rs`) - 25,000x faster viewport culling
2. **Vertex Compression** (`vertex_compression.rs`) - 75% memory reduction
3. **GPU Vertex Generation** (`gpu_vertex_gen.rs`) - 4x render speed
4. **Render Bundles** (`render_bundles.rs`) - 30% CPU reduction

All are already implemented and WASM-compatible!

## Decision Matrix

### Option 1: Start with Binary Search Culling (Recommended) ✅

**Why Start Here:**
- Simplest to integrate (just replace culling logic)
- Most dramatic improvement (25,000x)
- Pure compute shader - zero WASM issues
- Easy to measure and verify
- Can be tested in isolation

**Implementation Path:**
```
Day 1: Create unified crate + integrate culling
Day 2: Test and benchmark
Success metric: 1000x+ improvement on large datasets
```

### Option 2: Start with Vertex Compression

**Pros:**
- Biggest memory impact (75% reduction)
- Already has pack/unpack utilities
- Works on GPU only

**Cons:**
- Requires changing vertex format throughout pipeline
- More integration points to modify
- Harder to A/B test

### Option 3: Start with GPU Vertex Generation

**Pros:**
- Eliminates CPU→GPU transfer
- 4x overall render improvement

**Cons:**
- Most complex integration
- Requires rewriting render pipeline
- Hardest to rollback if issues

### Option 4: Do All Four Together

**Pros:**
- Get all benefits at once
- Single integration effort

**Cons:**
- Hard to isolate issues
- Can't measure individual impact
- Higher risk

## Recommended Approach

### Phase 1A: Binary Search Culling (Days 1-2)
```bash
# Create unified crate
cd /home/xander/projects/gpu-charts/crates
cargo new gpu-charts-unified --lib

# Copy culling implementation
cp renderer/src/culling.rs gpu-charts-unified/src/
cp renderer/src/shaders/cull_compute.wgsl gpu-charts-unified/src/shaders/

# Integrate into main charting lib
# Test with large dataset (1M+ points)
```

**Success Criteria:**
- Culling time < 0.1ms for 1M points
- No visual artifacts
- Works in Chrome, Firefox, Safari

### Phase 1B: Vertex Compression (Days 3-4)
```bash
# Add to unified crate
cp renderer/src/vertex_compression.rs gpu-charts-unified/src/
cp renderer/src/shaders/vertex_compression.wgsl gpu-charts-unified/src/shaders/

# Update vertex pipeline
# Test memory usage
```

**Success Criteria:**
- Memory usage reduced by 70%+
- No precision loss visible at normal zoom
- Decompression overhead < 0.1ms

### Phase 1C: GPU Vertex Generation (Days 5-7)
```bash
# Most complex - changes entire pipeline
cp renderer/src/gpu_vertex_gen.rs gpu-charts-unified/src/
cp renderer/src/shaders/vertex_gen.wgsl gpu-charts-unified/src/shaders/

# Rewrite CPU vertex building
# Add indirect draw support
```

**Success Criteria:**
- Zero CPU vertex building time
- 3x+ overall render improvement
- Dynamic LOD working

### Phase 1D: Render Bundles (Days 8-9)
```bash
# Final optimization
cp renderer/src/render_bundles.rs gpu-charts-unified/src/

# Cache render commands
# Measure CPU usage reduction
```

**Success Criteria:**
- 25%+ CPU usage reduction
- Command recording < 1ms
- Cache invalidation working

## Integration Strategy

### 1. Feature Flags for Each Optimization
```rust
pub struct OptimizationFlags {
    pub use_binary_search_culling: bool,  // Start true
    pub use_vertex_compression: bool,      // Start false
    pub use_gpu_vertex_gen: bool,          // Start false
    pub use_render_bundles: bool,          // Start false
}
```

### 2. Performance Monitoring
```javascript
// In React app
const metrics = {
    cullTime: 0,
    memoryUsage: 0,
    fps: 0,
    cpuUsage: 0,
};

// Compare before/after
console.log(`Culling improvement: ${oldCullTime / newCullTime}x`);
```

### 3. A/B Testing
```javascript
// 50% of users get optimization
if (Math.random() < 0.5 || forceOptimization) {
    chart.enableBinarySearchCulling();
}
```

## Quick Start Commands

```bash
# 1. Create the unified crate
cd /home/xander/projects/gpu-charts
mkdir -p crates/gpu-charts-unified/src/shaders
cd crates/gpu-charts-unified

# 2. Set up Cargo.toml
cat > Cargo.toml << 'EOF'
[package]
name = "gpu-charts-unified"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wgpu = "24.0"
bytemuck = { version = "1.7", features = ["derive"] }
gpu-charts-shared = { path = "../shared-types" }

[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "0.3", features = ["console"] }
console_error_panic_hook = "0.1"
EOF

# 3. Copy first optimization
cp ../renderer/src/culling.rs src/
mkdir -p src/shaders
# Note: Need to find/create the shader file

# 4. Create lib.rs
cat > src/lib.rs << 'EOF'
use wasm_bindgen::prelude::*;

pub mod culling;

#[wasm_bindgen]
pub struct GPUChartsUnified {
    culling_system: culling::CullingSystem,
}

#[wasm_bindgen]
impl GPUChartsUnified {
    pub async fn new() -> Result<GPUChartsUnified, JsValue> {
        // Implementation
        todo!()
    }
}
EOF

# 5. Build for WASM
wasm-pack build --target web --out-dir ../../web/pkg-unified
```

## Go/No-Go Decision Points

### After Binary Search Culling (Day 2)
**GO if:**
- 1000x+ improvement verified
- No browser compatibility issues
- Integration was straightforward

**STOP if:**
- Performance gain < 100x
- WebGPU issues on target browsers
- Integration requires major refactoring

### After Each Subsequent Optimization
**GO if:**
- Expected performance gain achieved
- No regressions
- Clean integration

**REASSESS if:**
- Diminishing returns
- Complexity exceeding value
- Time overrun

## Expected Outcomes by End of Phase 1

| Metric | Current | After Culling | After Compression | After GPU Gen | After Bundles |
|--------|---------|---------------|-------------------|---------------|---------------|
| FPS (1M points) | 15 | 30 | 35 | 50 | 55 |
| Memory (MB) | 400 | 400 | 100 | 100 | 100 |
| Cull Time (ms) | 50 | 0.05 | 0.05 | 0 | 0 |
| CPU Usage | 80% | 60% | 55% | 30% | 20% |

## Recommendation

**Start with Binary Search Culling TODAY**. It's the lowest risk, highest reward optimization that will prove the integration approach and deliver immediate value. The implementation is already there - we just need to wire it up.

After 2 days, we'll know if this approach works and can decide whether to continue with the remaining optimizations.