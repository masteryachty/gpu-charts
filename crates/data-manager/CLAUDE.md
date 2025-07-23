# Data Manager Crate - CLAUDE.md

This file provides guidance for working with the data-manager crate, which handles all data operations including fetching, parsing, caching, and GPU buffer management for the GPU Charts system.

## Overview

The data-manager crate provides:
- Asynchronous data fetching from HTTP/HTTPS endpoints
- Binary data parsing and deserialization
- GPU buffer creation and management
- Data caching and memory management
- Screen-space coordinate transformations
- Time-series data handling

## Architecture Position

```
shared-types
    ↑
data-manager (this crate)
    ↑
└── wasm-bridge
```

This crate is used by wasm-bridge to manage all data operations.

## Key Components

### DataManager (`src/lib.rs`)
Main interface for data operations:

```rust
pub struct DataManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    base_url: String,
    cache: HashMap<String, DataHandle>,
}

impl DataManager {
    pub async fn fetch_data(
        &mut self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
        columns: &[&str],
    ) -> Result<DataHandle, DataManagerError>;
}
```

### DataStore (`src/data_store.rs`)
Core data storage and transformation:

```rust
pub struct DataStore {
    pub topic: Option<String>,
    pub columns: Vec<ColumnData>,
    pub min_max_buffer: Option<Arc<wgpu::Buffer>>,
    pub screen_pos_buffer: Option<Arc<wgpu::Buffer>>,
    pub x_range: (u32, u32),
    pub y_range: (f32, f32),
    pub zoom_level: f32,
    pub pan_offset: (f32, f32),
}
```

### DataRetriever (`src/data_retriever.rs`)
Handles HTTP data fetching:

```rust
pub struct DataRetriever {
    base_url: String,
}

impl DataRetriever {
    pub async fn fetch_data(
        &self,
        topic: &str,
        start: u32,
        end: u32,
    ) -> Result<ParsedData, Box<dyn std::error::Error>>;
}
```

### Binary Parser (`src/binary_parser.rs`)
Parses custom binary format:

```rust
pub struct DataResponseHeader {
    pub column_count: u32,
    pub row_count: u32,
    pub columns: Vec<ColumnMeta>,
}

pub fn parse_binary_data(data: &[u8]) -> Result<ParsedData, ParserError>;
```

## Data Flow

1. **Request**: Client requests data for symbol/time range
2. **Fetch**: DataRetriever fetches from HTTP endpoint
3. **Parse**: Binary parser converts to ParsedData
4. **GPU Upload**: Create GPU buffers for rendering
5. **Cache**: Store in memory cache for reuse
6. **Transform**: Apply zoom/pan transformations

## Usage Patterns

### Basic Data Fetching

```rust
let mut data_manager = DataManager::new(device, queue, "https://api.example.com");

// Fetch market data
let handle = data_manager.fetch_data(
    "BTC-USD",
    1234567890,
    1234567900,
    &["time", "price", "volume"]
).await?;
```

### Direct Data Store Usage

```rust
let data_store = Rc::new(RefCell::new(DataStore::new()));

// Set data range
data_store.borrow_mut().set_x_range(start_time, end_time);

// Update zoom
data_store.borrow_mut().zoom(1.5, center_x);

// Pan view
data_store.borrow_mut().pan(delta_x, delta_y);
```

### Binary Data Format

The binary format is optimized for GPU upload:

```
Header (variable size):
- column_count: u32
- row_count: u32
- For each column:
  - name_length: u32
  - name: UTF-8 string
  - data_type: u8 (0=f32, 1=u32, 2=i32)

Data (fixed size):
- Columnar layout (all values for column 1, then column 2, etc.)
- Each value is 4 bytes (f32/u32/i32)
```

## GPU Buffer Management

### Buffer Types

1. **Raw Data Buffers**: Store original time-series data
2. **Min/Max Buffer**: Computed bounds for normalization
3. **Screen Position Buffer**: Transformed coordinates for rendering

### Buffer Creation

```rust
fn create_gpu_buffers(device: &Device, data: &ParsedData) -> GpuBufferSet {
    let buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Data Buffer"),
        contents: bytemuck::cast_slice(&data.values),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });
    
    GpuBufferSet {
        buffers: vec![buffer],
        metadata: data.metadata.clone(),
    }
}
```

## Performance Optimizations

### Caching Strategy

```rust
// Check cache before fetching
if let Some(handle) = self.cache.get(&cache_key) {
    if handle.is_valid_for(start_time, end_time) {
        return Ok(handle.clone());
    }
}
```

### Efficient Data Updates

```rust
// Only update changed ranges
pub fn update_partial_data(&mut self, column: usize, start: usize, new_data: &[f32]) {
    let offset = column * self.row_count + start;
    self.queue.write_buffer(
        &self.buffer,
        (offset * 4) as u64,
        bytemuck::cast_slice(new_data),
    );
}
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum DataManagerError {
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("GPU error: {0}")]
    GpuError(String),
    
    #[error("Cache miss for key: {0}")]
    CacheMiss(String),
}
```

## Best Practices

1. **Batch Requests**: Fetch multiple columns in one request
2. **Cache Aggressively**: Memory is cheaper than network latency
3. **Validate Data**: Check bounds and types before GPU upload
4. **Handle Errors Gracefully**: Network requests can fail
5. **Profile Memory Usage**: Large datasets can exhaust GPU memory

## Testing

Key test scenarios:

```rust
#[test]
async fn test_data_fetching() {
    let manager = DataManager::new_mock();
    let result = manager.fetch_data("TEST", 0, 100, &["price"]).await;
    assert!(result.is_ok());
}

#[test]
fn test_binary_parsing() {
    let binary_data = create_test_binary_data();
    let parsed = parse_binary_data(&binary_data).unwrap();
    assert_eq!(parsed.columns.len(), 2);
}

#[test]
fn test_coordinate_transformation() {
    let mut store = DataStore::new();
    store.set_x_range(0, 100);
    store.zoom(2.0, 50.0);
    
    let transformed = store.transform_x(50.0);
    assert_eq!(transformed, expected_value);
}
```

## Integration with Server

The data-manager expects the server to provide:
- Binary data in the specified format
- CORS headers for web access
- Efficient range queries
- Column selection support

Example server endpoint:
```
GET /api/data?symbol=BTC-USD&start=123456&end=123466&columns=time,price,volume
```

## Future Enhancements

- WebSocket support for real-time data
- Delta compression for updates
- Multi-resolution data (LOD)
- Persistent disk caching
- Parallel data fetching
- Custom binary format optimization
- Protobuf/MessagePack support