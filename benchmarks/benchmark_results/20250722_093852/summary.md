# GPU Charts Benchmark Report

**Date**: Tue Jul 22 09:38:53 BST 2025
**System**: xander-win-0

## Summary

Total benchmarks run: 5
Failed benchmarks: 0

## Results

### Data Loading Performance
- Binary parsing throughput
- GPU buffer preparation
- Cache operations
- Data validation

### Rendering Performance
- Vertex generation speed
- Culling efficiency
- LOD selection
- Draw call optimization
- Overlay composition

### Memory Usage
- Buffer pool efficiency
- Memory fragmentation
- GPU memory transfer
- Zero-copy operations

### End-to-End Scenarios
- Small dataset (1K points)
- Medium dataset (100K points)
- Large dataset (10M points)
- Interactive scenarios (zoom/pan)

### Stress Tests
- Billion point simulation
- Memory limit testing
- 50 concurrent charts
- Sustained 60 FPS load

## Performance vs Targets

Target metrics from PERFORMANCE_GUIDE.md:
- Frame time: <16ms (60 FPS)
- GPU time: <14ms
- CPU time: <5ms
- Draw calls: <100

See individual benchmark reports for detailed results.

## Recommendations

Based on the benchmark results, consider:
1. Optimizing any operations that exceed target times
2. Investigating memory usage patterns
3. Improving cache hit rates
4. Reducing draw call counts where possible

