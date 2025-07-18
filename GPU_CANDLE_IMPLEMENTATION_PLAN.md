# GPU-Based Candle Aggregation Implementation Plan

## Overview
This document outlines the detailed implementation plan for replacing the CPU-based OHLC candle aggregation in `CandlestickRenderer` with a GPU-accelerated compute shader implementation.

## Architecture Summary
- **Current**: CPU-based aggregation using binary search and sequential processing
- **New**: GPU compute shader with parallel workgroups, each processing one candle
- **Integration**: Direct replacement of `aggregate_ohlc` method in `CandlestickRenderer`

## Implementation Tasks

### Task 1: Create GPU Compute Infrastructure
**Files to create:**
- `charting/src/calcables/candle_aggregator.rs`
- `charting/src/calcables/candle_aggregation.wgsl`

#### 1.1 Create Compute Shader (`candle_aggregation.wgsl`)
**Location**: `charting/src/calcables/candle_aggregation.wgsl`

**Implementation details:**
```wgsl
// Structure definitions
struct CandleParams {
    start_timestamp: u32,      // First candle's start time
    candle_timeframe: u32,     // Duration of each candle in seconds
    num_candles: u32,          // Total number of candles to generate
    tick_count: u32,           // Total number of input ticks
}

struct OhlcCandle {
    timestamp: u32,    // Candle start time
    open: f32,         // Opening price
    high: f32,         // Highest price
    low: f32,          // Lowest price
    close: f32,        // Closing price
}

// Bind group 0 layout:
// binding 0: timestamps buffer (storage, read)
// binding 1: prices buffer (storage, read)
// binding 2: output candles buffer (storage, read_write)
// binding 3: parameters uniform buffer
```

**Key features:**
- Workgroup size of 64 threads (optimal for most GPUs)
- Each workgroup processes one complete candle
- Parallel search through tick data
- Shared memory reduction for OHLC calculation
- Handle empty candles gracefully

#### 1.2 Create Rust Compute Module (`candle_aggregator.rs`)
**Location**: `charting/src/calcables/candle_aggregator.rs`

**Structure:**
```rust
pub struct CandleAggregator {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    // Cached buffers for reuse
    params_buffer: Option<wgpu::Buffer>,
    output_buffer: Option<wgpu::Buffer>,
    last_num_candles: u32,
}

impl CandleAggregator {
    pub fn new(device: &wgpu::Device) -> Self
    
    pub fn aggregate_candles(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        timestamps_buffer: &wgpu::Buffer,
        prices_buffer: &wgpu::Buffer,
        tick_count: u32,
        start_timestamp: u32,
        candle_timeframe: u32,
        num_candles: u32,
    ) -> wgpu::Buffer
}
```

**Implementation considerations:**
- Buffer caching to avoid recreation
- Efficient parameter updates
- Direct GPU buffer output (no CPU readback needed)
- Integration with existing render pipeline

### Task 2: Update Module Exports
**File**: `charting/src/calcables/mod.rs`

**Changes needed:**
1. Add `mod candle_aggregator;`
2. Export `pub use candle_aggregator::CandleAggregator;`
3. Ensure `OhlcData` struct is accessible

### Task 3: Integrate into CandlestickRenderer
**File**: `charting/src/drawables/candlestick.rs`

#### 3.1 Add CandleAggregator Field
```rust
pub struct CandlestickRenderer {
    // Existing fields...
    candle_aggregator: CandleAggregator,  // NEW
    gpu_output_buffer: Option<wgpu::Buffer>,  // NEW
}
```

#### 3.2 Update Constructor
In `CandlestickRenderer::new()`:
```rust
let candle_aggregator = CandleAggregator::new(device);
```

#### 3.3 Replace aggregate_ohlc Method
**Current method signature:**
```rust
fn aggregate_ohlc(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, ds: &DataStore)
```

**New implementation steps:**
1. Calculate candle boundaries (keep existing logic)
2. Get GPU buffers from DataStore instead of ArrayBuffers
3. Call GPU aggregator
4. Create vertex buffers directly from GPU output
5. Update rendering to use new buffers

**Detailed changes:**
```rust
fn aggregate_ohlc(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, ds: &DataStore) {
    // Step 1: Calculate boundaries (reuse existing code)
    let first_candle_start = (ds.start_x / self.candle_timeframe) * self.candle_timeframe;
    let last_candle_end = ds.end_x.div_ceil(self.candle_timeframe) * self.candle_timeframe;
    let num_candles = ((last_candle_end - first_candle_start) / self.candle_timeframe) as u32;
    
    // Step 2: Get GPU buffers from DataStore
    let active_groups = ds.get_active_data_groups();
    if active_groups.is_empty() { return; }
    
    let data_series = &active_groups[0];
    if data_series.metrics.is_empty() { return; }
    
    // Access GPU buffers directly
    let time_buffers = &data_series.x_buffers;
    let price_buffers = &data_series.metrics[0].y_buffers;
    
    if time_buffers.is_empty() || price_buffers.is_empty() { return; }
    
    // Step 3: Create command encoder if needed
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Candle Aggregation"),
    });
    
    // Step 4: Run GPU aggregation
    // Handle multiple buffer chunks if necessary
    let tick_count = data_series.length;
    
    self.gpu_output_buffer = Some(
        self.candle_aggregator.aggregate_candles(
            device,
            queue,
            &mut encoder,
            &time_buffers[0],  // TODO: Handle multiple chunks
            &price_buffers[0], // TODO: Handle multiple chunks
            tick_count,
            first_candle_start,
            self.candle_timeframe,
            num_candles,
        )
    );
    
    // Step 5: Submit GPU work
    queue.submit(Some(encoder.finish()));
    
    // Step 6: Create vertex buffers from GPU output
    self.create_vertex_buffers_from_gpu(device, num_candles);
}
```

