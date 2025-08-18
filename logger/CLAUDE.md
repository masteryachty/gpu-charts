# CLAUDE.md - Market Data Logger

This file provides comprehensive guidance to Claude Code (claude.ai/code) when working with the multi-exchange market data logger.

## Overview

The logger is a high-performance, real-time market data collection service that connects to multiple cryptocurrency exchanges via WebSocket connections, normalizes the data, and writes it to binary files for ultra-low latency consumption by the server component. It's designed for production deployment with robust error handling, automatic reconnection, and comprehensive monitoring.

## Architecture

### Core Design Principles

- **Multi-Exchange Support**: Modular architecture supporting Coinbase, Binance, Bitfinex, Kraken, and OKX
- **Real-Time WebSocket Feeds**: Low-latency data collection via persistent WebSocket connections
- **Binary File Output**: Efficient binary format optimized for memory-mapped serving
- **Multi-Threaded Design**: Each exchange runs in its own Tokio task for isolation
- **Automatic Reconnection**: Exponential backoff with configurable retry limits
- **Data Normalization**: Unified data structures across all exchanges
- **Analytics Engine**: Real-time metrics and trade analytics
- **Health Monitoring**: HTTP health check endpoint and connection monitoring

### Directory Structure

```
logger/
├── src/
│   ├── main.rs              # CLI entry point and orchestration
│   ├── lib.rs               # Library interface and Logger struct
│   ├── config.rs            # Configuration management
│   ├── common/              # Shared utilities and types
│   │   ├── data_types.rs    # Unified data structures
│   │   ├── file_handlers.rs # Binary file writing and rotation
│   │   ├── analytics.rs     # Real-time analytics engine
│   │   ├── utils.rs         # Helper utilities
│   │   └── mod.rs           # Module exports
│   └── exchanges/           # Exchange-specific implementations
│       ├── mod.rs           # Exchange trait definitions
│       ├── coinbase/        # Coinbase exchange
│       ├── binance/         # Binance exchange
│       ├── bitfinex/        # Bitfinex exchange
│       ├── kraken/          # Kraken exchange
│       └── okx/             # OKX exchange
├── tests/
│   └── integration_tests.rs # Integration test suite
├── config.yaml              # Default configuration
├── Cargo.toml               # Dependencies
└── Dockerfile               # Container image
```

## Exchange Architecture

### Exchange Trait System

The logger uses a trait-based architecture where each exchange implements the `Exchange` trait:

```rust
#[async_trait]
pub trait Exchange: Send + Sync {
    fn name(&self) -> &'static str;
    fn id(&self) -> ExchangeId;
    async fn fetch_symbols(&self) -> Result<Vec<Symbol>>;
    fn normalize_symbol(&self, exchange_symbol: &str) -> String;
    fn denormalize_symbol(&self, normalized_symbol: &str) -> String;
    async fn create_connection(...) -> Result<Box<dyn ExchangeConnection>>;
    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>>;
    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>>;
    fn max_symbols_per_connection(&self) -> usize;
    fn max_connections(&self) -> usize;
    async fn run(&self) -> Result<()>;
}
```

### Exchange Implementations

#### 1. **Coinbase** (`exchanges/coinbase/`)
- REST API for symbol discovery
- WebSocket feed for real-time data
- Supports ticker and trade channels
- Subscription-based heartbeats
- Symbol format: `BTC-USD`

#### 2. **Binance** (`exchanges/binance/`)
- REST API for symbol listing
- Combined stream WebSocket endpoint
- Requires periodic pings (20s interval)
- Symbol format: `BTCUSDT`
- Stream multiplexing support

#### 3. **Bitfinex** (`exchanges/bitfinex/`)
- Public WebSocket API v2
- Limited subscriptions per connection (15)
- Requires frequent pings (15s interval)
- Symbol format: `tBTCUSD`

#### 4. **Kraken** (`exchanges/kraken/`)
- Public WebSocket feed
- Supports batch subscriptions
- Heartbeat interval: 60s
- Symbol format: `XBT/USD`

#### 5. **OKX** (`exchanges/okx/`)
- WebSocket v5 public endpoint
- High throughput support (100 symbols/connection)
- Ping interval: 30s
- Symbol format: `BTC-USDT`

### Connection Management

Each exchange connection implements the `ExchangeConnection` trait:

```rust
#[async_trait]
pub trait ExchangeConnection: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&mut self, channels: Vec<Channel>) -> Result<()>;
    async fn read_message(&mut self) -> Result<Option<Value>>;
    async fn send_ping(&mut self) -> Result<()>;
    async fn reconnect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
    fn symbols(&self) -> &[String];
}
```

Features:
- Automatic reconnection with exponential backoff
- Concurrent connection management
- Symbol distribution across connections
- Per-exchange connection limits

