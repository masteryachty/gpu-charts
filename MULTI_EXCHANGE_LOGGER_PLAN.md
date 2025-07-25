# Multi-Exchange Logger Refactoring Plan

## Executive Summary

This document outlines the comprehensive plan to refactor the current `coinbase-logger` into a flexible `logger` system that supports multiple cryptocurrency exchanges, starting with Binance integration. The refactoring will maintain high performance while introducing a unified data format and modular architecture.

## Core Objectives

1. **Multi-Exchange Support**: Create a plugin-based architecture supporting multiple exchanges
2. **Unified Data Model**: Standardize data formats across all exchanges for easy comparison
3. **Maintainable Architecture**: Modular design allowing easy addition of new exchanges
4. **Performance**: Maintain sub-millisecond latency and high-throughput data collection
5. **Data Integrity**: Ensure consistent, reliable data storage across all exchanges

## Proposed Architecture

### 1. New Directory Structure

```
logger/
├── Cargo.toml
├── src/
│   ├── main.rs                      # Main entry point with exchange selection
│   ├── lib.rs                       # Library exports
│   ├── config.rs                    # Configuration management
│   ├── common/                      # Shared functionality
│   │   ├── mod.rs
│   │   ├── data_types.rs            # Unified data models
│   │   ├── file_handlers.rs         # Generic file handling
│   │   ├── analytics.rs             # Common analytics
│   │   └── utils.rs                 # Shared utilities
│   └── exchanges/                   # Exchange-specific implementations
│       ├── mod.rs                   # Exchange trait definition
│       ├── coinbase/
│       │   ├── mod.rs
│       │   ├── connection.rs        # Coinbase WebSocket handler
│       │   ├── parser.rs            # Coinbase-specific parsing
│       │   └── config.rs            # Coinbase configuration
│       └── binance/
│           ├── mod.rs
│           ├── connection.rs        # Binance WebSocket handler
│           ├── parser.rs            # Binance-specific parsing
│           └── config.rs            # Binance configuration
│
└── tests/
    ├── integration_tests.rs
    └── data_consistency_tests.rs
```

### 2. New File Storage Structure

```
/mnt/md/data/{exchange}/{symbol}/
├── MD/                              # Market Data
│   ├── time.{DD}.{MM}.{YY}.bin     # Unix timestamps (4 bytes)
│   ├── nanos.{DD}.{MM}.{YY}.bin    # Nanosecond precision (4 bytes)
│   ├── price.{DD}.{MM}.{YY}.bin    # Last trade price (4 bytes float)
│   ├── volume.{DD}.{MM}.{YY}.bin   # Last trade volume (4 bytes float)
│   ├── side.{DD}.{MM}.{YY}.bin     # Buy/sell indicator (4 bytes)
│   ├── best_bid.{DD}.{MM}.{YY}.bin # Best bid price (4 bytes float)
│   └── best_ask.{DD}.{MM}.{YY}.bin # Best ask price (4 bytes float)
│
└── TRADES/                          # Individual trades
    ├── trade_id.{DD}.{MM}.{YY}.bin      # Unique ID (8 bytes)
    ├── trade_time.{DD}.{MM}.{YY}.bin    # Unix timestamp (4 bytes)
    ├── trade_nanos.{DD}.{MM}.{YY}.bin   # Nanoseconds (4 bytes)
    ├── trade_price.{DD}.{MM}.{YY}.bin   # Price (4 bytes float)
    ├── trade_size.{DD}.{MM}.{YY}.bin    # Size (4 bytes float)
    ├── trade_side.{DD}.{MM}.{YY}.bin    # Side (4 bytes)
    ├── maker_order_id.{DD}.{MM}.{YY}.bin # UUID (16 bytes)
    └── taker_order_id.{DD}.{MM}.{YY}.bin # UUID (16 bytes)
```

Example paths:
- `/mnt/md/data/coinbase/BTC-USD/MD/`
- `/mnt/md/data/binance/BTCUSDT/MD/`

## Unified Data Models

### 1. Market Data (Ticker)
```rust
pub struct UnifiedMarketData {
    pub exchange: Exchange,
    pub symbol: String,              // Normalized symbol (e.g., BTC-USD)
    pub timestamp: u32,              // Unix timestamp
    pub nanos: u32,                  // Nanosecond precision
    pub price: f32,                  // Last trade price
    pub volume: f32,                 // Last trade volume
    pub side: TradeSide,             // Buy/Sell
    pub best_bid: f32,               // Best bid price
    pub best_ask: f32,               // Best ask price
    pub exchange_specific: Option<HashMap<String, Value>>, // Extra fields
}
```

### 2. Trade Data
```rust
pub struct UnifiedTradeData {
    pub exchange: Exchange,
    pub symbol: String,              // Normalized symbol
    pub trade_id: u64,               // Exchange trade ID
    pub timestamp: u32,              // Unix timestamp
    pub nanos: u32,                  // Nanosecond precision
    pub price: f32,                  // Trade price
    pub size: f32,                   // Trade size
    pub side: TradeSide,             // Buy/Sell
    pub maker_order_id: [u8; 16],    // UUID bytes
    pub taker_order_id: [u8; 16],    // UUID bytes
    pub exchange_specific: Option<HashMap<String, Value>>,
}
```

