# Phase 2: Market Trades Channel Integration

## Overview

This phase adds support for Coinbase's `market_trades` WebSocket channel, providing access to all individual trades rather than just the last trade from the ticker channel. This enables comprehensive trade analysis and visualization.

## Goals

1. Subscribe to `market_trades` channel alongside `ticker`
2. Process and store all individual trades
3. Handle increased data volume efficiently
4. Maintain correlation between ticker and trade data
5. Enable advanced trade visualization features

## Technical Design

### New Data Structures

```rust
// Add to data_types.rs
#[derive(Clone, Debug, PartialEq)]
pub struct MarketTradeData {
    pub trade_id: u64,
    pub timestamp_secs: u32,
    pub timestamp_nanos: u32,
    pub price: f32,
    pub size: f32,
    pub side: u8,  // 1 = buy, 0 = sell
    pub maker_order_id: [u8; 16],  // UUID as bytes
    pub taker_order_id: [u8; 16],  // UUID as bytes
}

#[derive(Clone, Debug)]
pub enum MarketMessage {
    Ticker(TickerData),
    Trade(MarketTradeData),
}
```

### File Structure

```
/mnt/md/data/{symbol}/
├── MD/                    # Existing ticker data
├── TICKER_TRADES/         # Phase 1 trade data
└── TRADES/                # New individual trades
    ├── trade_id.{date}.bin        # 8 bytes per record
    ├── trade_time.{date}.bin      # 4 bytes per record
    ├── trade_nanos.{date}.bin     # 4 bytes per record
    ├── trade_price.{date}.bin     # 4 bytes per record
    ├── trade_size.{date}.bin      # 4 bytes per record
    ├── trade_side.{date}.bin      # 4 bytes per record
    ├── maker_order_id.{date}.bin  # 16 bytes per record
    └── taker_order_id.{date}.bin  # 16 bytes per record
```

### WebSocket Subscription Update

```rust
// Updated subscription message
let subscribe_msg = json!({
    "type": "subscribe",
    "channels": [
        {
            "name": "ticker",
            "product_ids": self.symbols
        },
        {
            "name": "market_trades",
            "product_ids": self.symbols
        }
    ]
});
```

### Enhanced Buffer Management

```rust
pub struct ConnectionHandler {
    // Existing fields...
    pub ticker_buffer: BTreeMap<(u64, String), TickerData>,
    pub trade_buffer: BTreeMap<(u64, String), MarketTradeData>,
    pub trades_file_handles: HashMap<String, TradeFileHandles>,
}

pub struct TradeFileHandles {
    pub trade_id_file: BufWriter<File>,
    pub trade_time_file: BufWriter<File>,
    pub trade_nanos_file: BufWriter<File>,
    pub trade_price_file: BufWriter<File>,
    pub trade_size_file: BufWriter<File>,
    pub trade_side_file: BufWriter<File>,
    pub maker_order_id_file: BufWriter<File>,
    pub taker_order_id_file: BufWriter<File>,
}
```

## Implementation Tasks

### Task 1: Update WebSocket Configuration
1. Increase buffer sizes for higher message volume
2. Update `create_websocket_config()` with larger limits
3. Consider message queue depth for trades

### Task 2: Extend Data Types
1. Add `MarketTradeData` struct
2. Implement UUID to bytes conversion
3. Add `MarketMessage` enum for unified processing
4. Create trade validation methods

### Task 3: Update Connection Handler
1. Add `trades_file_handles` HashMap
2. Create separate trade buffer with higher capacity
3. Implement dual-channel subscription
4. Add trade-specific error handling

### Task 4: Implement Market Trades Processing
1. Parse market_trades messages
2. Extract trade fields and convert types
3. Handle trade_id as u64
4. Convert UUID strings to byte arrays
5. Add to trade buffer with nanosecond precision

### Task 5: Enhance Buffer Management
1. Separate flush logic for trades vs ticker
2. Implement trade buffer size limits (50K trades)
3. Add emergency flush on memory pressure
4. Optimize BTreeMap usage for trades

### Task 6: Update File Writing
1. Create TRADES directory structure
2. Implement 8-byte trade_id writes
3. Handle 16-byte UUID writes
4. Ensure atomic multi-file writes

## Code Implementation

### 1. WebSocket Configuration Update:
```rust
pub fn create_websocket_config() -> WebSocketConfig {
    WebSocketConfig {
        max_message_size: Some(128 << 20),    // 128 MB for trades
        max_frame_size: Some(32 << 20),       // 32 MB frames
        write_buffer_size: 512 * 1024,        // 512KB buffer
        max_write_buffer_size: 1024 * 1024,   // 1MB max
        max_send_queue: Some(1000),           // Larger queue
        ..Default::default()
    }
}
```