## Data Types and Normalization

### Unified Data Structures

```rust
pub struct UnifiedMarketData {
    pub exchange: ExchangeId,
    pub symbol: String,
    pub timestamp: u32,      // Unix timestamp
    pub nanos: u32,          // Nanosecond precision
    pub price: f32,          // Last trade price
    pub volume: f32,         // Last trade volume
    pub side: TradeSide,     // Buy/Sell
    pub best_bid: f32,       // Best bid price
    pub best_ask: f32,       // Best ask price
}

pub struct UnifiedTradeData {
    pub exchange: ExchangeId,
    pub symbol: String,
    pub trade_id: u64,
    pub timestamp: u32,
    pub nanos: u32,
    pub price: f32,
    pub size: f32,
    pub side: TradeSide,
    pub maker_order_id: [u8; 16],  // UUID bytes
    pub taker_order_id: [u8; 16],  // UUID bytes
}
```

### Symbol Information

```rust
pub struct Symbol {
    pub exchange: ExchangeId,
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub asset_class: AssetClass,
    pub active: bool,
    pub min_size: Option<f64>,
    pub tick_size: Option<f64>,
}
```

## File Output System

### Binary File Format

The logger writes data in a columnar binary format optimized for memory-mapped access:

#### Directory Structure
```
/mnt/md/data/
├── {exchange}/
│   └── {symbol}/
│       ├── MD/                    # Market data
│       │   ├── time.{DD}.{MM}.{YY}.bin
│       │   ├── nanos.{DD}.{MM}.{YY}.bin
│       │   ├── price.{DD}.{MM}.{YY}.bin
│       │   ├── volume.{DD}.{MM}.{YY}.bin
│       │   ├── side.{DD}.{MM}.{YY}.bin
│       │   ├── best_bid.{DD}.{MM}.{YY}.bin
│       │   └── best_ask.{DD}.{MM}.{YY}.bin
│       └── TRADES/                # Trade data
│           ├── id.{DD}.{MM}.{YY}.bin
│           ├── time.{DD}.{MM}.{YY}.bin
│           ├── nanos.{DD}.{MM}.{YY}.bin
│           ├── price.{DD}.{MM}.{YY}.bin
│           ├── size.{DD}.{MM}.{YY}.bin
│           ├── side.{DD}.{MM}.{YY}.bin
│           ├── maker_order_id.{DD}.{MM}.{YY}.bin
│           └── taker_order_id.{DD}.{MM}.{YY}.bin
```

#### Binary Encoding
- All numeric values use little-endian encoding
- `u32` for timestamps and sides (4 bytes)
- `f32` for prices and volumes (4 bytes)
- `u64` for trade IDs (8 bytes)
- UUID as raw 16-byte arrays

### File Management

The `FileHandlerManager` provides:
- Automatic file rotation at midnight UTC
- Buffered writes (64KB buffers)
- Concurrent file access via DashMap
- Retry logic for I/O errors
- Periodic flush intervals

## Analytics and Monitoring

### Analytics Engine

Real-time trade analytics with:
- Volume-weighted average price (VWAP)
- High/low price tracking
- Buy/sell volume ratios
- Large trade detection
- Trade count statistics
- Configurable reporting intervals

### Market Metrics

Connection and performance monitoring:
- Messages per second per exchange
- Connection health status
- Reconnection count tracking
- Error logging and tracking
- Last message timestamps

### Health Check Server

HTTP endpoint on port 8080:
- `/health` - Returns JSON health status
- Used for Docker health checks
- Kubernetes liveness probes

## Configuration System

### Configuration File (`config.yaml`)

```yaml
logger:
  data_path: "/mnt/md/data"
  buffer_size: 8192
  flush_interval_secs: 5
  health_check_port: 8080

exchanges:
  coinbase:
    enabled: true
    ws_endpoint: "wss://ws-feed.exchange.coinbase.com"
    rest_endpoint: "https://api.exchange.coinbase.com"
    max_connections: 15
    symbols_per_connection: 50
    reconnect_delay_secs: 1
    max_reconnect_delay_secs: 60
    symbols: []  # Optional: specific symbols to monitor
```

### Environment Variables

Configuration can be overridden via environment variables:
- `LOGGER_DATA_PATH` - Base directory for data files
- `LOGGER_EXCHANGES_COINBASE_ENABLED` - Enable/disable exchange
- `LOGGER_EXCHANGES_BINANCE_MAX_CONNECTIONS` - Connection limits

## CLI Interface

### Commands

```bash
# Run with default configuration
logger

# Run specific exchanges
logger run --exchanges coinbase,binance

# Test exchange connectivity
logger test coinbase

# List available symbols
logger symbols binance

# Custom config file
logger --config /path/to/config.yaml run

# Enable debug logging
logger --debug run
```

