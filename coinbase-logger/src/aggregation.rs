use std::collections::VecDeque;
use std::time::Instant;

use crate::data_types::MarketTradeData;
use crate::Result;

/// Aggregated candle data
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

impl CandleData {
    pub fn new(timestamp: u32) -> Self {
        Self {
            timestamp,
            open: 0.0,
            high: 0.0,
            low: f32::MAX,
            close: 0.0,
            volume: 0.0,
            trade_count: 0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            vwap: 0.0,
        }
    }

    pub fn update_with_trade(&mut self, trade: &MarketTradeData) {
        // First trade sets the open
        if self.trade_count == 0 {
            self.open = trade.price;
        }

        // Update high/low
        self.high = self.high.max(trade.price);
        self.low = self.low.min(trade.price);

        // Latest trade is the close
        self.close = trade.price;

        // Update volumes
        self.volume += trade.size;
        if trade.side == 1 {
            self.buy_volume += trade.size;
        } else {
            self.sell_volume += trade.size;
        }

        // Update VWAP
        // Note: self.volume already includes trade.size from line 52
        let previous_volume = self.volume - trade.size;
        self.vwap = (self.vwap * previous_volume + trade.price * trade.size) / self.volume;

        self.trade_count += 1;
    }

    pub fn is_complete(&self, current_time: u32, period_seconds: u32) -> bool {
        current_time >= self.timestamp + period_seconds
    }
}

/// Trade metrics for analysis
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

/// Significant trade detection
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

/// Configuration for trade aggregation
#[derive(Clone, Debug)]
pub struct AggregationConfig {
    pub candle_periods: Vec<u32>,   // [60, 300, 900] seconds
    pub large_trade_threshold: f32, // e.g., 1.0 BTC
    pub significance_window: u32,   // seconds for comparison
    pub metrics_interval: u32,      // seconds between calculations
    pub momentum_window: usize,     // number of candles for momentum
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            candle_periods: vec![60, 300, 900], // 1m, 5m, 15m
            large_trade_threshold: 1.0,
            significance_window: 300,
            metrics_interval: 60,
            momentum_window: 20,
        }
    }
}

/// Real-time trade aggregator for a single symbol
pub struct TradeAggregator {
    pub symbol: String,
    pub config: AggregationConfig,
    pub current_candles: std::collections::HashMap<u32, CandleData>, // period -> candle
    pub trades_buffer: VecDeque<MarketTradeData>,
    pub recent_candles: std::collections::HashMap<u32, VecDeque<CandleData>>, // period -> history
    pub significant_trades: Vec<SignificantTrade>,
    pub last_metrics_update: Instant,
}

impl TradeAggregator {
    pub fn new(symbol: String, config: AggregationConfig) -> Self {
        let mut current_candles = std::collections::HashMap::new();
        let mut recent_candles = std::collections::HashMap::new();

        // Initialize candles for each period
        for &period in &config.candle_periods {
            current_candles.insert(period, CandleData::new(0));
            recent_candles.insert(period, VecDeque::with_capacity(config.momentum_window));
        }

        Self {
            symbol,
            config,
            current_candles,
            trades_buffer: VecDeque::with_capacity(10000),
            recent_candles,
            significant_trades: Vec::new(),
            last_metrics_update: Instant::now(),
        }
    }

    pub fn process_trade(&mut self, trade: &MarketTradeData) -> Result<()> {
        // Add to buffer for metrics calculation
        self.trades_buffer.push_back(trade.clone());

        // Update candles for each period
        for &period in &self.config.candle_periods {
            let candle = self.current_candles.get_mut(&period).unwrap();

            // Check if we need to start a new candle
            if candle.timestamp == 0 {
                candle.timestamp = (trade.timestamp_secs / period) * period;
            } else if trade.timestamp_secs >= candle.timestamp + period {
                // Complete current candle and start new one
                let completed = candle.clone();

                // Store in history
                let history = self.recent_candles.get_mut(&period).unwrap();
                history.push_back(completed);
                if history.len() > self.config.momentum_window {
                    history.pop_front();
                }

                // Start new candle
                *candle = CandleData::new((trade.timestamp_secs / period) * period);
            }

            candle.update_with_trade(trade);
        }

        // Check for significant trades
        if trade.size >= self.config.large_trade_threshold {
            self.detect_significant_trade(trade);
        }

        // Clean old trades from buffer
        self.clean_old_trades(trade.timestamp_secs);

        Ok(())
    }

