# Data Manager Crate - CLAUDE.md

This file provides comprehensive guidance for working with the data-manager crate, which handles all data operations including fetching, parsing, caching, GPU buffer management, and coordinate transformations for the GPU Charts visualization system.

## Purpose and Responsibility

The data-manager crate is the central data orchestration layer responsible for:
- **Data Acquisition**: Fetching time-series data from HTTP/HTTPS API endpoints
- **Data Parsing**: Processing server responses with JSON headers and binary payloads
- **GPU Buffer Management**: Creating and managing WebGPU buffers for high-performance rendering
- **Caching**: LRU cache implementation for minimizing network requests
- **Data Store Management**: Maintaining chart data state and coordinate transformations
- **Computed Metrics**: Supporting derived metrics through GPU compute operations
- **Multi-Type Data Handling**: Managing multiple data types (market data, trades) simultaneously

## Architecture Position

```
shared-types (foundation types)
    ↑
├── config-system (presets and configuration)
│   ↑
└── data-manager (this crate)
    ↑
    wasm-bridge (orchestration layer)
```

## Core Components

### DataManager (`src/lib.rs`)

The main orchestration component that coordinates all data operations:

```rust
pub struct DataManager {
    device: Rc<Device>,           // WebGPU device for buffer creation
    base_url: String,              // API endpoint base URL
    cache: DataCache,              // LRU cache for data handles
    active_handles: HashMap<Uuid, GpuBufferSet>, // Active GPU buffers
}
```

**Key Methods:**
- `fetch_data()`: Fetches data from API and creates GPU buffers
- `fetch_data_for_preset()`: Loads all data required by a preset configuration
- `process_data_handle()`: Processes fetched data into the DataStore
- `create_computed_metrics_for_preset()`: Creates derived metrics based on preset definitions
- `get_buffers()`: Retrieves GPU buffers for a data handle
- `update_cache_size()`: Dynamically adjusts cache capacity
- `clear_cache()`: Clears all cached data

### DataStore (`src/data_store.rs`)

Manages the actual chart data and coordinate transformations:

```rust
pub struct DataStore {
    pub preset: Option<ChartPreset>,        // Active chart preset
    pub symbol: Option<String>,             // Trading symbol
    pub start_x: u32,                       // Time range start
    pub end_x: u32,                         // Time range end
    pub data_groups: Vec<DataSeries>,       // Multiple data series
    pub active_data_group_indices: Vec<usize>, // Active series indices
    pub range_bind_group: Option<wgpu::BindGroup>, // GPU bind group
    pub screen_size: ScreenDimensions,      // Viewport dimensions
    dirty: bool,                            // Re-render flag
    pub min_max_buffer: Option<Rc<wgpu::Buffer>>, // GPU-calculated bounds
    pub gpu_min_y: Option<f32>,             // GPU-calculated min Y
    pub gpu_max_y: Option<f32>,             // GPU-calculated max Y
}
```

**Key Features:**
- **Dirty Flag Pattern**: Tracks when data changes require re-rendering
- **GPU Bounds**: Stores GPU-calculated min/max values for efficient rendering
- **Multi-Series Support**: Handles multiple data series with shared time axes
- **Computed Metrics**: Supports derived metrics with dependency tracking
- **Coordinate Transformation**: World-to-screen and screen-to-world conversions

### DataSeries and MetricSeries

```rust
pub struct DataSeries {
    pub x_buffers: Vec<wgpu::Buffer>,  // Time axis buffers (chunked)
    pub x_raw: ArrayBuffer,             // Raw time data for CPU access
    pub metrics: Vec<MetricSeries>,     // Y-axis metrics
    pub length: u32,                    // Number of data points
}

pub struct MetricSeries {
    pub y_buffers: Vec<wgpu::Buffer>,   // Value buffers (chunked)
    pub y_raw: ArrayBuffer,              // Raw value data
    pub color: [f32; 3],                // RGB color
    pub visible: bool,                  // Visibility flag
    pub name: String,                    // Metric name
    pub is_computed: bool,               // Computed metric flag
    pub compute_type: Option<ComputeOp>, // Computation operation
    pub dependencies: Vec<MetricRef>,    // Dependencies for computed metrics
    pub is_computed_ready: bool,         // Computation status
    pub compute_version: u64,            // Version for invalidation
}
```

## Data Fetching and Caching

### API Communication