### Task 4: Create Vertex Buffer Generation
**New method in `candlestick.rs`:**

```rust
fn create_vertex_buffers_from_gpu(&mut self, device: &wgpu::Device, num_candles: u32) {
    // Create transformation compute shader to convert 
    // OhlcCandle format to vertex format
    // OR modify the candle shader to output vertex-ready data
}
```

**Options:**
1. Add a second compute pass to transform candle data to vertices
2. Modify the aggregation shader to output vertex-ready data
3. Modify the vertex shader to accept candle data directly

**Recommendation**: Option 3 - Modify vertex shader to read candle data directly

### Task 5: Update Vertex Shaders
**File**: `charting/src/drawables/candlestick.wgsl`

**Changes needed:**
1. Add storage buffer binding for candle data
2. Modify `vs_body` to read from candle buffer
3. Modify `vs_wick` to read from candle buffer
4. Calculate vertex positions based on vertex ID and candle data

**Example vertex shader modification:**
```wgsl
@group(0) @binding(3)
var<storage, read> candles: array<OhlcCandle>;

@vertex
fn vs_body(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    let candle_idx = vertex_idx / 4u;
    let corner_idx = vertex_idx % 4u;
    
    let candle = candles[candle_idx];
    
    // Calculate body rectangle corners
    // corner_idx: 0=bottom-left, 1=bottom-right, 2=top-right, 3=top-left
    var x_pos: f32;
    var y_pos: f32;
    
    // Calculate positions based on corner...
}
```

### Task 6: Handle Multiple Buffer Chunks
**Challenge**: DataStore uses chunked buffers for large datasets

**Solution approaches:**
1. Run compute shader multiple times, once per chunk
2. Concatenate chunks before processing
3. Modify compute shader to handle multiple input buffers

**Implementation steps:**
1. Check if data spans multiple chunks
2. If single chunk: use existing approach
3. If multiple chunks: 
   - Option A: Process each chunk separately and merge results
   - Option B: Create a larger buffer and copy all chunks

### Task 7: Performance Optimizations

#### 7.1 Buffer Caching Strategy
- Cache output buffer when num_candles doesn't change
- Cache params buffer and only update contents
- Reuse bind groups when possible

#### 7.2 Empty Candle Handling
- Add early termination in compute shader
- Use previous close price for empty candles
- Consider sparse representation

#### 7.3 Multi-Metric Support
- Process multiple price series in single compute pass
- Use array bindings for multiple buffers
- Output multiple OHLC series

### Task 8: Testing and Validation

#### 8.1 Unit Tests
Create test in `charting/src/calcables/candle_aggregator.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_candle_aggregation_accuracy() {
        // Compare GPU results with CPU implementation
    }
    
    #[test]
    fn test_empty_candles() {
        // Test handling of time periods with no data
    }
    
    #[test]
    fn test_partial_candles() {
        // Test candles at view boundaries
    }
}
```

#### 8.2 Integration Tests
- Verify visual output matches CPU implementation
- Test with various timeframes (1m, 5m, 15m, 1h, etc.)
- Test with real market data

#### 8.3 Performance Benchmarks
- Add timing measurements
- Compare with CPU implementation
- Test with various data sizes

### Task 9: Error Handling

#### 9.1 GPU Resource Errors
- Handle buffer creation failures
- Check for GPU memory limits
- Provide CPU fallback option

#### 9.2 Data Validation
- Verify timestamp ordering
- Handle NaN/Inf price values
- Check buffer size compatibility

### Task 10: Documentation

#### 10.1 Code Documentation
- Document compute shader algorithm
- Explain workgroup/thread organization
- Document buffer layouts

#### 10.2 Update CLAUDE.md
- Add GPU candle aggregation to architecture section
- Document performance characteristics
- Add troubleshooting guide

## Implementation Order

### Phase 1: Core Implementation (Tasks 1-3)
1. Create compute shader file
2. Create Rust aggregator module
3. Basic integration into CandlestickRenderer

### Phase 2: Rendering Integration (Tasks 4-5)
1. Update vertex buffer creation
2. Modify vertex shaders
3. Test basic rendering

### Phase 3: Robustness (Tasks 6-7)
1. Handle multiple buffer chunks
2. Implement buffer caching
3. Add multi-metric support

### Phase 4: Validation (Tasks 8-10)
1. Add comprehensive tests
2. Performance benchmarking
3. Documentation updates

## Risk Mitigation

### Technical Risks
1. **GPU Memory Limits**: Large datasets might exceed GPU memory
   - Mitigation: Process in smaller batches
   
2. **Shader Compilation**: Different GPUs might have issues
   - Mitigation: Test on various hardware
   
3. **Precision Loss**: Float32 might not match CPU double precision
   - Mitigation: Use consistent float32 throughout

### Integration Risks
1. **Breaking Changes**: Might affect existing functionality
   - Mitigation: Extensive testing, gradual rollout
   
2. **Performance Regression**: GPU might be slower for small datasets
   - Mitigation: Add heuristic to choose CPU vs GPU

## Success Metrics
- 10x+ performance improvement for datasets > 100k ticks
- Identical visual output to CPU implementation
- No memory leaks or GPU resource exhaustion
- Maintains 60 FPS during real-time updates