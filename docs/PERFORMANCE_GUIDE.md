# Performance Optimization Guide

## Overview
This guide details the performance-critical aspects of the GPU Charts architecture and how to maintain peak performance while extending the system.

## Performance Principles

### 1. Zero JS Boundary Crossings
**Critical**: Never pass large datasets through the JavaScript/WASM boundary.

```typescript
// ❌ BAD: Passing data through JS
const data = await fetch('/api/data');
const parsed = parseData(data);
wasmModule.renderData(parsed); // Millions of points crossing boundary!

// ✅ GOOD: Data stays in WASM
const dataHandle = await dataManager.fetchData(request);
renderer.renderWithHandle(dataHandle); // Only handle crosses boundary
```

### 2. GPU-First Architecture
All data processing happens on GPU when possible:
- Data parsing → GPU buffers
- Aggregation → Compute shaders
- Culling → GPU-driven
- Rendering → Direct from GPU buffers

### 3. Memory Hierarchy Optimization
```
Network → WASM Memory → GPU Memory → Render
         ↑______________|
         (Zero intermediate copies)
```

## Critical Performance Paths

### Data Fetching Pipeline
1. **HTTP/2 Stream** → Binary data chunks
2. **Direct Parse** → GPU buffer creation (no intermediate arrays)
3. **Buffer Pool** → Reuse allocations
4. **Cache Hit** → Skip network entirely

### Rendering Pipeline
1. **Viewport Culling** → Reduce vertex count
2. **LOD Selection** → Appropriate detail level
3. **Instancing** → Batch similar elements
4. **Draw Call Minimization** → <100 calls per frame

## Performance Targets

### Data Operations
| Operation | 1M points | 100M points | 1B points |
|-----------|-----------|-------------|-----------|
| Fetch     | <20ms     | <200ms      | <2s       |
| Parse     | <10ms     | <100ms      | <1s       |
| Aggregate | <5ms      | <50ms       | <500ms    |

### Rendering Operations
| Operation | Target | Maximum |
|-----------|--------|---------|
| Frame Time | 16ms | 16.67ms |
| GPU Time | 14ms | 15ms |
| CPU Time | 2ms | 5ms |
| Draw Calls | 50 | 100 |

## Optimization Techniques

### 1. Buffer Management
```rust
// Use buffer pools to avoid allocations
let buffer = self.buffer_pool.acquire(size);
// ... use buffer ...
self.buffer_pool.release(buffer);
```

### 2. Culling Strategies
```rust
// GPU-based frustum culling
pub fn cull_gpu(
    data_buffer: &wgpu::Buffer,
    viewport: &Viewport,
) -> wgpu::Buffer {
    // Compute shader filters visible points
}
```

### 3. Level of Detail
```rust
// Automatic LOD based on zoom level
pub fn select_lod(zoom_level: f32, point_count: u32) -> LODLevel {
    match (zoom_level, point_count) {
        (z, n) if z < 0.1 && n > 1_000_000 => LODLevel::Aggressive,
        (z, n) if z < 0.5 && n > 100_000 => LODLevel::Moderate,
        _ => LODLevel::Full,
    }
}
```

### 4. Aggregation Strategies
For time-series data at different zoom levels:
- **Zoomed out**: Show aggregated min/max/avg
- **Mid zoom**: Show OHLC candles
- **Zoomed in**: Show individual points

## Memory Management

### GPU Memory Budget
```rust
const MAX_GPU_MEMORY: usize = 2 * 1024 * 1024 * 1024; // 2GB
const BUFFER_POOL_SIZE: usize = 512 * 1024 * 1024; // 512MB
```

### Cache Eviction Strategy
```rust
impl DataCache {
    fn evict_if_needed(&mut self, required_size: usize) {
        while self.total_size + required_size > MAX_CACHE_SIZE {
            self.evict_lru();
        }
    }
}
```

## Profiling & Monitoring

### Key Metrics to Track
1. **Frame Time Distribution** - p50, p95, p99
2. **GPU Memory Usage** - Current, peak, available
3. **Cache Hit Rate** - Should be >80%
4. **Network Utilization** - Bandwidth efficiency
5. **Draw Call Count** - Per frame

### Performance Debugging
```typescript
// Enable performance monitoring
const chart = new ChartSystem({
    performance: {
        gpuTiming: true,
        memoryTracking: true,
        networkStats: true,
    }
});

chart.on('performanceReport', (report) => {
    console.log(`Frame time: ${report.frameTime}ms`);
    console.log(`GPU memory: ${report.gpuMemory}MB`);
});
```

## Common Performance Pitfalls

### 1. Unnecessary Data Fetching
```typescript
// ❌ BAD: Fetching all columns
const data = await fetchData(['*']);

// ✅ GOOD: Fetch only needed columns
const data = await fetchData(['time', 'price']);
```

### 2. Render State Thrashing
```typescript
// ❌ BAD: Multiple config updates
chart.setChartType('line');
chart.setTimeRange(start, end);
chart.setOverlays(['volume']);

// ✅ GOOD: Batch updates
chart.updateConfig({
    chartType: 'line',
    timeRange: { start, end },
    overlays: ['volume']
});
```

### 3. Memory Leaks
```rust
// ❌ BAD: Forgetting to clean up GPU resources
let buffer = device.create_buffer(&desc);
// ... buffer never destroyed

// ✅ GOOD: Proper cleanup
let buffer = device.create_buffer(&desc);
defer! { buffer.destroy(); }
```

## Platform-Specific Optimizations

### Desktop (High-end GPU)
- Enable all quality features
- Use larger buffer pools
- More aggressive caching
- Higher LOD thresholds

### Mobile (Limited GPU)
- Reduce buffer pool size
- More aggressive LOD
- Limit overlay count
- Simplify shaders

### WebGPU Limits
```typescript
const limits = {
    maxBufferSize: 256 * 1024 * 1024, // 256MB
    maxTextureDimension2D: 8192,
    maxComputeWorkgroupsPerDimension: 65535,
};
```

## Testing Performance

### Benchmark Suite
```bash
# Run performance benchmarks
npm run bench

# Specific scenarios
npm run bench:billion-points
npm run bench:rapid-zoom
npm run bench:memory-pressure
```

### Load Testing
```typescript
// Simulate extreme usage
async function stressTest() {
    const charts = [];
    for (let i = 0; i < 10; i++) {
        charts.push(new ChartSystem());
        await charts[i].load({
            symbol: 'BTC-USD',
            points: 100_000_000,
        });
    }
    // Monitor memory and performance
}
```

## Future Optimizations

### Under Consideration
1. **WebGPU Compute Graphs** - Chain operations without CPU roundtrips
2. **Mesh Shaders** - When widely supported
3. **Variable Rate Shading** - Reduce shader cost in less important areas
4. **Temporal Upsampling** - Reuse previous frame data
5. **GPU-Driven Rendering** - Move more logic to GPU

Remember: **Measure first, optimize second**. Use profiling data to guide optimization efforts.