# Server Directory - CLAUDE.md

This file provides specific guidance for working with the high-performance data server component of the graph visualization application.

## Overview

The server directory contains an ultra-low latency HTTP/2 TLS server built in Rust for serving financial time-series data. This server is optimized for zero-copy data access using memory-mapped files and is designed to handle high-frequency market data queries with minimal latency.

## Development Commands

### Build and Run
```bash
# Development server (from web/ directory)
npm run dev:server

# Direct development (from server/ directory)
cargo run --target x86_64-unknown-linux-gnu

# Production build
cargo build --release --target x86_64-unknown-linux-gnu

# Development build
cargo build --target x86_64-unknown-linux-gnu
```

### Testing
```bash
# All tests (MUST use native target - critical for testing)
cargo test --target x86_64-unknown-linux-gnu

# Unit tests only (18 tests)
cargo test --target x86_64-unknown-linux-gnu unit_tests

# Integration tests only (8 tests)  
cargo test --target x86_64-unknown-linux-gnu data_tests

# Live API tests (requires running server)
./test_api.sh
```

**Critical**: Always use `--target x86_64-unknown-linux-gnu` to avoid WASM compilation issues during development and testing.

## Server Architecture

### Core Technology Stack
- **HTTP Server**: Hyper with HTTP/2 and TLS (tokio-rustls)
- **Data Storage**: Memory-mapped binary files via memmap2
- **Performance**: Memory locking (mlock) for ultra-low latency
- **Security**: Full TLS encryption with local SSL certificates
- **Async Runtime**: Tokio with connection pooling and non-blocking I/O

### Module Organization
```
src/
├── main.rs          # HTTP/2 TLS server, request routing, CORS
├── lib.rs           # Public module exports
├── data.rs          # Core data serving, memory mapping, binary search
└── symbols.rs       # Symbol discovery and API endpoint
```

## API Endpoints

### Data Endpoint: `GET /api/data`
Serves time-series financial data with zero-copy streaming.

**Query Parameters:**
- `symbol`: Trading symbol (e.g., "BTC-USD")
- `type`: Data type (e.g., "MD" for Market Data)
- `start`: Unix timestamp (u32)
- `end`: Unix timestamp (u32)
- `columns`: Comma-separated column names

**Supported Columns:**
- `time`: 4-byte Unix timestamps
- `best_bid`: 4-byte bid prices
- `best_ask`: 4-byte ask prices
- `price`: 4-byte trade prices
- `volume`: 4-byte trade volumes
- `side`: 4-byte trade sides

**Response Format:**
1. JSON header line with column metadata
2. Binary data stream (memory-mapped file chunks)

**Example Request:**
```
https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid
```

### Symbols Endpoint: `GET /api/symbols`
Returns available trading symbols from the data directory.

**Response:**
```json
{
  "symbols": ["BTC-USD", "ETH-USD", "SOL-USD", ...]
}
```

## Data Storage Architecture

### File Organization
```
/mnt/md/data/
├── {symbol}/          # e.g., BTC-USD/
│   └── {type}/        # e.g., MD/
│       ├── time.{DD}.{MM}.{YY}.bin
│       ├── best_bid.{DD}.{MM}.{YY}.bin
│       ├── best_ask.{DD}.{MM}.{YY}.bin
│       └── ...
```

### Binary File Format
- **Record Size**: Fixed 4-byte records for all columns
- **Byte Order**: Little-endian (x86_64 standard)
- **Time Format**: Unix timestamps as u32 values
- **Pricing**: Raw u32 values (scaling handled by client)

### Multi-Day Query Processing
1. **Date Range Expansion**: Convert query timestamps to date list
2. **File Discovery**: Locate and validate per-day files
3. **Binary Search**: O(log n) time range filtering within each day
4. **Chunk Streaming**: Memory-mapped chunks streamed to client

## Performance Optimizations

### Memory Management
- **Memory-Mapped Files**: Zero-copy data access via memmap2
- **Memory Locking**: mlock() system calls prevent swapping
- **Arc-based Sharing**: Efficient memory sharing across async tasks
- **Lazy Loading**: Files loaded only when needed

### Network Optimizations
- **HTTP/2 Only**: Maximum protocol efficiency
- **TCP_NODELAY**: Disabled Nagle's algorithm for low latency
- **TLS Session Reuse**: Connection pooling for repeated requests
- **Chunked Streaming**: Large datasets served without full buffering

### Data Access Patterns
- **Binary Search**: O(log n) time range queries
- **Day-based Partitioning**: Efficient multi-day processing
- **Sorted Data Assumption**: Files must be sorted by timestamp
- **Index Caching**: In-memory indices for frequently accessed ranges

## Security Configuration

### TLS Setup
- **Certificates**: Development certificates included (localhost.crt/key)
- **Modern TLS**: rustls for memory-safe TLS implementation
- **ALPN Support**: HTTP/2 and HTTP/1.1 protocol negotiation
- **Certificate Loading**: Supports both .crt/.key and .pem formats

### CORS Configuration
- **Wildcard Origins**: `Access-Control-Allow-Origin: *`
- **Preflight Support**: OPTIONS method handling
- **Headers**: Accepts `Content-Type`, `Authorization`, custom headers
- **Methods**: GET, POST, OPTIONS

## Testing Infrastructure

### Unit Tests (18 tests in `tests/unit_tests.rs`)
**Query Parameter Testing:**
- Valid parameter parsing and validation
- Missing required field detection
- Invalid data type handling
- Multiple column parameter support

**Binary Search Algorithm:**
- Exact timestamp matches
- Between-element searches
- Boundary conditions (before first, after last)
- Single-element and empty array edge cases

