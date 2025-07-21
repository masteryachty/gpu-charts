# Phase 3: Advanced Trade Analytics and Aggregation

## Overview

This phase builds upon the raw trade data collected in Phases 1 and 2 to provide advanced analytics, aggregations, and real-time metrics that enhance chart visualization and trading insights.

## Goals

1. Implement real-time trade aggregation (candles, VWAP)
2. Calculate trade intensity and momentum metrics
3. Detect and flag significant trades
4. Create efficient query interfaces for charts
5. Enable advanced visualization features

## Technical Design

### New Data Structures

```rust
// Aggregated candle data
#[derive(Clone, Debug)]
pub struct CandleData {
    pub timestamp: u32,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    pub volume: f32,
    pub trade_count: u32,
    pub buy_volume: f32,
    pub sell_volume: f32,
    pub vwap: f32,
}

// Trade metrics
#[derive(Clone, Debug)]
pub struct TradeMetrics {
    pub timestamp: u32,
    pub trades_per_second: f32,
    pub volume_per_second: f32,
    pub buy_sell_ratio: f32,
    pub large_trade_count: u32,
    pub price_momentum: f32,
    pub volume_momentum: f32,
}

// Significant trade detection
#[derive(Clone, Debug)]
pub struct SignificantTrade {
    pub trade_id: u64,
    pub timestamp: u32,
    pub price: f32,
    pub size: f32,
    pub side: u8,
    pub significance_score: f32,
    pub price_impact: f32,
}

// Real-time aggregator
pub struct TradeAggregator {
    pub symbol: String,
    pub current_candle: CandleData,
    pub candle_period: u32,  // seconds
    pub trades_buffer: VecDeque<MarketTradeData>,
    pub metrics_calculator: MetricsCalculator,
    pub significant_trades: Vec<SignificantTrade>,
}
```

### File Structure

```
/mnt/md/data/{symbol}/
├── MD/                    # Ticker data
├── TICKER_TRADES/         # Phase 1 data
├── TRADES/                # Phase 2 individual trades
└── ANALYTICS/             # New analytics data
    ├── candles/
    │   ├── 1m/            # 1-minute candles
    │   │   ├── ohlc.{date}.bin
    │   │   ├── volume.{date}.bin
    │   │   └── vwap.{date}.bin
    │   ├── 5m/            # 5-minute candles
    │   └── 15m/           # 15-minute candles
    ├── metrics/
    │   ├── trade_intensity.{date}.bin
    │   ├── momentum.{date}.bin
    │   └── buy_sell_ratio.{date}.bin
    └── significant/
        ├── trade_ids.{date}.bin
        ├── scores.{date}.bin
        └── impacts.{date}.bin
```

### Aggregation Pipeline

```rust
pub struct AggregationPipeline {
    pub aggregators: HashMap<String, TradeAggregator>,
    pub candle_writers: HashMap<String, CandleWriters>,
    pub metrics_writers: HashMap<String, MetricsWriters>,
    pub config: AggregationConfig,
}

pub struct AggregationConfig {
    pub candle_periods: Vec<u32>,  // [60, 300, 900] seconds
    pub large_trade_threshold: f32,  // e.g., 10.0 BTC
    pub significance_window: u32,    // seconds for comparison
    pub metrics_interval: u32,       // seconds between calculations
}
```

## Implementation Tasks

### Task 1: Implement Trade Aggregator
1. Create `TradeAggregator` struct
2. Implement candle building logic
3. Handle candle period transitions
4. Calculate OHLC values in real-time
5. Track buy/sell volumes separately

### Task 2: Build Metrics Calculator
1. Implement sliding window for trades
2. Calculate trades per second
3. Compute volume-weighted metrics
4. Track buy/sell pressure
5. Detect momentum changes

### Task 3: Significant Trade Detection
1. Define significance criteria
2. Implement rolling statistics
3. Calculate price impact estimation
4. Score trades by multiple factors
5. Maintain top N significant trades

### Task 4: Create Analytics Writers
1. Implement binary writers for analytics
2. Handle multiple timeframes
3. Ensure atomic candle writes
4. Compress metrics data
5. Index significant trades

### Task 5: Real-time Processing
1. Integrate with existing trade processing
2. Update aggregators on each trade
3. Trigger periodic calculations
4. Handle memory constraints
5. Implement graceful degradation

### Task 6: Query Interface
1. Create metadata files for fast lookup
2. Implement time-range queries
3. Add symbol-specific indexes
4. Enable streaming updates
5. Support multiple consumers

## Advanced Features Implementation

### 1. VWAP Calculation:
```rust
impl TradeAggregator {
    fn update_vwap(&mut self, trade: &MarketTradeData) {
        self.current_candle.vwap = 
            (self.current_candle.vwap * self.current_candle.volume + 
             trade.price * trade.size) / 
            (self.current_candle.volume + trade.size);
    }
}
```