    fn detect_significant_trade(&mut self, trade: &MarketTradeData) {
        let mut score = 0.0;

        // Size score
        score += (trade.size / self.config.large_trade_threshold).min(10.0);

        // Calculate average trade size in window
        let avg_size = if !self.trades_buffer.is_empty() {
            let total_size: f32 = self.trades_buffer.iter().map(|t| t.size).sum();
            total_size / self.trades_buffer.len() as f32
        } else {
            trade.size
        };

        // Relative size score
        if avg_size > 0.0 {
            score += (trade.size / avg_size).min(5.0);
        }

        // Price impact estimation (simplified)
        let price_impact = if let Some(candle) = self.current_candles.get(&60) {
            if candle.trade_count > 0 {
                ((trade.price - candle.vwap) / candle.vwap).abs() * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        score += price_impact.min(5.0);

        if score > 5.0 {
            self.significant_trades.push(SignificantTrade {
                trade_id: trade.trade_id,
                timestamp: trade.timestamp_secs,
                price: trade.price,
                size: trade.size,
                side: trade.side,
                significance_score: score,
                price_impact,
            });

            // Keep only recent significant trades
            if self.significant_trades.len() > 100 {
                self.significant_trades.remove(0);
            }
        }
    }

    fn clean_old_trades(&mut self, current_time: u32) {
        let cutoff_time = current_time.saturating_sub(self.config.significance_window);

        while let Some(front) = self.trades_buffer.front() {
            if front.timestamp_secs < cutoff_time {
                self.trades_buffer.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn calculate_metrics(&self, current_time: u32) -> TradeMetrics {
        let window_start = current_time.saturating_sub(self.config.metrics_interval);
        let window_trades: Vec<&MarketTradeData> = self
            .trades_buffer
            .iter()
            .filter(|t| t.timestamp_secs >= window_start)
            .collect();

        let trade_count = window_trades.len() as f32;
        let window_duration = self.config.metrics_interval as f32;

        let trades_per_second = trade_count / window_duration;

        let total_volume: f32 = window_trades.iter().map(|t| t.size).sum();
        let volume_per_second = total_volume / window_duration;

        let buy_volume: f32 = window_trades
            .iter()
            .filter(|t| t.side == 1)
            .map(|t| t.size)
            .sum();
        let sell_volume: f32 = total_volume - buy_volume;
        let buy_sell_ratio = if sell_volume > 0.0 {
            buy_volume / sell_volume
        } else {
            buy_volume
        };

        let large_trade_count = window_trades
            .iter()
            .filter(|t| t.size >= self.config.large_trade_threshold)
            .count() as u32;

        // Calculate momentum from 1-minute candles
        let (price_momentum, volume_momentum) = if let Some(history) = self.recent_candles.get(&60)
        {
            self.calculate_momentum(history)
        } else {
            (0.0, 0.0)
        };

        TradeMetrics {
            timestamp: current_time,
            trades_per_second,
            volume_per_second,
            buy_sell_ratio,
            large_trade_count,
            price_momentum,
            volume_momentum,
        }
    }

    fn calculate_momentum(&self, candles: &VecDeque<CandleData>) -> (f32, f32) {
        if candles.len() < 2 {
            return (0.0, 0.0);
        }

        // Simple rate of change for price
        let oldest = candles.front().unwrap();
        let newest = candles.back().unwrap();

        let price_momentum = if oldest.close > 0.0 {
            ((newest.close - oldest.close) / oldest.close) * 100.0
        } else {
            0.0
        };

        // Volume momentum (relative to average)
        let avg_volume: f32 = candles.iter().map(|c| c.volume).sum::<f32>() / candles.len() as f32;
        let volume_momentum = if avg_volume > 0.0 {
            ((newest.volume - avg_volume) / avg_volume) * 100.0
        } else {
            0.0
        };

        (price_momentum, volume_momentum)
    }
}
