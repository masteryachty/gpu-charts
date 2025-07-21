# Phase 3: Renderer Refactor

## Overview
Transform the existing renderer into a pure, configuration-driven rendering engine that accepts GPU buffers from the data manager.

## Duration: 5-6 days

## Tasks

### 3.1 Renderer Architecture Refactor
- [ ] Extract renderer into separate crate
- [ ] Remove all data fetching logic
- [ ] Create clean renderer interface
  ```rust
  pub struct Renderer {
      engine: RenderEngine,
      chart_renderer: Box<dyn ChartRenderer>,
      overlay_renderers: Vec<Box<dyn OverlayRenderer>>,
      config: RenderConfiguration,
  }
  
  pub trait ChartRenderer {
      fn render(&mut self, buffers: &GpuBufferSet, pass: &mut RenderPass);
      fn update_config(&mut self, config: &ChartConfig);
  }
  ```
- [ ] Implement configuration hot-reloading
- [ ] Add renderer performance metrics

### 3.2 GPU Pipeline Optimization
- [ ] Optimize shader compilation and caching
- [ ] Implement pipeline state object (PSO) caching
- [ ] Add GPU timing queries for profiling
- [ ] Optimize buffer binding for minimum state changes
- [ ] Implement instanced rendering where applicable

### 3.3 Chart Renderer Implementations
- [ ] Refactor LineChartRenderer
  - Remove data-specific logic
  - Accept generic buffer configuration
  - Optimize for large point counts
- [ ] Refactor CandlestickRenderer
  - Configuration-driven timeframes
  - Efficient GPU-based rendering
  - LOD system for many candles
- [ ] Create AreaChartRenderer
- [ ] Create BarChartRenderer

### 3.4 Rendering Optimization
- [ ] Implement viewport culling
  ```rust
  pub fn cull_to_viewport(
      data_range: &DataRange,
      viewport: &Viewport,
  ) -> Option<RenderRange> {
      // GPU-based culling for billions of points
  }
  ```
- [ ] Add level-of-detail (LOD) system
- [ ] Implement render batching
- [ ] Add occlusion culling for overlapping data
- [ ] Optimize draw call submission

### 3.5 Overlay System
- [ ] Design overlay renderer interface
  ```rust
  pub trait OverlayRenderer {
      fn render(&mut self, context: &RenderContext, pass: &mut RenderPass);
      fn requires_own_pass(&self) -> bool;
      fn render_location(&self) -> RenderLocation;
  }
  ```
- [ ] Implement volume overlay
- [ ] Implement moving average overlay
- [ ] Add overlay composition system
- [ ] Support sub-chart rendering

### 3.6 Render Configuration System
- [ ] Define comprehensive render configuration
  ```rust
  pub struct RenderConfiguration {
      pub chart_type: ChartType,
      pub data_mapping: DataMapping,
      pub visual_style: VisualStyle,
      pub overlays: Vec<OverlayConfig>,
      pub performance_hints: PerformanceHints,
  }
  ```
- [ ] Implement configuration validation
- [ ] Add configuration diffing for efficient updates
- [ ] Create configuration presets

## Performance Optimizations

### 3.7 Advanced GPU Techniques
- [ ] Implement GPU-driven rendering
  - Indirect draw calls
  - GPU-based LOD selection
  - Compute shader culling
- [ ] Add multi-resolution rendering
- [ ] Implement temporal upsampling
- [ ] Use render bundles for static content

### 3.8 Memory and Bandwidth Optimization
- [ ] Implement vertex compression
- [ ] Use indexed rendering where beneficial
- [ ] Optimize vertex formats
- [ ] Add vertex buffer streaming
- [ ] Implement render target pooling

## Performance Checkpoints

### Rendering Performance
- [ ] 1B points rendered at 60 FPS
- [ ] <16ms frame time with complex overlays
- [ ] GPU utilization >90% (no CPU bottlenecks)
- [ ] Draw call count <100 for any chart

### Memory Efficiency
- [ ] Vertex buffer size optimized (<8 bytes per point)
- [ ] Render target memory <100MB
- [ ] Shader memory <10MB
- [ ] Zero memory allocations during render

### Scalability
- [ ] Linear performance scaling with point count
- [ ] Consistent frame time regardless of zoom level
- [ ] Smooth interaction with 10+ overlays

## Success Criteria
- [ ] Renderer fully separated from data logic
- [ ] All chart types converted to new system
- [ ] Performance targets achieved
- [ ] Overlay system implemented
- [ ] Configuration-driven rendering working

## Visual Quality Tests
- [ ] Pixel-perfect rendering at all zoom levels
- [ ] Smooth antialiasing for all chart types
- [ ] No rendering artifacts
- [ ] Consistent visual style
- [ ] Proper overlay composition

## Integration Requirements
- [ ] Accepts buffers from data manager
- [ ] Configuration updates without re-render
- [ ] Proper error handling
- [ ] Performance monitoring API
- [ ] Debug visualization modes

## Risks & Mitigations
- **Risk**: GPU driver compatibility issues
  - **Mitigation**: Multiple render paths, capability detection
- **Risk**: Performance regression from abstraction
  - **Mitigation**: Continuous benchmarking, optimization passes
- **Risk**: Visual quality degradation
  - **Mitigation**: Automated visual regression tests

## Next Phase
[Phase 4: Configuration Layer](./PHASE_4_CONFIGURATION.md) - Build the intelligent configuration system