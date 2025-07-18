# Coinbase Logger - CLAUDE.md

This file provides specific guidance for working with the real-time market data collection component that feeds the graph visualization application.

## Overview

The coinbase-logger is a high-performance Rust application that connects to Coinbase's WebSocket feed to collect real-time market data for all available trading pairs. It writes data in binary format that's directly compatible with the server's memory-mapped file architecture, enabling seamless real-time data visualization.

## Architecture Overview

### Core Design Philosophy
- **Real-time Collection**: Live WebSocket connection to Coinbase exchange
- **Multi-threaded Processing**: Concurrent data collection for hundreds of trading pairs
- **Binary Compatibility**: Output format exactly matches server's memory-mapped expectations
- **Fault Tolerance**: Automatic reconnection with exponential backoff
- **Zero-Copy Pipeline**: Direct binary writes for maximum performance

### System Architecture
```
Coinbase WebSocket → Multi-threaded Processing → Binary Files → Server Memory Mapping → Web Visualization
```

## Development Commands

### Build and Run
```bash
# Development mode (from web/ directory)
npm run dev:logger

# Direct development (from coinbase-logger/ directory)
cargo run --target x86_64-unknown-linux-gnu

# Production build
cargo build --release --target x86_64-unknown-linux-gnu

# Development build
cargo build --target x86_64-unknown-linux-gnu

# Docker build and run
docker compose up --build              # Development mode (builds locally)
docker compose -f docker-compose.prod.yml up  # Production mode (uses pre-built image)
```

### Complete Development Workflow
```bash
# Full stack with real-time data collection
npm run dev:suite:full

# This runs concurrently:
# 1. WASM file watcher and rebuilder
# 2. Data server (port 8443)
# 3. Coinbase logger (real-time data collection)
# 4. React dev server (port 3000)
```

### Testing
```bash
# Run tests (from web/ directory)
npm run test:logger

# Direct testing (from coinbase-logger/ directory)
cargo test --target x86_64-unknown-linux-gnu
```

**Critical**: Always use `--target x86_64-unknown-linux-gnu` for optimal performance and compatibility.

## Application Architecture

### Multi-Threading Model (Performance-Optimized)
```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Discover all available trading pairs via status channel
    // 2. Create 10 connection handlers concurrently (no rate limiting!)
    // 3. Each connection subscribes to ~20 symbols in a single request
    // 4. Smart message buffering with automatic timestamp sorting
}
```

**Key Performance Features:**
- **Connection Pooling**: 10 connections handle 200+ symbols (20x reduction from previous 200+ connections)
- **Concurrent Creation**: All connections created in parallel (386x faster startup)
- **Multi-Symbol Subscriptions**: Single subscription request per connection for ~20 symbols
- **Smart Buffering**: BTreeMap automatically sorts messages by timestamp before writing
- **Extended Flush Interval**: 5-second flush reduces disk I/O by 5x
- **Buffered File Writes**: 64KB BufWriter reduces syscalls by 10-100x
- **Larger WebSocket Buffers**: 256KB buffers (32x increase) for better throughput
- **Nanosecond Precision**: Separate `nanos.{date}.bin` files preserve full timestamp accuracy

### Performance Metrics Achieved
Based on the recent optimizations, the coinbase-logger has achieved remarkable performance improvements:

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Startup Time | 386+ seconds | ~1 second | **386x faster** |
| WebSocket Connections | 200+ | 10 | **20x reduction** |
| Connection Creation | 1/second (rate limited) | All concurrent | **No rate limit** |
| File Write Syscalls | Every message | Buffered (64KB) | **10-100x fewer** |
| Disk Flush Frequency | Every 1 second | Every 5 seconds | **5x reduction** |
| WebSocket Buffer Size | 8KB | 256KB | **32x larger** |
| Timestamp Precision | Seconds only | Nanoseconds | **Full precision** |
| Message Ordering | Best effort | Guaranteed (BTreeMap) | **100% ordered** |