The crate expects a specific API response format:
1. **JSON Header**: Terminated by newline (ASCII 10)
2. **Binary Payload**: Columnar data in little-endian format

```rust
// API Response Structure
pub struct ApiHeader {
    pub columns: Vec<ColumnMeta>,
}

pub struct ColumnMeta {
    pub name: String,
    pub data_length: usize,  // Total bytes for this column
}
```

### Caching Mechanism

LRU (Least Recently Used) cache implementation:

```rust
pub struct DataCache {
    capacity: usize,                    // Max cache size in bytes
    entries: HashMap<String, DataHandle>, // Cached data handles
    access_order: Vec<String>,          // LRU tracking
}
```

**Cache Key Format**: `"{symbol}-{data_type}-{start_time}-{end_time}-{columns:?}"`

## Binary Data Parsing

### Parser Implementation (`src/binary_parser.rs`)

Handles parsing of server binary responses:

```rust
pub struct DataResponseHeader {
    pub symbol: String,
    pub columns: Vec<String>,
    pub start_time: u32,
    pub end_time: u32,
    pub row_count: usize,
}
```

**Key Features:**
- **Row-by-row parsing**: Processes data sequentially for memory efficiency
- **Type conversion**: Handles u32 timestamps and f32 values
- **Bounds checking**: Validates buffer access to prevent overruns
- **Binary search**: Optimized timestamp lookups for range queries
- **Batch parsing**: Chunked processing for large datasets

### Data Format

Binary data layout (little-endian):
- **Timestamps**: 4 bytes (u32)
- **Values**: 4 bytes (f32)
- **Layout**: Row-major order (all columns for row 1, then row 2, etc.)

## GPU Buffer Creation and Management

### Buffer Creation Strategy

```rust
pub fn create_chunked_gpu_buffer_from_arraybuffer(
    device: &wgpu::Device,
    data: &ArrayBuffer,
    label: &str,
) -> Vec<wgpu::Buffer>
```

**Chunking Strategy:**
- Maximum chunk size: 128MB
- Prevents GPU memory exhaustion
- Enables streaming of large datasets
- Maintains buffer continuity for rendering

### Buffer Types and Usage Flags

```rust
usage: wgpu::BufferUsages::VERTEX
    | wgpu::BufferUsages::COPY_DST
    | wgpu::BufferUsages::STORAGE
    | wgpu::BufferUsages::COPY_SRC
```

## Screen-Space Coordinate Transformations

### Transformation Methods

```rust
impl DataStore {
    // World to screen with 10% Y-axis margin
    pub fn world_to_screen_with_margin(&self, x: f32, y: f32) -> (f32, f32)
    
    // Y coordinate to screen position for axis labels
    pub fn y_to_screen_position(&self, y: f32) -> f32
    
    // Screen to world with margin (for mouse interactions)
    pub fn screen_to_world_with_margin(&self, screen_x: f32, screen_y: f32) -> (f32, f32)
}
```

**Transformation Pipeline:**
1. Apply 10% margin to Y range for visual padding
2. Create orthographic projection matrix
3. Transform world coordinates to NDC (-1 to 1)
4. Convert NDC to screen pixels

## HTTP Client Configuration

### Request Building

```rust
// URL encoding for API requests
let encoded_symbol = urlencoding::encode(symbol);
let encoded_columns = urlencoding::encode(&columns_str);

let url = format!(
    "{}/api/data?symbol={}&type={}&start={}&end={}&columns={}&exchange=coinbase",
    self.base_url, encoded_symbol, data_type, start_time, end_time, encoded_columns
);
```

### Error Handling

```rust
fetch_api_response(&url)
    .await
    .map_err(|e| GpuChartsError::DataFetch {
        message: format!("{e:?} (URL: {url})"),
    })?;
```

## Dependencies and Integration Points

### External Dependencies
- **reqwasm**: HTTP client for WASM environment
- **wgpu**: WebGPU bindings for buffer creation
- **bytemuck**: Zero-copy transmutation for GPU data
- **nalgebra-glm**: Matrix operations for transformations
- **js-sys/web-sys**: JavaScript/Web API bindings
- **uuid**: Unique identifiers for data handles
- **serde/serde_json**: JSON parsing for API headers

### Internal Dependencies
- **shared-types**: Common types and error definitions
- **config-system**: Chart presets and configuration

## Error Handling Patterns

