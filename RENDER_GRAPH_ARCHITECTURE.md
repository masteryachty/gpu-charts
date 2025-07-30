# Render Graph Architecture Visualization

## Current vs Proposed Architecture

### Current Architecture (Complex, Sequential)

```
┌─────────────────────────────────────────────────────────────┐
│                    RenderLoopController                      │
│  ┌─────┐  ┌──────────┐  ┌──────────────┐  ┌──────────┐    │
│  │ Off │→ │PreProcess│→ │PreProcessing │→ │Rendering │    │
│  └─────┘  └──────────┘  └──────────────┘  └──────────┘    │
│     ↑          ↓              ↓                  ↓          │
│     └─────────────────────────────────────────────          │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│                        MultiRenderer                         │
│  ┌────────────┐  ┌──────────────┐  ┌─────────────┐        │
│  │PlotRenderer│  │CandlestickRen│  │AxisRenderer │        │
│  └────────────┘  └──────────────┘  └─────────────┘        │
│       Each creates own render pass (inefficient)            │
└─────────────────────────────────────────────────────────────┘
```

### Proposed Render Graph Architecture (Efficient, Parallel)

```
┌─────────────────────────────────────────────────────────────┐
│                      Render Graph                            │
│                                                              │
│  Compute Nodes                 Render Nodes                  │
│  ┌────────────┐               ┌─────────────┐              │
│  │DataFetch   │──────────────▶│Clear Pass   │              │
│  └────────────┘               └─────────────┘              │
│         │                            │                       │
│         ▼                            ▼                       │
│  ┌────────────┐               ┌─────────────┐              │
│  │ComputeBounds│──────┬──────▶│Background   │              │
│  └────────────┘       │       │(Candlestick)│              │
│         │             │       └─────────────┘              │
│         ▼             │              │                       │
│  ┌────────────┐       │              ▼                       │
│  │ComputeAvg  │───────┴──────▶┌─────────────┐              │
│  └────────────┘               │Main Pass    │              │
│                               │(Plot Lines) │              │
│                               └─────────────┘              │
│                                      │                       │
│                                      ▼                       │
│                               ┌─────────────┐              │
│                               │Overlay Pass │              │
│                               │(Axes)       │              │
│                               └─────────────┘              │
└─────────────────────────────────────────────────────────────┘
```

## Render Pass Batching Example

### Current (Multiple Passes)
```
Frame N:
├─ Clear Pass
├─ Candlestick Pass (own clear)
├─ Plot Pass (own clear)
├─ X-Axis Pass (own clear)
└─ Y-Axis Pass (own clear)
Total: 5 render passes ❌
```

### Proposed (Batched Passes)
```
Frame N:
├─ Compute Phase (parallel)
│  ├─ Bounds calculation
│  └─ Average calculation
└─ Render Phase (batched)
   ├─ Clear + Background Pass
   ├─ Main Pass (all plots)
   └─ Overlay Pass (all UI)
Total: 3 render passes ✅
```

## State Flow Comparison

### Current State Flow
```
User Input
    ↓
React State ──────┐
    ↓             ↓
WASM Bridge   (duplicate)
    ↓             ↓
Rust State    GPU State
    ↓             ↓
Check Dirty   Check Dirty
    ↓             ↓
Maybe Render  Maybe Update
```

### Proposed Unified State Flow
```
User Input
    ↓
State Diff Generator
    ↓
Update Plan
    ↓
Render Graph Execution
    ├─ Compute Phase
    └─ Render Phase
```

## Performance Impact Visualization

```
Current Timeline (Sequential):
|--Data--|--Compute--|--Clear--|--Render1--|--Render2--|--Render3--|
                                                         Total: 150ms

Proposed Timeline (Parallel + Batched):
|--Data--|==Compute==|--Batch1--|--Batch2--|
         (parallel)              Total: 80ms

Legend:
-- Sequential execution
== Parallel execution
```

## Memory Layout Optimization

### Current Memory Usage
```
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│ Plot Buffer │ │Candle Buffer│ │ Axis Buffer │
│   (10MB)    │ │   (10MB)    │ │   (2MB)     │
└─────────────┘ └─────────────┘ └─────────────┘
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│Plot BindGrp │ │Candle BndGrp│ │Axis BindGrp │
└─────────────┘ └─────────────┘ └─────────────┘
Total: 22MB + 3 bind groups
```

### Proposed Memory Usage
```
┌─────────────────────────┐
│   Shared Data Buffer    │
│        (15MB)           │
└─────────────────────────┘
┌─────────────────────────┐
│  Resource Pool (cached) │
│    - Bind Groups        │
│    - Pipelines          │
└─────────────────────────┘
Total: 15MB + pooled resources
```

## Code Simplification Example

### Current Renderer Implementation
```rust
impl PlotRenderer {
    fn render(&mut self, encoder, view, data_store, device, queue) {
        // Manual state checking
        if !data_store.is_dirty() { return; }
        
        // Manual pass creation
        let mut pass = encoder.begin_render_pass(&desc);
        
        // Manual resource binding
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        
        // Manual draw call
        pass.draw(0..vertex_count, 0..1);
    }
}
```

### Proposed Graph Node Implementation
```rust
impl RenderNode for PlotNode {
    fn execute(&self, context: &mut RenderContext) {
        context.draw(DrawCommand {
            pipeline: PipelineId::Plot,
            vertices: self.vertex_range(),
            instances: 1,
        });
    }
    
    fn dependencies(&self) -> &[NodeId] {
        &[NodeId::ComputeBounds]
    }
}
```

## Benefits Summary

```
┌────────────────┬─────────────┬──────────────┐
│    Metric      │   Current   │   Proposed   │
├────────────────┼─────────────┼──────────────┤
│ State Count    │      7      │      3       │
│ Render Passes  │      5      │      3       │
│ Memory Usage   │    22MB     │    15MB      │
│ Code Lines     │    ~500     │    ~200      │
│ CPU Overhead   │    High     │     Low      │
└────────────────┴─────────────┴──────────────┘
```