# Current Data Types in Coinbase Logger

## Overview

The coinbase-logger currently collects and stores two types of data from Coinbase:

### 1. Market Data (MD)
- **Source**: Ticker channel WebSocket subscription
- **Directory**: `/mnt/md/data/{symbol}/MD/`
- **Purpose**: Real-time market snapshots including best bid/ask prices
- **Update Frequency**: ~1 update per second per symbol
- **Files**:
  - `time.{DD}.{MM}.{YY}.bin` - Unix timestamp (seconds)
  - `nanos.{DD}.{MM}.{YY}.bin` - Nanosecond component
  - `price.{DD}.{MM}.{YY}.bin` - Last trade price
  - `volume.{DD}.{MM}.{YY}.bin` - Last trade volume
  - `side.{DD}.{MM}.{YY}.bin` - Last trade side (1=buy, 0=sell)
  - `best_bid.{DD}.{MM}.{YY}.bin` - Current best bid price
  - `best_ask.{DD}.{MM}.{YY}.bin` - Current best ask price

### 2. Market Trades (TRADES)
- **Source**: Matches channel WebSocket subscription
- **Directory**: `/mnt/md/data/{symbol}/TRADES/`
- **Purpose**: Complete record of every individual trade executed
- **Update Frequency**: Every trade (can be hundreds per second for active symbols)
- **Files**:
  - `trade_id.{DD}.{MM}.{YY}.bin` - Unique trade identifier (8 bytes)
  - `trade_time.{DD}.{MM}.{YY}.bin` - Unix timestamp (seconds)
  - `trade_nanos.{DD}.{MM}.{YY}.bin` - Nanosecond component
  - `trade_price.{DD}.{MM}.{YY}.bin` - Trade execution price
  - `trade_size.{DD}.{MM}.{YY}.bin` - Trade volume/size
  - `trade_side.{DD}.{MM}.{YY}.bin` - Buy/sell indicator (1=buy, 0=sell)
  - `maker_order_id.{DD}.{MM}.{YY}.bin` - Maker's order UUID (16 bytes)
  - `taker_order_id.{DD}.{MM}.{YY}.bin` - Taker's order UUID (16 bytes)

## API Access

Both data types can be accessed through the server API:

### Market Data (MD)
```
https://localhost:8443/api/data?symbol=BTC-USD&type=MD&start=1753400000&end=1753410000&columns=time,price,best_bid,best_ask
```

### Market Trades (TRADES)
```
https://localhost:8443/api/data?symbol=BTC-USD&type=TRADES&start=1753400000&end=1753410000&columns=trade_time,trade_price,trade_size,trade_side
```

## Binary Format

All data files use little-endian encoding:
- Most fields: 4-byte records
- `trade_id`: 8-byte records
- `maker_order_id`, `taker_order_id`: 16-byte records
- `side` fields: 1 byte padded to 4 bytes for consistency

Files are append-only and automatically rotate daily at midnight.