### Error Types

The crate uses `GpuChartsError` from shared-types:
- `DataFetch`: Network or API errors
- `DataNotFound`: Missing resources or buffers
- `ParseError`: Binary data parsing failures

### Error Recovery

```rust
match result {
    Ok(data_handle) => {
        self.process_data_handle(&data_handle, data_store)?;
    }
    Err(e) => {
        log::error!("Failed to fetch {data_type} data: {e:?}");
        // Continue processing other data types
    }
}
```

## Performance Considerations

### Memory Optimization
- **Chunked GPU buffers**: Prevents single large allocations
- **LRU cache eviction**: Manages memory pressure
- **ArrayBuffer reuse**: Minimizes JavaScript heap allocations
- **Columnar storage**: Optimizes GPU memory access patterns

### Computational Efficiency
- **Binary search**: O(log n) timestamp lookups
- **Batch processing**: Reduces JavaScript-WASM boundary crossings
- **GPU-side calculations**: Min/max computed on GPU for large datasets
- **Lazy computation**: Computed metrics calculated on-demand

### Network Optimization
- **Cache-first strategy**: Checks cache before network requests
- **URL encoding**: Properly encodes parameters for reliability
- **Column selection**: Fetches only required data columns
- **Binary format**: More efficient than JSON for numeric data

## Preset and Computed Metrics Support

### Preset Data Loading

The `fetch_data_for_preset()` method:
1. Analyzes preset chart types for data requirements
2. Aggregates column requirements by data type
3. Fetches all required data in parallel
4. Creates computed metrics after base data loads

### Computed Metric System

```rust
pub enum ComputeOp {
    MidPrice,      // (bid + ask) / 2
    Spread,        // ask - bid
    // Additional operations...
}
```

**Dependency Resolution:**
- Tracks metric dependencies via `MetricRef`
- Validates all dependencies are loaded
- Invalidates computed metrics when dependencies change
- Maintains computation version for cache invalidation

## Best Practices

### Data Management
1. **Always check cache first**: Reduces unnecessary network requests
2. **Use presets for consistency**: Ensures proper data loading order
3. **Handle partial failures**: Continue processing available data
4. **Clear GPU bounds on data change**: Ensures correct rendering

### Buffer Management
1. **Chunk large datasets**: Prevents GPU memory exhaustion
2. **Label buffers descriptively**: Aids debugging and profiling
3. **Use appropriate usage flags**: Optimize for access patterns
4. **Clean up unused buffers**: Call `clear_cache()` when switching datasets

### Error Handling
1. **Log errors with context**: Include URLs and parameters
2. **Gracefully degrade**: Show partial data when possible
3. **Validate data bounds**: Prevent buffer overruns
4. **Handle network timeouts**: Implement retry logic in caller

### Performance
1. **Batch related requests**: Minimize round trips
2. **Profile memory usage**: Monitor GPU buffer allocation
3. **Use binary search for lookups**: Efficient range queries
4. **Minimize coordinate transformations**: Cache when possible

## Testing Considerations

### Unit Tests
- Cache LRU eviction logic
- Binary search implementation
- Coordinate transformations
- Data parsing edge cases

### Integration Tests
- API request handling
- GPU buffer creation
- Preset data loading
- Computed metric calculations

### Performance Tests
- Large dataset handling (>1M points)
- Cache efficiency metrics
- GPU memory usage
- Network request latency

## Common Usage Patterns

### Basic Data Loading
```rust
let mut data_manager = DataManager::new(device, queue, base_url);
let handle = data_manager.fetch_data(
    "BTC-USD", "MD", 
    start_time, end_time,
    &["time", "best_bid", "best_ask"]
).await?;
```

### Preset-Based Loading
```rust
data_store.set_preset_and_symbol(Some(&preset), Some("ETH-USD".to_string()));
data_manager.fetch_data_for_preset(&mut data_store).await?;
```

### Coordinate Transformation
```rust
let (screen_x, screen_y) = data_store.world_to_screen_with_margin(
    timestamp as f32, 
    price
);
```

## Future Enhancements

### Planned Improvements
- WebSocket support for real-time updates
- Incremental data loading for infinite scroll
- Multi-resolution data (LOD) for zoom levels
- Compressed binary formats (zstd, lz4)
- Parallel chunk processing
- GPU-accelerated parsing
- Persistent browser cache via IndexedDB