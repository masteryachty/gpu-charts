# Phase 3 Rendering Migration Plan

## Overview
This document outlines the plan to migrate from the legacy charting library to the new Phase 3 architecture and connect the configuration system to actual chart rendering.

## Current State
- **Legacy System**: Fully functional rendering in `/charting` directory
- **New System**: Implemented but not connected (`/crates/renderer`)
- **Configuration**: Working but only affects settings, not rendering
- **WASM Bridge**: Minimal version avoids dependency issues

## Migration Strategy

### Phase 1: Create Full WASM Bridge with Rendering
1. Fix dependency issues in the full WASM bridge
2. Create a unified WebGPU initialization system
3. Connect DataManager → Renderer with shared GPU resources
4. Expose rendering APIs through WASM

### Phase 2: Create Transition Component
1. Build a React component that can use both renderers
2. Add feature flag to switch between legacy and new
3. Implement side-by-side comparison mode
4. Ensure feature parity for basic line charts

### Phase 3: Incremental Feature Migration
1. Migrate line chart rendering first
2. Add candlestick chart support
3. Implement axis renderers
4. Add grid and labels
5. Port mouse/zoom interactions

### Phase 4: Connect Configuration
1. Wire configuration updates to renderer
2. Implement quality preset effects
3. Enable feature toggles (scatter, heatmap, etc.)
4. Add performance monitoring

### Phase 5: Complete Migration
1. Remove legacy rendering code
2. Update all imports to use new system
3. Clean up build scripts
4. Update documentation

## Technical Implementation Steps

### Step 1: Fix Full WASM Bridge Dependencies
```rust
// Create feature flags to exclude problematic deps
[features]
default = ["wasm"]
wasm = ["web-sys", "wasm-bindgen", "js-sys"]
native = ["tokio/full", "hyper", "memmap2"]
```

### Step 2: Unified WebGPU Initialization
```rust
// In wasm-bridge/src/webgpu_init.rs
pub async fn initialize_webgpu(canvas_id: &str) -> Result<(Device, Queue, Surface)> {
    // Create adapter, device, queue, and surface
    // Share these across DataManager and Renderer
}
```

### Step 3: Connect Components
```rust
// In wasm-bridge/src/lib.rs
impl ChartSystem {
    pub async fn new(canvas_id: String) -> Result<Self> {
        // Initialize WebGPU
        let (device, queue, surface) = initialize_webgpu(&canvas_id).await?;
        
        // Create components with shared resources
        let data_manager = DataManager::new(&device, &queue);
        let renderer = Renderer::new(&device, &queue, surface).await?;
        let config_manager = HotReloadManager::new(default_config());
        
        // Connect them
        let system_integration = SystemIntegration::new(
            data_manager.clone(),
            renderer.clone(),
            config_manager.clone()
        );
        
        Ok(Self { /* ... */ })
    }
}
```

### Step 4: React Integration
```typescript
// New hook for Phase 3 renderer
export function usePhase3Chart() {
    const [chart, setChart] = useState<ChartSystem | null>(null);
    
    useEffect(() => {
        const init = async () => {
            const wasmModule = await import('@pkg/gpu_charts_wasm');
            await wasmModule.default();
            
            const chartSystem = new wasmModule.ChartSystem('canvas-id');
            setChart(chartSystem);
        };
        init();
    }, []);
    
    return chart;
}
```

## Key Challenges

### 1. WebGPU Context Sharing
- Legacy creates its own context
- New system expects shared context
- Solution: Centralized initialization

### 2. Data Format Compatibility
- Legacy uses custom data format
- New system uses GPU-optimized buffers
- Solution: Adapter layer during transition

### 3. Feature Parity
- Legacy has working zoom/pan
- New system needs these implemented
- Solution: Port interaction handlers

### 4. Performance Monitoring
- Need to prove new system is faster
- Solution: Built-in metrics collection

## Success Criteria
1. New renderer displays charts correctly
2. Configuration changes affect rendering
3. Performance improves (measured by FPS)
4. All legacy features work
5. Clean migration path

## Timeline Estimate
- Week 1: Fix dependencies and WebGPU init
- Week 2: Basic rendering working
- Week 3: Feature parity
- Week 4: Configuration integration
- Week 5: Testing and optimization
- Week 6: Cleanup and documentation