# CLAUDE.md - Ultra-Low Latency Data Server

This file provides guidance to Claude Code when working with the high-performance HTTP/2 TLS server in this directory.

## Purpose and Architecture

This is an ultra-low latency financial data server built in Rust, designed to serve time-series market data with minimal overhead. The server achieves microsecond-level latencies through:

- **Memory-mapped I/O** for zero-copy data access
- **HTTP/2 with TLS** for secure, multiplexed connections
- **Binary data format** with efficient 4-byte records
- **Memory locking (mlock)** on Linux for consistent performance
- **Asynchronous I/O** with Tokio for high concurrency
- **Multi-day query support** with automatic date range handling

## Core Technologies

- **Hyper 0.14**: HTTP/2 server implementation
- **Tokio**: Async runtime with full feature set
- **Tokio-rustls**: TLS implementation using rustls
- **Memmap2**: Memory-mapped file I/O for zero-copy access
- **Chrono**: Date/time handling for multi-day queries
- **Serde/serde_json**: JSON serialization for API responses

## Server Implementation Details

### Main Entry Point (`src/main.rs`)
- Configurable TLS/HTTP mode via `USE_TLS` environment variable
- HTTP/2-only for TLS connections (optimal for multiplexing)
- HTTP/1.1-only for plain HTTP (Cloudflare Tunnel compatibility)
- TCP_NODELAY enabled for minimal latency
- CORS headers on all responses for web frontend integration
- Handles OPTIONS preflight requests for CORS

### Data API (`src/data.rs`)
The `/api/data` endpoint serves time-series market data with sophisticated features:

#### Zero-Copy Architecture
```rust
struct ZeroCopyChunk {
    mmap: Arc<Mmap>,  // Shared memory-mapped file
    offset: usize,    // Start offset in the mmap
    len: usize,       // Length of this chunk
    pos: usize,       // Current read position
}
```
- Memory-mapped files avoid kernel buffer copies
- Arc allows shared ownership across async tasks
- Streaming response with backpressure support

#### Multi-Day Query Processing
- Automatically spans queries across multiple day files
- File naming: `{column}.{DD}.{MM}.{YY}.bin`
- Efficient binary search to find time ranges within each day
- Aggregates data from multiple days into a single response

#### Performance Optimizations
- **mlock() on Linux**: Locks memory pages to prevent swapping
- **Binary search**: O(log n) time range lookups
- **Streaming response**: Starts sending data before full query completes
- **Parallel column loading**: Concurrent mmap operations per column

#### Response Format
```json
{
  "columns": [
    {
      "name": "time",
      "record_size": 4,
      "num_records": 1000,
      "data_length": 4000
    },
    {
      "name": "best_bid",
      "record_size": 4,
      "num_records": 1000,
      "data_length": 4000
    }
  ]
}
```
Followed by raw binary data (4-byte little-endian records).

### Symbols API (`src/symbols.rs`)
The `/api/symbols` endpoint provides symbol discovery and metadata:

#### Features
- Lists all available trading symbols across exchanges
- Shows last update timestamp for each symbol
- Supports exchange filtering via query parameter
- Sorts symbols by recency (newest first)
- Scans filesystem for real-time accuracy

#### Implementation Details
- Asynchronous directory traversal with `tokio::fs`
- Stream processing with `tokio_stream` for memory efficiency
- Finds latest modification time across all data files
- Human-readable date formatting with Chrono

## File Format and Data Structure

### Directory Layout
```
/mnt/md/data/
├── {exchange}/           # e.g., coinbase, binance, kraken
│   └── {symbol}/         # e.g., BTC-USD, ETH-USD
│       └── {type}/       # e.g., MD (Market Data), TRADES
│           ├── time.{DD}.{MM}.{YY}.bin      # Unix timestamps (4 bytes each)
│           ├── best_bid.{DD}.{MM}.{YY}.bin   # Bid prices (4 bytes each)
│           ├── best_ask.{DD}.{MM}.{YY}.bin   # Ask prices (4 bytes each)
│           ├── price.{DD}.{MM}.{YY}.bin      # Trade prices (4 bytes each)
│           ├── volume.{DD}.{MM}.{YY}.bin     # Trade volumes (4 bytes each)
│           └── side.{DD}.{MM}.{YY}.bin       # Trade sides (4 bytes each)
```

### Binary Format
- All data stored as 4-byte little-endian values
- Time column: u32 Unix timestamps (seconds since epoch)
- Price columns: f32 values encoded as u32
- Volume column: f32 values encoded as u32
- Side column: u32 (0 = buy, 1 = sell)
- Files must be sorted by timestamp for binary search

## Configuration Management

### Build-Time Configuration (`build.rs`)
- Reads `config.toml` at compile time
- Embeds configuration as environment variables
- Supports development/production profiles
- Default data path: `/mnt/md/data`
- Default port: 8443

### Runtime Configuration
Environment variables override build-time defaults:
- `USE_TLS`: Enable/disable TLS (default: true)
- `PORT`: Server port (default: 8443)
- `SSL_CERT_PATH`: Path to SSL certificate
- `SSL_PRIVATE_FILE`: Path to SSL private key
- `DATA_PATH`: Override data directory path (not currently used)