**Memory-Mapped File Operations:**
- Successful file loading and mapping
- Nonexistent file error handling
- Empty file handling
- Data integrity verification

### Integration Tests (8 tests in `tests/data_tests.rs`)
**End-to-End Processing:**
- Complete request pipeline testing
- Mock data structure creation and validation
- Multi-column data serving scenarios
- Response format verification

**Test Data Generation:**
```rust
// Creates realistic test directory structure
async fn create_test_data_structure() -> TempDir {
    // Generates /tmp/.../symbol/type/column.DD.MM.YY.bin
    // Creates binary data with sorted timestamps
    // Tests multi-day query scenarios
}
```

### Live API Tests (`test_api.sh`)
**Comprehensive API Validation:**
- Server connectivity and HTTPS endpoint availability
- Symbols endpoint response format validation
- Data endpoint with valid parameter processing
- Error handling for missing/invalid parameters
- HTTP status code verification (404 for invalid endpoints)
- CORS headers and OPTIONS preflight testing
- SSL/TLS certificate validation

**Test Examples:**
```bash
# Symbols endpoint
curl -k -s "https://localhost:8443/api/symbols"

# Valid data request
curl -k -s "https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1745322750&end=1745391150&columns=time,best_bid"

# Error case - missing parameters
curl -k -s "https://localhost:8443/api/data?symbol=BTC-USD&type=MD"
```

## Error Handling Patterns

### Comprehensive Error Coverage
- **File I/O Errors**: Graceful handling of missing files
- **Parse Errors**: Detailed error messages for invalid parameters
- **Memory Mapping**: Safe handling of mmap failures
- **TLS Errors**: Certificate and connection error handling
- **Data Validation**: Sort order verification and integrity checks

### Logging and Monitoring
```rust
// Request timing for performance analysis
let start = Instant::now();
// ... process request ...
println!("Request handled in {:?}", start.elapsed());

// Memory lock warnings
if mlock_result != 0 {
    eprintln!("Warning: mlock failed for {} (errno {})", path, ret);
}
```

## Build Configuration

### Cargo.toml Key Dependencies
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }      # Async runtime
hyper = { version = "0.14", features = ["full"] }   # HTTP/2 server
memmap2 = "0.5"                                     # Memory-mapped files
rustls = "0.21.11"                                  # Modern TLS
tokio-rustls = "0.24"                              # Async TLS integration
serde = { version = "1.0", features = ["derive"] }  # JSON serialization
chrono = "0.4.41"                                   # Date/time handling
libc = "0.2"                                        # System calls (mlock)
bytes = "1.0"                                       # Zero-copy byte handling
```

### Build Requirements
- **Target Platform**: Must use `x86_64-unknown-linux-gnu` for native performance
- **SSL Certificates**: Required for HTTPS (localhost.crt/key included)
- **Data Directory**: Expects `/mnt/md/data/` structure or configure path
- **Memory Permissions**: May require increased memory limits for large datasets

## Integration with Frontend

### API Contract
- **Base URL**: `https://localhost:8443/api/`
- **Content-Type**: `application/octet-stream` for data responses
- **CORS**: Fully enabled for web frontend integration
- **Authentication**: None (local development setup)

### Response Protocol
1. **JSON Header**: Single line with column metadata and record counts
2. **Binary Data**: Raw memory-mapped file contents streamed sequentially
3. **End-of-Stream**: Connection close indicates complete response

### Frontend Integration Points
- **WebAssembly Client**: Rust WASM module consumes binary data directly
- **React Component**: Chart component manages data fetching and display
- **Error Handling**: HTTP status codes and JSON error responses

## Performance Considerations

### Memory Usage
- **Memory-Mapped Files**: Virtual memory usage can be large but physical usage is efficient
- **Buffer Sizes**: Configurable chunk sizes for different memory environments
- **Memory Locking**: Consider system limits for mlock() usage

### Latency Optimization
- **Cold Start**: First query per file may be slower due to initial mapping
- **Warm Cache**: Subsequent queries on same files are extremely fast
- **Data Locality**: Keep frequently accessed files on fast storage (SSD/NVMe)

### Scalability
- **Concurrent Connections**: Tokio async runtime handles many simultaneous connections
- **File Handle Limits**: Consider system limits for large numbers of data files
- **CPU Usage**: Binary search and data processing are CPU-efficient

## Deployment Considerations

### Development Deployment
- SSL certificates included for localhost development
- Default port 8443 (configurable)
- Expects data in `/mnt/md/data/` (or configure alternative path)

### Production Deployment
- Replace development certificates with production certificates
- Configure proper data directory paths
- Consider file permissions and security
- Monitor memory usage and file handle limits
- Set up proper logging and monitoring

## Common Development Tasks

### Adding New Data Columns
1. Ensure binary files follow naming convention: `{column}.{DD}.{MM}.{YY}.bin`
2. Verify data is sorted by timestamp
3. Add column name to query parameter validation if needed
4. Test with integration tests

### Debugging Data Issues
```bash
# Check file existence and size
ls -la /mnt/md/data/BTC-USD/MD/

# Verify binary data format (should be sorted timestamps)
xxd -l 64 /mnt/md/data/BTC-USD/MD/time.01.03.25.bin

# Test API directly
curl -k -v "https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time"
```

### Performance Profiling
- Use `perf` for CPU profiling during heavy load
- Monitor memory usage with system tools
- Check file I/O patterns with `iotop`
- Use async profiling tools for tokio runtime analysis

This server component is designed for extreme performance in financial data serving scenarios and requires careful attention to system-level optimizations and proper data management.