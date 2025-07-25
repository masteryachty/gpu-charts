# Phase 1: Enhanced Ticker Trade Logging

> **⚠️ DEPRECATED**: This feature has been removed from the codebase. The ticker_trades functionality was found to be redundant with the more comprehensive market trades (Phase 2) feature. This documentation is preserved for historical reference only.
>
> **Current Status**: 
> - All ticker_trades code has been removed
> - Only market trades (TRADES) data is now collected
> - The ticker channel is used only for market data (MD)

## Overview

This phase enhances the existing ticker channel subscription to extract and store trade-specific data in a format optimized for chart visualization. We'll create dedicated trade data files while maintaining backward compatibility with the existing system.

## Goals

1. Extract trade data from existing ticker messages
2. Create separate binary files for trade-specific data
3. Add trade metadata tracking (counts, cumulative volume)
4. Enable basic trade marking on charts without new WebSocket subscriptions

## Technical Design

### New Data Structure

```rust
// Add to data_types.rs
#[derive(Clone, Debug, PartialEq)]
pub struct TickerTradeData {
    pub timestamp_secs: u32,
    pub timestamp_nanos: u32,
    pub trade_price: f32,
    pub trade_volume: f32,
    pub trade_side: u8,  // 1 = buy, 0 = sell
    pub spread: f32,     // best_ask - best_bid at time of trade
}
```

### File Structure

```
/mnt/md/data/{symbol}/
├── MD/                    # Existing market data
│   ├── time.{date}.bin
│   ├── price.{date}.bin
│   ├── volume.{date}.bin
│   ├── side.{date}.bin
│   ├── best_bid.{date}.bin
│   └── best_ask.{date}.bin
└── TICKER_TRADES/         # New trade-specific data
    ├── trade_time.{date}.bin
    ├── trade_nanos.{date}.bin
    ├── trade_price.{date}.bin
    ├── trade_volume.{date}.bin
    ├── trade_side.{date}.bin
    └── trade_spread.{date}.bin
```

### Enhanced FileHandles Structure

```rust
pub struct FileHandles {
    // Existing fields...
    
    // New trade-specific handles
    pub trade_time_file: BufWriter<File>,
    pub trade_nanos_file: BufWriter<File>,
    pub trade_price_file: BufWriter<File>,
    pub trade_volume_file: BufWriter<File>,
    pub trade_side_file: BufWriter<File>,
    pub trade_spread_file: BufWriter<File>,
}
```

## Implementation Tasks

### Task 1: Update Data Types
1. Add `TickerTradeData` struct to `data_types.rs`
2. Add helper methods for spread calculation
3. Add trade data validation methods

### Task 2: Extend File Handlers
1. Update `FileHandles` struct in `file_handlers.rs`
2. Add trade file initialization in `create_file_handles_for_symbol`
3. Update `close()` and `flush_all()` methods to include trade files
4. Create `TICKER_TRADES` directory structure

### Task 3: Enhance Message Processing
1. Modify `process_message()` in `connection.rs` to extract trade data
2. Calculate spread from best_bid and best_ask
3. Store both market data and trade data from ticker messages
4. Add trade data to buffer with same timestamp key

### Task 4: Update Buffer Flushing
1. Extend `flush_buffer()` to write trade files
2. Ensure atomic writes for trade data
3. Add error handling for trade file writes

### Task 5: Add Trade Metadata Tracking
1. Create daily trade count file: `trade_count.{date}.bin`
2. Track cumulative volume in `cumulative_volume.{date}.bin`
3. Update metadata files during buffer flush

## Code Changes

### 1. data_types.rs additions:
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct TickerTradeData {
    pub timestamp_secs: u32,
    pub timestamp_nanos: u32,
    pub trade_price: f32,
    pub trade_volume: f32,
    pub trade_side: u8,
    pub spread: f32,
}

impl TickerTradeData {
    pub fn from_ticker(ticker: &TickerData) -> Self {
        Self {
            timestamp_secs: ticker.timestamp_secs,
            timestamp_nanos: ticker.timestamp_nanos,
            trade_price: ticker.price,
            trade_volume: ticker.volume,
            trade_side: ticker.side,
            spread: ticker.best_ask - ticker.best_bid,
        }
    }
}
```

### 2. file_handlers.rs modifications:
```rust
impl FileHandles {
    pub async fn create_with_trades(base_path: &str, date: &str) -> Result<Self> {
        // Create both MD and TICKER_TRADES directories
        tokio::fs::create_dir_all(&format!("{}/MD", base_path)).await?;
        tokio::fs::create_dir_all(&format!("{}/TICKER_TRADES", base_path)).await?;
        
        // Initialize all file handles including trade files
        // ...
    }
}
```

### 3. connection.rs buffer enhancement:
```rust
pub struct ConnectionHandler {
    // Existing fields...
    pub trade_buffer: BTreeMap<(u64, String), TickerTradeData>,
}
```

## Testing Strategy

### Unit Tests
1. Test `TickerTradeData` creation from `TickerData`
2. Test spread calculation with various bid/ask values
3. Test trade file creation and writing
4. Test buffer sorting and flushing

### Integration Tests
1. Mock ticker messages with trade data
2. Verify trade files are created correctly
3. Check file format compatibility
4. Validate timestamp ordering in trade files

### Manual Testing
1. Run logger with single symbol (e.g., BTC-USD)
2. Verify TICKER_TRADES directory creation
3. Use `xxd` to inspect binary trade files
4. Compare trade data with ticker data for consistency

## Performance Considerations

### Memory Impact
- Additional buffer for trade data: ~50MB for 10K trades
- 6 additional file handles per symbol
- Minimal overhead as we're reusing existing ticker data

### I/O Impact
- 6 additional file writes per flush
- Same flush frequency (5 seconds)
- Buffered writes minimize syscalls

### CPU Impact
- Negligible: Only adding spread calculation
- No additional parsing or network operations

## Monitoring and Validation

### Metrics to Track
1. Trade count per symbol per minute
2. Average spread per symbol
3. Buy/sell ratio
4. Trade file sizes

### Validation Checks
1. Trade timestamps match ticker timestamps
2. Trade prices fall within bid/ask spread
3. Volume is positive
4. Side is valid (0 or 1)

## Rollback Plan

If issues arise:
1. Remove TICKER_TRADES directory creation
2. Comment out trade file writes
3. Original MD files remain unaffected
4. No WebSocket subscription changes needed

## Success Criteria

1. Trade files created successfully for all symbols
2. No performance degradation
3. Trade data visible in binary files
4. Backward compatibility maintained
5. Server can read trade files for charting

## Next Steps

After successful implementation:
1. Update server to read TICKER_TRADES data
2. Add trade markers to chart renderer
3. Monitor for data quality issues
4. Prepare for Phase 2 (market_trades channel)