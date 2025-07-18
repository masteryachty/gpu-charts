# Candlestick Chart Performance Improvements

## Summary

I've implemented the two key performance improvements requested in the MR feedback:

### 1. OHLC Aggregation Caching

**Problem**: The OHLC aggregation was recreating all candles on every render.

**Solution**: Implemented a caching mechanism with `CacheKey` that tracks:
- Time range (`start_x`, `end_x`)
- Candle timeframe
- Data hash (to detect when underlying data changes)

**Implementation**:
```rust
#[derive(PartialEq, Clone, Debug)]
struct CacheKey {
    start_x: u32,
    end_x: u32,
    candle_timeframe: u32,
    data_hash: u64,
}
```

The cache is invalidated only when:
- Time range changes
- Candle timeframe changes
- Underlying data changes (detected via hash)

### 2. Indexed Rendering for Memory Efficiency

**Problem**: Vertex data was duplicated (20 bytes per vertex × 6 vertices per body × 4 vertices per wick).

**Solution**: Implemented indexed rendering using index buffers:
- Bodies: 4 unique vertices with 6 indices (2 triangles)
- Wicks: 4 unique vertices with 4 indices (2 lines)

**Memory Savings Calculation**:
```
Non-indexed (per candle):
- Body: 6 vertices × 20 bytes = 120 bytes
- Wick: 4 vertices × 20 bytes = 80 bytes
- Total: 200 bytes per candle

Indexed (per candle):
- Body vertices: 4 × 20 bytes = 80 bytes
- Body indices: 6 × 2 bytes = 12 bytes
- Wick vertices: 4 × 20 bytes = 80 bytes
- Wick indices: 4 × 2 bytes = 8 bytes
- Total: 180 bytes per candle

Memory savings: 20 bytes per candle (10% reduction)
```

For a dataset with 1,000 candles:
- Non-indexed: 200,000 bytes
- Indexed: 180,000 bytes
- **Savings: 20,000 bytes (10%)**

Note: The actual savings may be higher when considering GPU memory alignment and buffer overhead.

## Key Changes

1. **Added caching fields** to `CandlestickRenderer`:
   - `cache_key: Option<CacheKey>` for tracking cache state
   - `body_index_buffer` and `wick_index_buffer` for indexed rendering

2. **Updated `create_vertex_buffers`** to generate index buffers:
   - Creates unique vertices only (4 per body, 4 per wick)
   - Generates index arrays for triangle and line topology

3. **Modified render method** to use `draw_indexed`:
   - `render_pass.draw_indexed(0..self.body_index_count, 0, 0..1)`

4. **Added data hashing** to detect when underlying data changes

## Benefits

1. **Performance**: Aggregation only happens when data or view actually changes
2. **Memory**: ~10% reduction in GPU memory usage
3. **Scalability**: More efficient for large datasets
4. **Cache-friendly**: Reduces redundant calculations

The improvements maintain backward compatibility while providing better performance for real-time financial data visualization.