## TLS/SSL Implementation

### Certificate Loading
- Supports PEM format certificates
- Reads PKCS8 private keys
- Configurable paths via environment variables
- Defaults: `localhost.crt` and `localhost.key`

### Protocol Support
- HTTP/2 preferred for TLS connections
- HTTP/1.1 fallback available
- ALPN negotiation for protocol selection
- Safe TLS defaults via rustls

## Testing Infrastructure

### Unit Tests (`tests/unit_tests.rs`)
18 comprehensive tests covering:
- Query parameter parsing and validation
- Binary search algorithms (start/end index)
- Memory-mapped file operations
- Edge cases (empty files, single elements)
- Column metadata and record sizes

### Integration Tests (`tests/data_tests.rs`)
8 tests validating:
- End-to-end request handling
- Mock data generation
- Multi-column queries
- Error handling

### Test Scripts
- `test_symbols_api.sh`: Tests symbols endpoint functionality
- `test_symbols_advanced.sh`: Advanced symbol API testing

### Running Tests
```bash
# Must use native target to avoid WASM issues
cargo test --target x86_64-unknown-linux-gnu

# Or use npm scripts from project root
npm run test:server
```

## Error Handling Patterns

### Graceful Degradation
- Missing day files are skipped with warnings
- Partial data returned when some files unavailable
- Detailed error messages for debugging

### Error Response Format
HTTP status codes with descriptive messages:
- 400 Bad Request: Invalid query parameters
- 404 Not Found: Unknown endpoints
- 500 Internal Server Error: File I/O or data format issues

## Performance Characteristics

### Latency Targets
- Sub-millisecond response times for cached data
- 1-5ms for multi-day queries
- Zero-copy path from disk to network

### Memory Usage
- Virtual memory scales with data size (mmap)
- Physical memory usage minimal (page cache)
- Optional mlock() prevents swapping

### Concurrency
- Tokio runtime with work-stealing scheduler
- One task spawned per connection
- Async I/O prevents blocking on file operations

## Deployment Considerations

### Docker Deployment
- Multi-stage build for minimal image size
- cargo-chef for dependency caching
- Non-root user (uid 1000) for security
- Health checks via `/api/symbols` endpoint

### Production Setup
- Mount data as read-only volume
- Use SSD/NVMe storage for best performance
- Consider dedicated CPU cores
- Increase file descriptor limits
- Monitor memory usage and page faults

### Cloudflare Tunnel Integration
- Set `USE_TLS=false` for tunnel deployment
- HTTP/1.1 mode for compatibility
- Tunnel handles TLS termination
- CORS headers for web access

## Security Considerations

- TLS encryption for data in transit
- Read-only data access recommended
- No built-in authentication (use reverse proxy)
- Runs as non-privileged user in container
- Input validation on all query parameters

## API Usage Examples

### Data Query
```bash
curl -k "https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1234567890&end=1234567900&columns=time,best_bid,best_ask"
```

### Symbol Discovery
```bash
# All symbols
curl -k "https://localhost:8443/api/symbols"

# Filter by exchange
curl -k "https://localhost:8443/api/symbols?exchange=coinbase"
```

## Common Issues and Solutions

### Issue: mlock() warnings
**Solution**: Increase memory lock limits or run with appropriate capabilities

### Issue: Certificate errors
**Solution**: Use `-k` flag for self-signed certs or provide valid certificates

### Issue: Slow queries spanning many days
**Solution**: Optimize date range or implement caching layer

### Issue: High memory usage
**Solution**: Data is memory-mapped, virtual memory is normal; monitor RSS instead

## Future Optimization Opportunities

1. **Caching Layer**: LRU cache for frequently accessed data
2. **Compression**: Support for compressed responses (gzip/brotli)
3. **Index Files**: Pre-computed indices for faster time lookups
4. **Parallel Processing**: Multi-threaded column processing
5. **Connection Pooling**: Reuse mmap handles across requests
6. **WebSocket Support**: Real-time data streaming
7. **Authentication**: JWT or API key authentication
8. **Rate Limiting**: Protect against abuse
9. **Metrics**: Prometheus/Grafana integration
10. **Distributed Cache**: Redis/Memcached for multi-server deployments

## Development Workflow

```bash
# Build and run locally
cargo build --release
./target/release/ultra_low_latency_server_chunked_parallel

# Run with custom settings
USE_TLS=false PORT=8080 cargo run

# Test with data
curl -k "https://localhost:8443/api/symbols"

# Run tests
cargo test --target x86_64-unknown-linux-gnu

# Build Docker image
docker build -f server/Dockerfile -t gpu-charts-server .

# Run container
docker run -p 8443:8443 -v /mnt/md/data:/mnt/md/data:ro gpu-charts-server
```

## Code Quality Standards

- Use `cargo fmt` for consistent formatting
- Run `cargo clippy` for linting
- Maintain comprehensive test coverage
- Document performance-critical sections
- Profile before optimizing
- Measure after implementing