### WebSocket Client Design

#### Product Discovery
```rust
async fn get_all_products() -> Result<Vec<String>, Box<dyn Error>> {
    // 1. Connect to Coinbase WebSocket status channel
    // 2. Subscribe to "status" messages
    // 3. Extract all "online" trading pairs
    // 4. Return filtered list of active symbols
}
```

#### Connection Handler (Performance-Optimized)
```rust
struct ConnectionHandler {
    connection_id: usize,
    symbols: Vec<String>,
    buffer: BTreeMap<(u64, String), TickerData>, // Auto-sorted by timestamp
    file_handles: HashMap<String, BufWriter<File>>, // Buffered writers!
    reconnect_delay: Duration,
    last_flush: Instant,
}

impl ConnectionHandler {
    // 1. Subscribe to multiple symbols in one WebSocket connection
    // 2. Buffer messages in BTreeMap for automatic timestamp sorting  
    // 3. Flush buffer when: 10,000 messages OR 5 seconds elapsed
    // 4. Handle reconnection with exponential backoff (1s → 60s)
    // 5. Use 64KB BufWriter for each file to minimize syscalls
}
```

### Reconnection and Fault Tolerance (Enhanced)
- **Exponential Backoff**: Starts at 1s, doubles up to 60s max (was fixed 5s)
- **Per-Connection Isolation**: Failed connections don't affect others  
- **No Rate Limiting**: All connections created concurrently for instant startup
- **Automatic Recovery**: Resets backoff timer on successful reconnection
- **Buffer Management**: Flushes remaining messages before reconnecting
- **Connection Health**: Each connection independently monitored and recovered

## Data Format and Output

### Binary File Structure
The logger outputs data in the exact format expected by the server's memory-mapped architecture:

#### File Naming Convention
```
# Default path (Docker)
/usr/src/app/data/{symbol}/MD/{column}.{DD}.{MM}.{YY}.bin

# Legacy path (native)
/mnt/md/data/{symbol}/MD/{column}.{DD}.{MM}.{YY}.bin

Examples:
- /usr/src/app/data/BTC-USD/MD/time.07.06.25.bin
- /usr/src/app/data/BTC-USD/MD/price.07.06.25.bin
- /usr/src/app/data/ETH-USD/MD/best_bid.07.06.25.bin
- /usr/src/app/data/BTC-USD/MD/nanos.07.06.25.bin  # NEW!
```

#### Data Columns and Format (Enhanced)
```rust
// All values written as 4-byte little-endian records
time:     u32  // Unix timestamp (seconds since epoch)
nanos:    u32  // Nanosecond component (NEW - separate file)
price:    f32  // Trade price 
volume:   f32  // Trade volume (last_size)
side:     u8   // 1 = buy, 0 = sell (padded to 4 bytes)
best_bid: f32  // Current best bid price
best_ask: f32  // Current best ask price
```

#### Binary Encoding
```rust
// Little-endian encoding for x86_64 compatibility
let time_bytes = timestamp.to_le_bytes();      // [u8; 4]
let price_bytes = price.to_le_bytes();         // [u8; 4]
let volume_bytes = volume.to_le_bytes();       // [u8; 4]
let best_bid_bytes = best_bid.to_le_bytes();   // [u8; 4]
let best_ask_bytes = best_ask.to_le_bytes();   // [u8; 4]
let side_bytes = [side, 0, 0, 0];             // [u8; 4] with padding
```

### Daily File Rotation
- **Automatic Rotation**: New files created daily at midnight
- **Date Format**: `DD.MM.YY` (e.g., `07.06.25` for June 7, 2025)
- **Append Mode**: Continuous writing throughout the day
- **Sort Order**: Records naturally sorted by timestamp for server binary search

## WebSocket Integration

### Coinbase WebSocket Protocol