### 2. Trade Intensity Detection:
```rust
impl MetricsCalculator {
    fn calculate_intensity(&self, trades: &VecDeque<MarketTradeData>) -> f32 {
        let time_span = trades.back().unwrap().timestamp_secs - 
                       trades.front().unwrap().timestamp_secs;
        if time_span > 0 {
            trades.len() as f32 / time_span as f32
        } else {
            trades.len() as f32
        }
    }
}
```

### 3. Significance Scoring:
```rust
impl SignificanceDetector {
    fn score_trade(&self, trade: &MarketTradeData, context: &TradingContext) -> f32 {
        let mut score = 0.0;
        
        // Size relative to average
        score += (trade.size / context.avg_trade_size).min(10.0);
        
        // Price deviation
        let price_dev = (trade.price - context.avg_price).abs() / context.avg_price;
        score += price_dev * 100.0;
        
        // Volume spike
        if trade.size > context.large_trade_threshold {
            score += 5.0;
        }
        
        // Time clustering
        if context.recent_large_trades > 3 {
            score += 2.0;
        }
        
        score
    }
}
```

### 4. Momentum Calculation:
```rust
impl MomentumCalculator {
    fn calculate_momentum(&self, trades: &VecDeque<MarketTradeData>) -> (f32, f32) {
        // Price momentum (rate of change)
        let price_momentum = self.calculate_price_roc(trades);
        
        // Volume momentum (relative to average)
        let volume_momentum = self.calculate_volume_ratio(trades);
        
        (price_momentum, volume_momentum)
    }
}
```

## Performance Optimization

### Memory Management
1. Sliding windows with fixed capacity
2. Periodic cleanup of old data
3. Memory-mapped aggregations
4. Compressed storage formats
5. Lazy loading of historical data

### Computation Optimization
1. Incremental calculations
2. Pre-computed lookup tables
3. SIMD operations for aggregations
4. Parallel processing per symbol
5. GPU acceleration for complex metrics

### I/O Optimization
1. Batch writes for aggregated data
2. Async I/O for all operations
3. Memory-mapped files for reads
4. Columnar storage format
5. Delta encoding for time series

## Advanced Visualization Features

### 1. Trade Flow Visualization
- Bubble charts with trade sizes
- Color coding by buy/sell
- Animated trade flow
- Cluster detection

### 2. Market Depth Evolution
- Order book reconstruction
- Liquidity heat maps
- Support/resistance levels
- Imbalance indicators

### 3. Microstructure Analysis
- Bid-ask spread evolution
- Trade timing patterns
- Order flow toxicity
- Market maker detection

### 4. Real-time Alerts
- Large trade notifications
- Momentum shifts
- Unusual activity detection
- Price level breaks

## Integration with Charting

### Data API Endpoints
```rust
// Server endpoints for analytics
GET /api/candles?symbol={symbol}&period={period}&start={start}&end={end}
GET /api/metrics?symbol={symbol}&type={type}&start={start}&end={end}
GET /api/significant-trades?symbol={symbol}&min_score={score}&limit={limit}
GET /api/trade-flow?symbol={symbol}&window={seconds}
```

### WebSocket Streaming
```rust
// Real-time analytics stream
{
    "type": "analytics_update",
    "symbol": "BTC-USD",
    "candle": { /* current candle */ },
    "metrics": { /* latest metrics */ },
    "significant_trades": [ /* recent significant */ ]
}
```

## Testing Strategy

### Unit Tests
1. Candle aggregation accuracy
2. VWAP calculations
3. Significance scoring
4. Momentum calculations
5. Edge cases handling

### Performance Tests
1. Process 10K trades/second
2. Memory usage under 500MB/symbol
3. Aggregation latency < 10ms
4. File write batching efficiency
5. Query response times

### Accuracy Tests
1. Compare with exchange candles
2. Validate VWAP calculations
3. Verify trade counts
4. Check volume aggregations
5. Test boundary conditions

## Monitoring and Alerts

### Health Metrics
1. Aggregation lag (ms)
2. Memory usage per symbol
3. Calculation errors
4. Write queue depth
5. CPU usage by component

### Quality Metrics
1. Candle completeness
2. Trade coverage percentage
3. Significance detection rate
4. Metric calculation frequency
5. Data consistency checks

## Success Criteria

1. Real-time candle generation < 10ms lag
2. Support for 200+ symbols concurrently
3. 100% trade coverage in aggregations
4. Significant trade detection accuracy > 95%
5. Query response times < 50ms

## Future Enhancements

### Machine Learning Integration
1. Trade pattern recognition
2. Anomaly detection models
3. Price prediction features
4. Market regime detection
5. Sentiment analysis

### Advanced Analytics
1. Market microstructure metrics
2. Cross-symbol correlations
3. Liquidity analysis
4. Order book reconstruction
5. Market impact modeling

### Scalability
1. Distributed aggregation
2. Cloud storage integration
3. Multi-region deployment
4. Horizontal scaling
5. Real-time replication

## Conclusion

Phase 3 transforms raw trade data into actionable insights and advanced visualizations. By implementing sophisticated aggregation and analysis capabilities, the coinbase logger becomes a comprehensive market data platform supporting professional-grade trading and analysis tools.