### Command-Line Options
- `--config` - Custom configuration file path
- `--debug` - Enable debug-level logging
- `--exchanges` - Comma-separated list of exchanges

## Multi-Threading Architecture

### Concurrency Model

1. **Main Thread**: CLI interface and orchestration
2. **Exchange Tasks**: One Tokio task per exchange
3. **Connection Tasks**: Multiple tasks per exchange for WebSocket connections
4. **File I/O Tasks**: Async file operations with buffering
5. **Analytics Task**: Periodic reporting and metrics
6. **Health Server**: Dedicated HTTP server task

### Data Flow

1. WebSocket messages received by connection tasks
2. Parsed into unified data structures
3. Sent via MPSC channels to processing tasks
4. Buffered in memory (BTreeMap for ordering)
5. Periodically flushed to binary files
6. Analytics updated in real-time

## Error Handling

### Reconnection Strategy

- Exponential backoff starting at 1 second
- Maximum delay configurable per exchange
- Automatic symbol re-subscription
- Connection health tracking

### I/O Error Handling

- Retry logic for file operations
- Partial flush success handling
- File rotation on date changes
- Graceful degradation

### Message Parsing

- Silent dropping of unknown message types
- Error logging for malformed data
- Exchange-specific error handling

## Testing

### Unit Tests
- Data type conversions
- File handler operations
- Analytics calculations
- Symbol distribution

### Integration Tests (`tests/integration_tests.rs`)
- End-to-end data flow
- Buffer and flush operations
- Analytics engine
- Configuration loading
- Message flow simulation

### Running Tests
```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test -- --nocapture

# Run specific test
cargo test test_data_buffer_integration
```

## Docker Deployment

### Building the Image

```dockerfile
# Multi-stage build
FROM rust:1.82-slim AS builder
# Build stage...

FROM debian:bookworm-slim
# Runtime stage...
```

### Running the Container

```bash
# Build image
docker build -t market-logger .

# Run with volume mount
docker run -v /mnt/md/data:/mnt/md/data \
  -p 8080:8080 \
  -e LOGGER_EXCHANGES_COINBASE_ENABLED=true \
  market-logger
```

### Health Checks

```bash
# Docker health check
curl -f http://localhost:8080/health

# Kubernetes liveness probe
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 60
  periodSeconds: 30
```

## Integration with Server

The logger produces binary files that are consumed by the server component:

1. **File Format**: Compatible with server's memory-mapped file reader
2. **Directory Structure**: Matches server's expected paths
3. **Data Types**: 4-byte aligned for efficient memory mapping
4. **File Rotation**: Daily files matching server's date format

### Data Pipeline

```
Exchange WebSocket → Logger → Binary Files → Server (mmap) → Client API
```

## Performance Considerations

### Optimization Strategies

1. **Buffered Writes**: 64KB buffers reduce syscalls
2. **Columnar Storage**: Efficient memory-mapped access
3. **Binary Format**: No parsing overhead in server
4. **Concurrent Processing**: Multi-threaded architecture
5. **Lock-Free Structures**: DashMap for concurrent access

### Resource Requirements

- **Memory**: ~500MB base + buffer allocations
- **CPU**: 1-2 cores for normal operation
- **Disk I/O**: Sequential writes, periodic flushes
- **Network**: WebSocket connections to exchanges

## Common Operations

### Adding a New Exchange

1. Create new module in `src/exchanges/{name}/`
2. Implement `Exchange` and `ExchangeConnection` traits
3. Add configuration in `config.rs`
4. Register in `lib.rs` Logger initialization
5. Add to CLI in `main.rs`
6. Update config.yaml template

### Monitoring Production

```bash
# Check health
curl http://localhost:8080/health

# View logs
docker logs market-logger

# Check file generation
ls -la /mnt/md/data/{exchange}/{symbol}/MD/

# Monitor message rates
grep "messages/sec" logs.txt
```

### Troubleshooting

1. **Connection Issues**: Check network, exchange status, credentials
2. **File I/O Errors**: Verify permissions, disk space
3. **High Memory**: Adjust buffer sizes, flush intervals
4. **Missing Data**: Check symbol configuration, exchange availability

## Best Practices

1. **Configuration**: Use config file for production, env vars for overrides
2. **Monitoring**: Always run health checks in production
3. **Logging**: Use appropriate log levels (info for production)
4. **Deployment**: Use Docker for consistent environment
5. **Data Retention**: Implement cleanup for old files
6. **Testing**: Test exchange connections before production deployment

## Security Considerations

1. **Non-Root User**: Container runs as unprivileged user
2. **Network**: Uses TLS for all WebSocket connections
3. **File Permissions**: Restricted access to data directory
4. **No Credentials**: Public data feeds only
5. **Input Validation**: All exchange data validated before processing