### 3. Symbol Mapping System

#### Symbol Mapping Architecture
```rust
pub struct SymbolMapper {
    mappings: HashMap<String, ExchangeSymbolMap>,
    normalized_index: HashMap<String, String>, // exchange:symbol -> normalized
    asset_groups: HashMap<String, Vec<SymbolInfo>>, // BTC -> all BTC pairs
}

pub struct ExchangeSymbolMap {
    normalized: String,              // e.g., "BTC-USD"
    exchange_symbols: HashMap<ExchangeId, String>, // COINBASE -> "BTC-USD", BINANCE -> "BTCUSDT"
    asset_class: AssetClass,         // SPOT, FUTURES, etc.
    base_asset: String,              // "BTC"
    quote_asset: String,             // "USD"
    quote_type: QuoteType,           // FIAT, STABLE, CRYPTO
}

pub struct SymbolInfo {
    exchange: ExchangeId,
    symbol: String,
    normalized: String,
    active: bool,
    min_size: Option<f64>,
    tick_size: Option<f64>,
}

pub enum QuoteType {
    Fiat(String),        // USD, EUR, GBP
    Stablecoin(String),  // USDT, USDC, BUSD
    Crypto(String),      // BTC, ETH
}
```

#### Symbol Mapping Configuration
```yaml
# symbol_mappings.yaml
symbol_mappings:
  - normalized: "BTC-USD"
    base: "BTC"
    quote: "USD"
    quote_type: "fiat"
    exchanges:
      coinbase: "BTC-USD"
      binance: "BTCUSDT"    # Maps USDT to USD equivalent
      kraken: "XBTUSD"
      
  - normalized: "ETH-USD"
    base: "ETH"
    quote: "USD"
    quote_type: "fiat"
    exchanges:
      coinbase: "ETH-USD"
      binance: "ETHUSDT"
      binance_alt: "ETHBUSD"  # Alternative stablecoin pairing
      
  - normalized: "BTC-USDT"
    base: "BTC"
    quote: "USDT"
    quote_type: "stablecoin"
    exchanges:
      binance: "BTCUSDT"
      coinbase: "BTC-USDT"   # If available

# Asset equivalence rules
equivalence_rules:
  quote_assets:
    - group: "USD_EQUIVALENT"
      members: ["USD", "USDT", "USDC", "BUSD", "DAI"]
      primary: "USD"
```

#### Symbol Query API
```rust
impl SymbolMapper {
    // Get normalized symbol from exchange-specific symbol
    pub fn normalize(&self, exchange: ExchangeId, symbol: &str) -> Option<String>;
    
    // Get exchange-specific symbol from normalized
    pub fn to_exchange(&self, normalized: &str, exchange: ExchangeId) -> Option<String>;
    
    // Find all related symbols across exchanges
    pub fn find_related(&self, base: &str, quote: &str) -> Vec<SymbolInfo>;
    
    // Get all USD-equivalent pairs for an asset
    pub fn get_usd_pairs(&self, asset: &str) -> Vec<SymbolInfo>;
    
    // Check if symbols are equivalent (e.g., BTC-USD ≈ BTCUSDT)
    pub fn are_equivalent(&self, sym1: &str, sym2: &str) -> bool;
}
```

## Exchange Trait System

```rust
#[async_trait]
pub trait Exchange: Send + Sync {
    // Exchange identification
    fn name(&self) -> &'static str;
    fn id(&self) -> ExchangeId;
    
    // Symbol management
    async fn fetch_symbols(&self) -> Result<Vec<Symbol>>;
    fn normalize_symbol(&self, exchange_symbol: &str) -> String;
    
    // WebSocket management
    async fn create_connection(&self, symbols: Vec<String>) -> Result<Box<dyn ExchangeConnection>>;
    
    // Data parsing
    fn parse_market_data(&self, raw: &Value) -> Result<Option<UnifiedMarketData>>;
    fn parse_trade_data(&self, raw: &Value) -> Result<Option<UnifiedTradeData>>;
    
    // Configuration
    fn config(&self) -> &ExchangeConfig;
}

#[async_trait]
pub trait ExchangeConnection: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn subscribe(&mut self, channels: Vec<Channel>) -> Result<()>;
    async fn read_message(&mut self) -> Result<Option<Message>>;
    async fn reconnect(&mut self) -> Result<()>;
    fn is_connected(&self) -> bool;
}
```

## Implementation Details

### Phase 1: Core Refactoring (Week 1-2)

1. **Extract Common Components**
   - Move shared logic from coinbase-logger to common modules
   - Create unified data types with proper serialization
   - Implement generic file handlers with exchange prefixes
   - Build exchange trait system

2. **Refactor Coinbase Implementation**
   - Adapt existing code to new trait system
   - Implement symbol normalization
   - Update file paths to include exchange prefix
   - Maintain backward compatibility during transition