### 2. Message Processing:
```rust
async fn process_message(&mut self, text: &str) -> Result<()> {
    let v: serde_json::Value = serde_json::from_str(text)?;
    
    match v.get("type").and_then(|t| t.as_str()) {
        Some("ticker") => self.process_ticker_message(&v).await?,
        Some("match") | Some("last_match") => self.process_trade_message(&v).await?,
        Some("subscriptions") => {
            println!("Subscription confirmed: {:?}", v);
        }
        _ => {}
    }
    Ok(())
}

async fn process_trade_message(&mut self, v: &serde_json::Value) -> Result<()> {
    if let (
        Some(product_id),
        Some(trade_id),
        Some(time_str),
        Some(price_str),
        Some(size_str),
        Some(side_str),
        maker_order_id,
        taker_order_id,
    ) = (
        v.get("product_id").and_then(|v| v.as_str()),
        v.get("trade_id").and_then(|v| v.as_u64()),
        v.get("time").and_then(|v| v.as_str()),
        v.get("price").and_then(|v| v.as_str()),
        v.get("size").and_then(|v| v.as_str()),
        v.get("side").and_then(|v| v.as_str()),
        v.get("maker_order_id").and_then(|v| v.as_str()),
        v.get("taker_order_id").and_then(|v| v.as_str()),
    ) {
        // Parse and process trade data
        // Add to trade_buffer
    }
    Ok(())
}
```

### 3. UUID Handling:
```rust
fn uuid_to_bytes(uuid_str: &str) -> Result<[u8; 16]> {
    // Remove hyphens and decode hex
    let clean = uuid_str.replace("-", "");
    let bytes = hex::decode(&clean)?;
    let mut array = [0u8; 16];
    array.copy_from_slice(&bytes);
    Ok(array)
}
```

## Performance Optimizations

### Buffer Strategy
1. Separate buffers for ticker and trades
2. Trade buffer: 50K capacity before forced flush
3. Flush trades every 2 seconds (more frequent than ticker)
4. Use separate tokio tasks for processing

### Memory Management
1. Pre-allocate buffers based on symbol count
2. Monitor memory usage and trigger emergency flushes
3. Clear processed trades immediately
4. Use `Box` for large trade structures

### I/O Optimization
1. Batch write operations per symbol
2. Use larger file buffers (128KB) for trades
3. Async parallel writes across symbols
4. Consider memory-mapped files for extreme volume

## Testing Strategy

### Load Testing
1. Simulate 1000 trades/second per symbol
2. Monitor memory usage under load
3. Verify no message drops
4. Check file write performance

### Data Validation
1. Compare trade_ids for uniqueness
2. Verify chronological ordering
3. Match trades with ticker updates
4. Validate UUID conversions

### Integration Testing
1. Run with high-volume symbols (BTC-USD)
2. Verify both channels receive data
3. Check correlation between ticker and trades
4. Monitor WebSocket stability

## Monitoring and Metrics

### Key Metrics
1. Trades per second per symbol
2. Trade buffer size and flush frequency
3. WebSocket message queue depth
4. File write latency
5. Memory usage by component

### Alerts
1. Trade buffer > 80% capacity
2. WebSocket disconnections
3. File write failures
4. Memory usage > 2GB
5. Trade processing lag > 1 second

## Migration Strategy

### Rollout Plan
1. Deploy with single test symbol first
2. Monitor for 24 hours
3. Gradually add more symbols
4. Full deployment after validation

### Rollback Plan
1. Remove market_trades subscription
2. Continue with ticker-only mode
3. Clean up TRADES directories
4. Revert WebSocket config changes

## Success Criteria

1. Process 100% of market trades
2. No WebSocket disconnections due to load
3. File writes complete within 100ms
4. Memory usage stable under 2GB
5. Trade data queryable by server

## Known Challenges

### Message Volume
- BTC-USD can generate 100+ trades/second
- Total volume across symbols: 1000+ trades/second
- Requires efficient buffering and I/O

### Order ID Handling
- UUIDs require 16 bytes storage
- Consider compression or indexing
- May need separate lookup tables

### Timestamp Precision
- Trades may have same timestamp
- Use trade_id for additional ordering
- Consider microsecond precision

## Next Steps

After successful implementation:
1. Update server to read TRADES data
2. Implement trade aggregation views
3. Add real-time trade streaming
4. Prepare for Phase 3 advanced features