# Exchange Integration Plan

## Overview
This plan outlines the integration of three additional cryptocurrency exchanges (OKX, Kraken, Bitfinex) into the existing logger system. The system already supports Coinbase and Binance.

## Top 5 Exchanges Selected
1. **Binance** ✓ (Already implemented)
2. **Coinbase** ✓ (Already implemented)
3. **OKX** - To be implemented
4. **Kraken** - To be implemented
5. **Bitfinex** - To be implemented

## Architecture Pattern
Each exchange follows the same implementation pattern:
- Implements the `Exchange` trait
- Has its own module under `src/exchanges/`
- Contains:
  - `mod.rs` - Main exchange implementation
  - `connection.rs` - WebSocket connection handling
  - `parser.rs` - Message parsing logic

## Exchange API Details

### OKX
- **WebSocket**: `wss://ws.okx.com:8443/ws/v5/public`
- **REST API**: `https://www.okx.com/api/v5/`
- **Features**:
  - Supports up to 100 channels per connection
  - Requires ping/pong every 30 seconds
  - Message format: JSON with channel-based routing

### Kraken
- **WebSocket**: `wss://ws.kraken.com`
- **REST API**: `https://api.kraken.com/0/`
- **Features**:
  - Supports multiple subscriptions per connection
  - Heartbeat/ping interval: 60 seconds
  - Message format: JSON array format

### Bitfinex
- **WebSocket**: `wss://api-pub.bitfinex.com/ws/2`
- **REST API**: `https://api-pub.bitfinex.com/v2/`
- **Features**:
  - Supports up to 30 subscriptions per connection
  - Requires ping every 15 seconds
  - Message format: JSON with event-based routing

## Implementation Steps

### 1. Configuration Updates
Add exchange configurations to `src/config.rs`:
- OKX configuration with WebSocket and REST endpoints
- Kraken configuration with appropriate limits
- Bitfinex configuration with connection parameters

### 2. Exchange Implementations
Create modules for each exchange:
- `src/exchanges/okx/`
- `src/exchanges/kraken/`
- `src/exchanges/bitfinex/`

### 3. Symbol Normalization
Each exchange has different symbol formats:
- OKX: `BTC-USDT`
- Kraken: `XBT/USD` (uses XBT for Bitcoin)
- Bitfinex: `tBTCUSD`

### 4. Testing Strategy
- Unit tests for parsers
- Integration tests for connections
- Verification of data file creation
- Data format validation

## Success Criteria
1. All three exchanges successfully connect and stream data
2. Data files are created in the expected format
3. Symbol normalization works correctly
4. Error handling and reconnection logic functions properly
5. All tests pass