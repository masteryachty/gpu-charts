# GPU Charts Benchmark Suite

A comprehensive performance benchmarking suite for the GPU Charts system, designed to measure and validate performance against the targets specified in `PERFORMANCE_GUIDE.md`.

## Overview

This benchmark suite tests:
- **Data Loading**: Binary parsing, GPU buffer preparation, caching
- **Rendering**: Vertex generation, culling, LOD, draw calls
- **Memory Usage**: Buffer pools, fragmentation, GPU transfers
- **End-to-End**: Complete pipeline from data to pixels
- **Stress Tests**: Extreme scenarios and sustained load

## Quick Start

```bash
# Run all benchmarks
./run_benchmarks.sh

# Run specific benchmark
cargo bench --bench data_loading

# Run with specific scenario
cargo bench --bench rendering -- line_vertices

# Compare with baseline
cargo bench --bench rendering -- --baseline main
```

## Benchmark Categories

### 1. Data Loading (`data_loading.rs`)
Tests data ingestion and preparation performance:
- Binary to f32 parsing
- Direct GPU buffer creation
- OHLC aggregation
- Min/max calculation
- Cache operations (hit/miss/eviction)
- Zero-copy validation

### 2. Rendering (`rendering.rs`)
Tests GPU rendering performance:
- Line chart vertex generation
- Candlestick vertex generation
- Viewport frustum culling
- Binary search culling
- LOD selection and reduction
- Draw call optimization
- Overlay alpha blending

### 3. Memory Usage (`memory_usage.rs`)
Tests memory efficiency:
- Buffer pool allocation vs direct allocation
- Memory fragmentation patterns
- LRU cache memory usage
- CPU to GPU transfer speeds
- Memory pressure handling
- Zero-copy operations

### 4. End-to-End (`end_to_end.rs`)
Tests complete scenarios:
- Small dataset (1K points)
- Medium dataset (100K points)
- Large dataset (10M points)
- Rapid zoom interactions
- Continuous panning
- Multiple concurrent charts
- Memory pressure scenarios

### 5. Stress Tests (`stress_test.rs`)
Tests extreme conditions:
- 1 billion point simulation
- GPU memory exhaustion
- Cache thrashing
- 50 concurrent charts
- All NaN data handling
- Extreme zoom ranges
- Sustained 60 FPS load

## Performance Targets

Based on `PERFORMANCE_GUIDE.md`:

| Metric | Target | Maximum |
|--------|--------|---------|
| Frame Time | 16ms | 16.67ms |
| GPU Time | 14ms | 15ms |
| CPU Time | 2ms | 5ms |
| Draw Calls | 50 | 100 |

### Data Operation Targets

| Operation | 1M points | 100M points | 1B points |
|-----------|-----------|-------------|-----------|
| Fetch | <20ms | <200ms | <2s |
| Parse | <10ms | <100ms | <1s |
| Aggregate | <5ms | <50ms | <500ms |

## Running Benchmarks

### Full Suite
```bash
./run_benchmarks.sh
```

This will:
1. Run all benchmark categories
2. Generate HTML reports with graphs
3. Save results with timestamp
4. Create system information file
5. Generate summary report
6. Create navigable HTML index

### Individual Benchmarks
```bash
# Data loading only
cargo bench --bench data_loading

# Rendering with custom settings
RUST_LOG=info cargo bench --bench rendering

# Memory usage with profiling
cargo bench --bench memory_usage --features profiling
```

### Continuous Integration
```bash
# CI-friendly mode (fails on regression)
cargo bench -- --save-baseline main
# ... make changes ...
cargo bench -- --baseline main
```

## Output Structure

```
benchmark_results/
└── 20240315_143022/
    ├── index.html          # Main report page
    ├── summary.md          # Executive summary
    ├── system_info.txt     # Hardware/OS details
    ├── data_loading.log    # Benchmark output
    ├── data_loading/       # Criterion HTML report
    │   └── report/
    │       └── index.html
    ├── rendering.log
    ├── rendering/
    │   └── report/
    │       └── index.html
    └── ...
```

## Interpreting Results

### Criterion Reports
Each benchmark generates a detailed HTML report showing:
- **Violin plots**: Distribution of measurements
- **Line charts**: Performance over iterations
- **Comparison**: Changes from baseline
- **Statistics**: Mean, median, std deviation

### Performance Indicators

✅ **Good Performance**
- Frame time consistently under 16ms
- Cache hit rate above 80%
- Linear scaling with data size
- Low memory fragmentation

⚠️ **Warning Signs**
- Frame time spikes above 16ms
- High cache miss rate
- Exponential scaling
- Memory pressure errors

❌ **Performance Issues**
- Frame time above 20ms
- Out of memory errors
- Thrashing behavior
- Failed stress tests

## Benchmark Development

### Adding New Benchmarks

1. Create new scenario in `scenarios.rs`:
```rust
pub enum BenchmarkScenario {
    YourNewScenario { param: usize },
}
```

2. Add benchmark function:
```rust
fn benchmark_your_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("your_feature");
    
    group.bench_function("test_case", |b| {
        b.iter(|| {
            // Your benchmark code
        });
    });
    
    group.finish();
}
```

3. Register in `criterion_group!` macro

### Best Practices

1. **Use realistic data**: Generate data similar to production
2. **Test edge cases**: Empty data, NaN values, extreme sizes
3. **Measure overhead**: Include setup/teardown in measurements
4. **Multiple iterations**: Ensure statistical significance
5. **Warmup runs**: Avoid cold start effects

## Profiling Integration

### With perf (Linux)
```bash
cargo bench --bench rendering -- --profile-time=10
```

### With Instruments (macOS)
```bash
cargo instruments -t "Time Profiler" --bench rendering
```

### GPU Profiling
The benchmarks include GPU timing when available:
- Uses `TIMESTAMP_QUERY` for accurate GPU timings
- Measures individual render passes
- Tracks GPU memory usage

## Continuous Monitoring

### Automated Regression Detection
```yaml
# .github/workflows/benchmark.yml
- name: Run benchmarks
  run: |
    cargo bench -- --save-baseline pr-${{ github.event.number }}
    cargo bench -- --baseline main --failfast
```

### Performance Dashboard
Results can be uploaded to tracking services:
- Criterion.rs integration
- Custom telemetry
- Grafana dashboards

## Troubleshooting

### Common Issues

**"No adapter found"**
- Ensure GPU drivers are installed
- Check WebGPU support
- Try software renderer

**"Out of memory"**
- Reduce stress test parameters
- Increase system RAM
- Check GPU memory limits

**"Benchmark timeout"**
- Reduce iteration count
- Check for infinite loops
- Increase timeout in `Criterion.toml`

## Future Enhancements

- [ ] Network latency simulation
- [ ] WebSocket streaming benchmarks
- [ ] Multi-GPU testing
- [ ] Browser-based benchmarks
- [ ] Real-world data replay
- [ ] Automated performance regression alerts