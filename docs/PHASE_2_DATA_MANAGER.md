# Phase 2: Data Manager Implementation

## Overview
Build the high-performance data management module that handles fetching, parsing, and GPU buffer creation with zero JS boundary crossings.

## Duration: 4-5 days

## Tasks

### 2.1 Core Data Manager Structure
- [ ] Implement DataManager struct with caching
  ```rust
  pub struct DataManager {
      device: Arc<wgpu::Device>,
      queue: Arc<wgpu::Queue>,
      cache: Arc<RwLock<DataCache>>,
      http_client: HttpClient,
      buffer_pool: BufferPool,
  }
  ```
- [ ] Design cache key strategy for efficient lookups
- [ ] Implement LRU cache with configurable memory limit
- [ ] Add cache statistics and monitoring

### 2.2 High-Performance HTTP Client
- [ ] Implement HTTP/2 client with connection pooling
- [ ] Add request pipelining for parallel fetches
- [ ] Implement progressive data streaming
- [ ] Add compression support (gzip, brotli)
- [ ] Configure for minimum latency
  - Keep-alive connections
  - TCP_NODELAY
  - Optimized buffer sizes

### 2.3 Zero-Copy Binary Parser
- [ ] Implement direct binary-to-GPU parsing
  ```rust
  pub fn parse_to_gpu(
      binary_data: &[u8],
      device: &wgpu::Device,
      queue: &wgpu::Queue,
  ) -> Result<GpuBufferSet, ParseError> {
      // Parse header
      // Allocate GPU buffers
      // Copy data directly to GPU without intermediate allocations
  }
  ```
- [ ] Add SIMD optimizations for data transformation
- [ ] Implement chunked parsing for large datasets
- [ ] Add data validation without copying
- [ ] Benchmark against current implementation

### 2.4 GPU Buffer Management
- [ ] Implement buffer pooling to reduce allocations
- [ ] Add buffer coalescing for small datasets
- [ ] Create efficient buffer layout for GPU access
- [ ] Implement buffer usage tracking
- [ ] Add automatic buffer cleanup on memory pressure

### 2.5 Data Request API
- [ ] Define data request interface
  ```rust
  pub struct DataRequest {
      pub symbol: String,
      pub time_range: TimeRange,
      pub columns: Vec<String>,
      pub aggregation: Option<AggregationConfig>,
      pub max_points: Option<u32>,
  }
  ```
- [ ] Implement intelligent data decimation
- [ ] Add request batching for efficiency
- [ ] Implement request prioritization
- [ ] Add cancellation support

### 2.6 WASM Bridge for Data Manager
- [ ] Create WASM bindings for DataManager
- [ ] Implement handle-based API (no data crossing JS boundary)
- [ ] Add async/await support
- [ ] Create TypeScript types
- [ ] Add progress callbacks for long operations

## Performance Optimizations

### 2.7 Advanced Optimizations
- [ ] Implement prefetching based on user patterns
- [ ] Add speculative caching for pan/zoom
- [ ] Use memory-mapped files where supported
- [ ] Implement delta compression for updates
- [ ] Add GPU buffer suballocation for small datasets

### 2.8 Aggregation Engine
- [ ] GPU-based OHLC aggregation
  ```rust
  pub fn aggregate_ohlc_gpu(
      timestamps: &wgpu::Buffer,
      prices: &wgpu::Buffer,
      timeframe: u32,
  ) -> wgpu::Buffer {
      // Compute shader for parallel aggregation
  }
  ```
- [ ] Multi-resolution data pyramids
- [ ] Intelligent level-of-detail selection
- [ ] Cached aggregation results

## Performance Checkpoints

### Data Fetching
- [ ] 100MB data fetched and parsed in <100ms
- [ ] 1GB data fetched and parsed in <1s
- [ ] HTTP/2 multiplexing reducing latency by >50%
- [ ] Network utilization >90% of available bandwidth

### Memory Efficiency
- [ ] Zero intermediate allocations for data parsing
- [ ] GPU buffer pool hit rate >95%
- [ ] Memory usage within 1.2x of raw data size
- [ ] Cache efficiency >80% for typical usage

### GPU Performance
- [ ] Direct GPU upload bandwidth >10GB/s
- [ ] OHLC aggregation processing >1B points/second
- [ ] Buffer allocation time <1ms for pooled buffers

## Success Criteria
- [ ] Data manager module fully implemented
- [ ] All performance targets met or exceeded
- [ ] Zero JS boundary crossings verified
- [ ] Comprehensive test coverage (>90%)
- [ ] Memory leaks verified absent

## Integration Tests
- [ ] Test with 1B+ point datasets
- [ ] Verify cache behavior under memory pressure
- [ ] Test concurrent requests handling
- [ ] Verify error handling and recovery
- [ ] Test WebGPU buffer lifecycle

## Risks & Mitigations
- **Risk**: WebGPU buffer size limitations
  - **Mitigation**: Automatic chunking for large datasets
- **Risk**: Memory pressure on mobile devices
  - **Mitigation**: Adaptive cache sizing based on available memory
- **Risk**: Network latency affecting UX
  - **Mitigation**: Progressive loading with early partial renders

## Dependencies
- reqwest with HTTP/2 support
- tokio for async runtime
- bytemuck for zero-copy parsing
- lru for cache implementation
- wgpu for GPU buffer management

## Next Phase
[Phase 3: Renderer Refactor](./PHASE_3_RENDERER.md) - Refactor renderer to be configuration-driven