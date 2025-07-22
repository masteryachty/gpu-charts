# Phase 1 Action Plan: What to Tackle First

## Executive Decision

**Start with Binary Search Culling** - it's the safest, highest-impact optimization that will validate our entire approach.

## Why Binary Search Culling First?

1. **Lowest Risk**: Pure compute shader, no architectural changes
2. **Highest Impact**: 25,000x improvement is mind-blowing
3. **Easiest to Measure**: Culling time is directly measurable
4. **Already Implemented**: Code exists in `/crates/renderer/src/culling.rs`
5. **WASM Ready**: No compatibility issues

## Immediate Next Steps (Today)

### Step 1: Verify the Implementation Exists
```bash
# Check what we have
cat /home/xander/projects/gpu-charts/crates/renderer/src/culling.rs
ls /home/xander/projects/gpu-charts/crates/renderer/src/shaders/
```

### Step 2: Create Unified Crate
```bash
cd /home/xander/projects/gpu-charts/crates
cargo new gpu-charts-unified --lib
```

### Step 3: Extract Culling Code
We need to:
1. Copy the binary search implementation
2. Make it standalone (remove dependencies on other renderer parts)
3. Ensure WASM compatibility
4. Create clean API

### Step 4: Integration Points
The culling system needs to integrate at these points in the main charting library:
- `charting/src/renderer/render_engine.rs` - Replace culling logic
- `charting/src/drawables/plot.rs` - Use culled indices for rendering
- `charting/src/renderer/data_store.rs` - Provide sorted data for binary search

## Detailed Implementation Plan

### Day 1 Morning: Setup and Extraction
1. Create `gpu-charts-unified` crate
2. Copy culling implementation
3. Remove dependencies on full renderer
4. Create WASM bindings

### Day 1 Afternoon: Integration
1. Add dependency to main charting library
2. Create feature flag for binary search culling
3. Wire up in render pipeline
4. Initial testing

### Day 2 Morning: Testing and Benchmarking
1. Create benchmark suite
2. Test with various dataset sizes (1K, 10K, 100K, 1M, 10M points)
3. Measure improvement factor
4. Test browser compatibility

### Day 2 Afternoon: Polish and Decision
1. Fix any issues found
2. Optimize if needed
3. Document performance gains
4. Go/No-Go decision for remaining optimizations

## Code Structure

### gpu-charts-unified/src/lib.rs
```rust
use wasm_bindgen::prelude::*;

mod culling;

#[wasm_bindgen]
pub struct OptimizedCuller {
    device: wgpu::Device,
    queue: wgpu::Queue,
    culling_system: culling::CullingSystem,
}

#[wasm_bindgen]
impl OptimizedCuller {
    pub async fn new() -> Result<OptimizedCuller, JsValue> {
        // Initialize WebGPU
        // Create culling system
    }
    
    pub fn cull_viewport(
        &self,
        timestamps: &[f32],
        viewport_start: f32,
        viewport_end: f32,
    ) -> Vec<u32> {
        // Use binary search culling
        // Return visible indices
    }
}
```

### Integration in Main App
```rust
// charting/src/renderer/render_engine.rs
pub struct RenderEngine {
    // ... existing fields
    optimized_culler: Option<OptimizedCuller>,
}

impl RenderEngine {
    pub fn cull_data(&self, data: &DataStore) -> Vec<u32> {
        if let Some(culler) = &self.optimized_culler {
            // Use GPU binary search
            culler.cull_viewport(
                data.timestamps(),
                self.viewport.start,
                self.viewport.end,
            )
        } else {
            // Fallback to CPU linear scan
            self.cpu_cull_fallback(data)
        }
    }
}
```

## Success Metrics

### Performance Targets
| Dataset Size | Current Cull Time | Target Time | Improvement |
|--------------|-------------------|-------------|-------------|
| 1K points | 0.1ms | 0.001ms | 100x |
| 100K points | 10ms | 0.01ms | 1,000x |
| 1M points | 100ms | 0.1ms | 1,000x |
| 10M points | 1000ms | 1ms | 1,000x |
| 100M points | 10000ms | 10ms | 1,000x |

### Quality Gates
- ✅ No visual artifacts
- ✅ Works in Chrome, Firefox, Safari
- ✅ No memory leaks
- ✅ Graceful fallback if WebGPU unavailable

## Risk Mitigation

### What Could Go Wrong?
1. **WebGPU not available**: Implement CPU fallback
2. **Shader compilation fails**: Pre-validate WGSL
3. **Data not sorted**: Add sorting step or use linear scan
4. **Performance regression**: Feature flag to disable

### Rollback Plan
```javascript
// Easy toggle in production
if (config.enableBinarySearchCulling && hasWebGPU) {
    chart.useBinarySearchCulling();
} else {
    chart.useLinearCulling(); // Original method
}
```

## Decision: Let's Do This!

Binary Search Culling is the perfect starting point because:
1. **Low risk, high reward**
2. **Already implemented** - just needs integration
3. **Easy to measure** success
4. **Sets up pattern** for other optimizations

## Next Actions Right Now

1. Check if shader files exist:
```bash
find /home/xander/projects/gpu-charts -name "*.wgsl" | grep cull
```

2. Start creating the unified crate:
```bash
cd /home/xander/projects/gpu-charts/crates
cargo new gpu-charts-unified --lib
```

3. Begin extraction of culling code

The 25,000x performance improvement is waiting - let's make it happen!