#### Connection Configuration
```rust
let config = WebSocketConfig {
    max_message_size: Some(64 << 20),    // 64 MB max message
    max_frame_size: Some(16 << 20),      // 16 MB max frame
    max_send_queue: Some(100),           // Queue depth
    write_buffer_size: 8191,             // 8 KiB write buffer
    max_write_buffer_size: 8192,         // Max buffer size
    accept_unmasked_frames: false,       // Security
};
```

#### Subscription Pattern
```rust
// Subscribe to ticker channel for real-time trade data
let subscribe_msg = json!({
    "type": "subscribe",
    "channels": [{
        "name": "ticker",
        "product_ids": [symbol]
    }]
});
```

#### Message Processing
```rust
// Parse ticker messages from Coinbase
if v.get("type") == Some(&serde_json::Value::String("ticker".to_string())) {
    // Extract: time, price, last_size, side, best_bid, best_ask
    // Convert to binary format
    // Write to respective files
}
```

### Data Flow Pipeline
1. **WebSocket Message**: Receive JSON ticker message from Coinbase
2. **Field Extraction**: Parse time, price, volume, side, best_bid, best_ask
3. **Type Conversion**: String → f32/u32 with error handling
4. **Binary Encoding**: Convert to little-endian byte arrays
5. **File Writing**: Append to daily binary files
6. **Error Handling**: Log failures without stopping collection

## Configuration and Customization

### Data Path Configuration
```rust
// Docker environment (default)
let base_path = format!("/usr/src/app/data/{}/MD", symbol);

// Native environment
let base_path = format!("/mnt/md/data/{}/MD", symbol);

// Directory structure created automatically:
// /usr/src/app/data/      (or /mnt/md/data/)
// ├── BTC-USD/MD/
// ├── ETH-USD/MD/
// ├── SOL-USD/MD/
// └── ...
```

### Symbol Management
- **Automatic Discovery**: Queries Coinbase for all available trading pairs
- **Online Status Filter**: Only collects data for "online" symbols
- **Dynamic Addition**: New symbols automatically detected and added
- **No Configuration Required**: Zero-config operation

### Performance Tuning
```rust
// Optimal worker thread configuration
worker_threads = 4  // Matches typical CPU core count

// WebSocket buffer sizes (32x increase for better throughput)
write_buffer_size = 262144      // 256KB (was 8KB)
max_write_buffer_size = 262144  // 256KB

// File write buffering
buf_writer_capacity = 65536     // 64KB per file

// Message buffering
max_buffer_size = 10000         // Messages before forced flush
flush_interval = 5              // Seconds between flushes
```

## Performance Characteristics

### Throughput and Latency (Dramatically Improved)
- **Startup Time**: ~1 second for 200+ symbols (was 386+ seconds)
- **Symbol Capacity**: Handles 200+ concurrent trading pairs with only 10 connections
- **Message Rate**: Processes thousands of messages per second
- **Write Latency**: Sub-millisecond buffered writes (10-100x fewer syscalls)
- **Memory Usage**: ~100MB typical with message buffering
- **CPU Usage**: Efficiently distributed across 4 worker threads

### Resource Management (Optimized)
- **File Handles**: 7 files per symbol (time, nanos, price, volume, side, best_bid, best_ask)
- **Network Connections**: 10 WebSocket connections total (was 200+)
- **Memory Buffers**: 256KB WebSocket buffers + 64KB file buffers
- **Disk I/O**: Buffered writes flush every 5 seconds or 10K messages
- **Message Ordering**: BTreeMap ensures chronological order

### Fault Tolerance (Enhanced)
- **Connection Failures**: Exponential backoff (1s → 60s) instead of fixed 5s
- **Parse Errors**: Skip malformed messages, continue processing
- **File I/O Errors**: Log errors but continue with other symbols
- **Network Issues**: Per-connection isolation (10 connections vs 200+)
- **Buffer Protection**: Always flush before reconnecting

## Integration with Server

### Binary Format Compatibility
The logger's output is designed for zero-copy integration with the server:

#### Server Memory Mapping
```rust
// Server reads these exact files via memory mapping
let file_path = format!("/mnt/md/data/{}/{}/{}.{}.bin", symbol, type_, column, date);
let mmap = unsafe { MmapOptions::new().map(&file)? };
```

#### Binary Search Compatibility
- **Sorted Data**: Records written in timestamp order
- **Fixed Size**: 4-byte records enable O(log n) binary search
- **Atomic Writes**: Each record written atomically for consistency

### Development Coordination
```bash
# Complete development stack
npm run dev:suite:full

# Data flow: Logger → Binary Files → Server → Web App
# 1. coinbase-logger collects real-time data
# 2. server memory-maps binary files
# 3. React/WASM renders live data
```

## Monitoring and Observability

### Logging Output
```rust
// Connection status
println!("Connected to Coinbase WebSocket feed for {}", symbol);

// Data processing
println!("{}: Logged record: time={} price={} volume={} side={} best_bid={} best_ask={}",
    symbol, timestamp, price, volume, side, best_bid, best_ask);

// Error handling
eprintln!("{}: Error in WebSocket connection: {}. Reconnecting in 5 seconds...", symbol, e);
```

### Health Monitoring

#### HTTP Health Check Endpoint
The logger exposes an HTTP health check endpoint that verifies the application is actively writing data:

- **Port**: 8080
- **Endpoint**: `/health`
- **Method**: GET
- **Response Codes**:
  - `200 OK`: Data files have been written within the last 60 seconds
  - `503 Service Unavailable`: No recent file writes detected
  - `500 Internal Server Error`: Error checking file status

The health check:
1. Scans the data directory for `.bin` files
2. Checks if any file has been modified within the last 60 seconds
3. Returns healthy status if recent writes are detected

This health check is used by:
- Docker's HEALTHCHECK directive
- TrueNAS Scale app health monitoring
- Kubernetes liveness/readiness probes
- External monitoring systems

Example usage:
```bash
# Check health status
curl http://localhost:8080/health

# Response when healthy:
# OK: Data files are being written

# Response when unhealthy:
# UNHEALTHY: No recent file writes detected
```

#### Traditional Monitoring
- **Connection Status**: Per-symbol connection monitoring
- **Message Rates**: Real-time throughput tracking
- **Error Rates**: Parse and connection error tracking
- **File I/O Status**: Disk write success/failure monitoring

### Production Monitoring
```bash
# Monitor active connections
ps aux | grep coinbase-logger

# Check disk usage
df -h /mnt/md/data/

# Monitor file creation
ls -la /mnt/md/data/BTC-USD/MD/

# Check recent activity
tail -f /var/log/coinbase-logger.log
```

## Common Development Tasks

### Adding New Data Fields
1. **Identify Field**: Find new field in Coinbase ticker messages
2. **Add Extraction**: Parse field from JSON message
3. **Add File Handle**: Create new binary file for the field
4. **Add Writing**: Convert to binary and write to file
5. **Update Server**: Ensure server supports the new column

### Debugging Connection Issues
```bash
# Check network connectivity
ping ws-feed.exchange.coinbase.com

# Test WebSocket manually
wscat -c wss://ws-feed.exchange.coinbase.com

# Monitor system resources
htop
iotop
```

### Performance Profiling
```bash
# CPU profiling
perf record ./target/release/coinbase-logger
perf report

# Memory profiling
valgrind --tool=massif ./target/release/coinbase-logger

# Network monitoring
netstat -an | grep 443
```

### Data Validation
```bash
# Check file sizes (should grow continuously)
ls -lh /mnt/md/data/BTC-USD/MD/

# Verify binary format
xxd -l 32 /mnt/md/data/BTC-USD/MD/time.07.06.25.bin

# Check timestamp ordering
hexdump -C /mnt/md/data/BTC-USD/MD/time.07.06.25.bin | head
```