3. **Configuration System**
   ```toml
   [logger]
   data_path = "/mnt/md/data"
   buffer_size = 8192
   flush_interval_secs = 5
   
   [exchanges.coinbase]
   enabled = true
   ws_endpoint = "wss://ws-feed.exchange.coinbase.com"
   rest_endpoint = "https://api.exchange.coinbase.com"
   max_connections = 10
   symbols_per_connection = 50
   
   [exchanges.binance]
   enabled = true
   ws_endpoint = "wss://stream.binance.com:9443"
   rest_endpoint = "https://api.binance.com"
   max_connections = 5
   symbols_per_connection = 100
   ```

### Phase 2: Binance Integration (Week 3-4)

1. **Binance Connection Handler**
   - WebSocket connection management
   - Handle Binance-specific ping/pong (20s interval)
   - Stream subscription format: `<symbol>@trade`, `<symbol>@ticker`
   - Combined streams support

2. **Binance Data Parsing**
   - Map Binance fields to unified format:
     ```
     Binance ticker → UnifiedMarketData
     - "c": last price → price
     - "v": volume → volume
     - "b": best bid price → best_bid
     - "a": best ask price → best_ask
     - "E": event time → timestamp + nanos
     
     Binance trade → UnifiedTradeData
     - "t": trade ID → trade_id
     - "p": price → price
     - "q": quantity → size
     - "m": is buyer maker → side (inverted)
     - "T": trade time → timestamp + nanos
     ```

3. **Symbol Handling**
   - Binance: `BTCUSDT` → Normalized: `BTC-USDT`
   - Support USDT, BUSD, and other quote currencies
   - Handle spot vs futures symbols

### Phase 3: Testing & Migration (Week 5)

1. **Comprehensive Testing**
   - Unit tests for data parsing
   - Integration tests for WebSocket connections
   - Data consistency tests across exchanges
   - Performance benchmarks

2. **Migration Strategy**
   - Run new logger in parallel with existing
   - Validate data consistency
   - Gradual cutover with fallback option
   - Update downstream consumers (server, web app)

## Performance Considerations

1. **Connection Pooling**
   - Coinbase: 10 connections, 50 symbols each
   - Binance: 5 connections, 100 symbols each (higher rate limits)
   - Dynamic connection scaling based on message rates

2. **Buffer Management**
   - Per-exchange buffer pools
   - Adaptive buffer sizing based on throughput
   - Parallel flush operations

3. **CPU Optimization**
   - SIMD for data transformation where applicable
   - Zero-copy parsing when possible
   - Minimal allocations in hot paths

## Monitoring & Operations

1. **Health Checks**
   ```json
   GET /health
   {
     "status": "healthy",
     "exchanges": {
       "coinbase": {
         "connected": true,
         "symbols": 500,
         "messages_per_second": 1250,
         "last_message": "2025-01-25T10:30:45Z"
       },
       "binance": {
         "connected": true,
         "symbols": 600,
         "messages_per_second": 2100,
         "last_message": "2025-01-25T10:30:46Z"
       }
     }
   }
   ```

2. **Metrics Collection**
   - Messages processed per exchange
   - Write throughput per symbol
   - Connection stability metrics
   - Data quality indicators

## Future Extensibility

1. **Additional Exchanges**
   - Kraken: Similar WebSocket structure to Coinbase
   - Bybit: High-frequency derivatives data
   - OKX: Large Asian market coverage

2. **Enhanced Features**
   - Order book depth tracking
   - Options/futures data support
   - Cross-exchange arbitrage signals
   - Real-time data validation

## Risk Mitigation

1. **Data Integrity**
   - Checksums for critical data
   - Duplicate detection
   - Gap detection and alerting

2. **Operational Risks**
   - Graceful degradation per exchange
   - Circuit breakers for bad data
   - Automated rollback capability

3. **Compliance**
   - Exchange-specific rate limit adherence
   - Data retention policies
   - Access control per exchange

## Success Criteria

1. **Functional Requirements**
   - ✓ Support for Coinbase and Binance
   - ✓ Unified data format across exchanges
   - ✓ Backward compatible file structure
   - ✓ Zero data loss during migration

2. **Performance Requirements**
   - ✓ < 1ms processing latency per message
   - ✓ Support 10,000+ messages/second aggregate
   - ✓ < 5 second data persistence guarantee
   - ✓ 99.99% uptime per exchange

3. **Operational Requirements**
   - ✓ Easy addition of new exchanges
   - ✓ Comprehensive monitoring
   - ✓ Automated error recovery
   - ✓ Clear documentation

## Timeline

- **Week 1-2**: Core refactoring and trait system
- **Week 3-4**: Binance implementation
- **Week 5**: Testing and migration
- **Week 6**: Production deployment and monitoring

## Conclusion

This refactoring will transform the single-exchange logger into a robust, multi-exchange data collection system while maintaining the high performance and reliability required for financial data. The modular architecture ensures easy addition of new exchanges while the unified data format simplifies downstream processing and analysis.