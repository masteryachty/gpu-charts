# Web Integration Plan - Replace Charting with New Architecture

## Overview
Replace the legacy charting crate with the new architecture (data-manager, renderer, wasm-bridge) in the web application, ensuring all features are preserved or improved.

## Current State Analysis

### Charting Crate Features
1. **Line Graph Rendering** - Basic line charts with WebGPU
2. **Data Fetching** - HTTP data retrieval from server
3. **User Interactions** - Zoom, pan, mouse controls
4. **Axis Rendering** - X/Y axes with labels
5. **Min/Max Calculations** - GPU compute shaders
6. **React Integration** - Chart class with event handlers

### New Architecture Coverage
1. **data-manager** ✅
   - HTTP data fetching (better with HTTP/2)
   - WebSocket support (real-time data)
   - Binary data parsing
   - GPU buffer management
   - SIMD optimizations

2. **renderer** ✅
   - Advanced line charts, candlesticks, bars, area charts
   - Binary culling (25,000x performance)
   - Vertex compression (<8 bytes)
   - Multi-resolution rendering
   - Better axis rendering

3. **wasm-bridge** ✅
   - Clean React integration
   - Event handling
   - Configuration management
   - Performance monitoring

## Implementation Plan

### Phase 1: Build and Prepare (30 mins)
1. **Build wasm-bridge for web**
   ```bash
   cd crates/wasm-bridge
   wasm-pack build --target web --out-dir ../../web/pkg-new
   ```

2. **Create TypeScript definitions**
   - Generate .d.ts files from wasm-bridge
   - Document API methods

3. **Verify all dependencies**
   - Ensure data-manager WASM features enabled
   - Ensure renderer WASM compatible
   - Check config-system integration

### Phase 2: Update React Integration (1 hour)
1. **Update Chart Component**
   ```typescript
   // Before: import { Chart } from '../pkg/GPU_charting';
   // After: import { ChartSystem } from '../pkg-new/gpu_charts_wasm_bridge';
   ```

2. **Update initialization**
   ```typescript
   // New initialization
   const chart = await ChartSystem.new(canvasId, apiBaseUrl);
   ```

3. **Update event handlers**
   - Map existing mouse events to new API
   - Update resize handling
   - Update data update calls

4. **Update configuration**
   - Use new config system
   - Set quality presets
   - Enable Phase 3 features

### Phase 3: Feature Mapping (1 hour)
1. **Data Fetching**
   - Old: `chart.init()` with URL params
   - New: `chart.update_chart(chartType, symbol, startTime, endTime)`

2. **Rendering**
   - Old: Automatic render loop
   - New: `chart.render()` with React's animation frame

3. **User Interactions**
   - Old: Individual mouse handlers
   - New: Unified event system (may need to add to wasm-bridge)

4. **Performance Monitoring**
   - Old: None
   - New: `chart.get_stats()` for detailed metrics

### Phase 4: Testing (30 mins)
1. **Functionality Tests**
   - Line chart rendering
   - Data loading from API
   - Zoom/pan interactions
   - Axis label rendering
   - Window resizing

2. **Performance Tests**
   - Compare render times
   - Check memory usage
   - Verify GPU optimizations active

3. **Edge Cases**
   - Large datasets
   - Rapid interactions
   - Network errors
   - WebGPU fallbacks

### Phase 5: Cleanup (30 mins)
1. **Remove charting crate**
   - Remove from workspace Cargo.toml
   - Delete charting directory
   - Update package.json scripts

2. **Update build scripts**
   - Remove old wasm build commands
   - Update to use new architecture

3. **Update documentation**
   - Update README
   - Update CLAUDE.md files
   - Document new API

## Migration Checklist

### Required Features
- [ ] Line chart rendering
- [ ] HTTP data fetching
- [ ] Mouse wheel zoom
- [ ] Click and drag pan
- [ ] X-axis time labels
- [ ] Y-axis value labels
- [ ] Responsive canvas sizing
- [ ] URL parameter support

### New Features Available
- [ ] Candlestick charts
- [ ] Bar charts
- [ ] Area charts
- [ ] 25,000x faster culling
- [ ] Vertex compression
- [ ] Configuration hot-reload
- [ ] Performance metrics
- [ ] Real-time WebSocket data

## Risk Mitigation

### Potential Issues
1. **API Differences**
   - Solution: Create adapter layer if needed
   - Or: Update wasm-bridge to match expected API

2. **Missing Features**
   - Solution: Add to wasm-bridge as needed
   - Most features already superior in new architecture

3. **Build Issues**
   - Solution: Ensure clean build environment
   - Test incremental migration if needed

## Success Criteria
1. Web app loads and renders charts
2. All user interactions work
3. Performance equal or better
4. No console errors
5. Clean build with no warnings
6. Charting crate fully removed

## Estimated Time: 3-4 hours total