## Troubleshooting

### Common Issues

#### WebSocket Connection Failures
```rust
// Symptoms: "Error in WebSocket connection" messages
// Solution: Check network connectivity, Coinbase API status
// Action: Logger automatically reconnects every 5 seconds
```

#### Parse Errors
```rust
// Symptoms: "Failed to parse JSON" or "Failed to parse as f32"
// Solution: Usually temporary Coinbase API issues
// Action: Logger skips malformed messages and continues
```

#### File I/O Errors
```rust
// Symptoms: "Error writing to file" messages
// Solution: Check disk space and permissions
// Action: Ensure /mnt/md/data/ is writable
```

#### Missing Symbols
```rust
// Symptoms: Expected symbols not appearing in data
// Solution: Check if symbol is "online" on Coinbase
// Action: Logger only processes symbols with status="online"
```

### Performance Issues

#### High CPU Usage
- **Cause**: Too many concurrent symbols
- **Solution**: Reduce worker thread count or implement symbol filtering
- **Monitoring**: Use `htop` to check CPU utilization

#### Disk Space Issues
- **Cause**: Continuous data accumulation
- **Solution**: Implement log rotation or archival
- **Monitoring**: Use `df -h` to check disk usage

#### Memory Usage
- **Cause**: Large message buffers or connection leaks
- **Solution**: Check WebSocket configuration and connection lifecycle
- **Monitoring**: Use `ps aux` to check memory usage

## Production Deployment

### Docker Deployment (Recommended)

#### Quick Start
```bash
# Using pre-built image from Docker Hub
docker run -d \
  --name coinbase-logger \
  -v ./data:/mnt/md/data \
  -p 8080:8080 \
  --security-opt no-new-privileges:true \
  masteryachty/coinbase-logger:latest

# Using Docker Compose (production)
docker compose -f docker-compose.prod.yml up -d

# Using Docker Compose (development)
docker compose up --build
```

#### Docker Features
- **Multi-stage Build**: Optimized ~100MB images
- **Multi-platform**: Supports linux/amd64 and linux/arm64
- **Security**: Non-root user, minimal base image, security options
- **Health Checks**: HTTP endpoint on port 8080 for monitoring
- **Resource Limits**: CPU (4 cores) and memory (2GB) limits
- **Volume Management**: Persistent data storage at `/mnt/md/data`
- **Automatic Restart**: Always restart policy
- **Port Exposure**: Port 8080 for health check endpoint

#### Docker Configuration
```yaml
# docker-compose.prod.yml
services:
  coinbase-logger:
    image: masteryachty/coinbase-logger:latest
    volumes:
      - ./data:/usr/src/app/data
    environment:
      - RUST_LOG=info
    deploy:
      resources:
        limits:
          cpus: '4'
          memory: 2G
    restart: always
```

### Native Deployment

#### System Requirements
- **OS**: Linux (x86_64)
- **CPU**: 4+ cores recommended
- **RAM**: 1GB+ available
- **Disk**: Fast storage (SSD/NVMe) for data directory
- **Network**: Stable internet connection

#### Service Configuration
```bash
# Systemd service file
[Unit]
Description=Coinbase Market Data Logger
After=network.target

[Service]
Type=simple
User=trader
WorkingDirectory=/opt/coinbase-logger
ExecStart=/opt/coinbase-logger/target/release/coinbase-logger
Restart=always
RestartSec=10
Environment="RUST_LOG=info"

[Install]
WantedBy=multi-user.target
```

### Monitoring and Alerting
- **Disk Space**: Alert when data directory usage > 90%
- **Connection Health**: Alert on extended connection failures
- **Data Freshness**: Alert if no new data written in 5+ minutes
- **Error Rates**: Alert on high parse error rates
- **Container Health**: Monitor Docker health checks

This coinbase-logger component provides the critical real-time data foundation for the entire visualization system, ensuring continuous, high-quality market data collection with maximum reliability and performance.