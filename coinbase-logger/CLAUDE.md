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

### Multi-Threading Model (Improved)
```rust
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Discover all available trading pairs via status channel
    // 2. Create 10 connection handlers, each managing ~20 symbols
    // 3. Each connection subscribes to multiple symbols at once
    // 4. Rate-limited connection creation (1 per second)
}
```

**Key Features:**
- **Connection Pooling**: 10 connections handle 200+ symbols (20x reduction)
- **Multi-Symbol Subscriptions**: Each connection subscribes to 20 symbols
- **Message Buffering**: BTreeMap automatically sorts messages by timestamp
- **Exponential Backoff**: Smart reconnection with delays from 1s to 60s
- **Nanosecond Precision**: Full timestamp precision preserved in separate files

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

#### Connection Handler (Improved)
```rust
struct ConnectionHandler {
    connection_id: usize,
    symbols: Vec<String>,
    buffer: BTreeMap<(u64, String), TickerData>, // Auto-sorted by timestamp
    file_handles: HashMap<String, FileHandles>,
    reconnect_delay: Duration,
}

impl ConnectionHandler {
    // 1. Subscribe to multiple symbols in one WebSocket connection
    // 2. Buffer messages in BTreeMap for automatic timestamp sorting  
    // 3. Flush buffer every second with messages in chronological order
    // 4. Handle reconnection with exponential backoff
}
```

### Reconnection and Fault Tolerance (Enhanced)
- **Exponential Backoff**: Starts at 1s, doubles up to 60s max
- **Per-Connection Isolation**: Failed connections don't affect others  
- **Rate-Limited Connections**: Respects 1 connection/second limit
- **Automatic Recovery**: Resets backoff timer on successful reconnection
- **Buffer Management**: Flushes remaining messages before reconnecting

## Data Format and Output

### Binary File Structure
The logger outputs data in the exact format expected by the server's memory-mapped architecture:

#### File Naming Convention
```
/mnt/md/data/{symbol}/MD/{column}.{DD}.{MM}.{YY}.bin

Examples:
- /mnt/md/data/BTC-USD/MD/time.07.06.25.bin
- /mnt/md/data/BTC-USD/MD/price.07.06.25.bin
- /mnt/md/data/ETH-USD/MD/best_bid.07.06.25.bin
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
// Default data path (matches server expectations)
let base_path = format!("/mnt/md/data/{}/MD", symbol);

// Directory structure created automatically:
// /mnt/md/data/
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

// WebSocket buffer sizes tuned for high-frequency data
write_buffer_size = 8191
max_frame_size = 16 MB
```

## Performance Characteristics

### Throughput and Latency
- **Symbol Capacity**: Handles 200+ concurrent trading pairs
- **Message Rate**: Processes thousands of messages per second
- **Write Latency**: Sub-millisecond binary file writes
- **Memory Usage**: Minimal memory footprint with streaming processing
- **CPU Usage**: Efficiently distributed across 4 worker threads

### Resource Management
- **File Handles**: 6 files per symbol (time, price, volume, side, best_bid, best_ask)
- **Network Connections**: 1 WebSocket per symbol for isolation
- **Memory Buffers**: Small per-connection buffers for efficiency
- **Disk I/O**: Sequential append-only writes for maximum performance

### Fault Tolerance
- **Connection Failures**: Automatic reconnection every 5 seconds
- **Parse Errors**: Skip malformed messages, continue processing
- **File I/O Errors**: Log errors but continue with other symbols
- **Network Issues**: Per-symbol isolation prevents cascading failures

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

### System Requirements
- **OS**: Linux (x86_64)
- **CPU**: 4+ cores recommended
- **RAM**: 1GB+ available
- **Disk**: Fast storage (SSD/NVMe) for `/mnt/md/data/`
- **Network**: Stable internet connection

### Service Configuration
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

[Install]
WantedBy=multi-user.target
```

### Monitoring and Alerting
- **Disk Space**: Alert when `/mnt/md/data/` usage > 90%
- **Connection Health**: Alert on extended connection failures
- **Data Freshness**: Alert if no new data written in 5+ minutes
- **Error Rates**: Alert on high parse error rates

This coinbase-logger component provides the critical real-time data foundation for the entire visualization system, ensuring continuous, high-quality market data collection with maximum reliability and performance.