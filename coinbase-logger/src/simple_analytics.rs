use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::data_types::MarketTradeData;

/// Simple analytics tracker for trade data
#[derive(Debug, Clone)]
pub struct SimpleAnalytics {
    pub symbol: String,
    pub start_time: Instant,
    pub trade_count: u64,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub high_price: f32,
    pub low_price: f32,
    pub open_price: f32,
    pub last_price: f32,
    pub vwap_sum: f64,
    pub large_trades: Vec<LargeTrade>,
    pub last_report: Instant,
}

#[derive(Debug, Clone)]
pub struct LargeTrade {
    pub trade_id: u64,
    pub timestamp: u32,
    pub price: f32,
    pub size: f32,
    pub side: u8,
}

impl SimpleAnalytics {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            start_time: Instant::now(),
            trade_count: 0,
            total_volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            high_price: 0.0,
            low_price: f32::MAX,
            open_price: 0.0,
            last_price: 0.0,
            vwap_sum: 0.0,
            large_trades: Vec::new(),
            last_report: Instant::now(),
        }
    }

    pub fn process_trade(&mut self, trade: &MarketTradeData, large_trade_threshold: f32) {
        // First trade sets the open
        if self.trade_count == 0 {
            self.open_price = trade.price;
        }

        // Update high/low
        self.high_price = self.high_price.max(trade.price);
        self.low_price = self.low_price.min(trade.price);

        // Update last price
        self.last_price = trade.price;

        // Update volumes
        let size_f64 = trade.size as f64;
        self.total_volume += size_f64;
        if trade.side == 1 {
            self.buy_volume += size_f64;
        } else {
            self.sell_volume += size_f64;
        }

        // Update VWAP sum
        self.vwap_sum += trade.price as f64 * size_f64;

        // Track large trades
        if trade.size >= large_trade_threshold {
            self.large_trades.push(LargeTrade {
                trade_id: trade.trade_id,
                timestamp: trade.timestamp_secs,
                price: trade.price,
                size: trade.size,
                side: trade.side,
            });

            // Keep only last 100 large trades
            if self.large_trades.len() > 100 {
                self.large_trades.remove(0);
            }
        }

        self.trade_count += 1;
    }

    pub fn should_report(&self, interval: Duration) -> bool {
        self.last_report.elapsed() >= interval
    }

    pub fn generate_report(&mut self) -> AnalyticsReport {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let trades_per_second = if elapsed > 0.0 {
            self.trade_count as f64 / elapsed
        } else {
            0.0
        };

        let vwap = if self.total_volume > 0.0 {
            self.vwap_sum / self.total_volume
        } else {
            0.0
        };

        let buy_sell_ratio = if self.sell_volume > 0.0 {
            self.buy_volume / self.sell_volume
        } else if self.buy_volume > 0.0 {
            f64::INFINITY
        } else {
            1.0
        };

        let report = AnalyticsReport {
            symbol: self.symbol.clone(),
            period_seconds: self.last_report.elapsed().as_secs(),
            trade_count: self.trade_count,
            trades_per_second,
            total_volume: self.total_volume,
            buy_volume: self.buy_volume,
            sell_volume: self.sell_volume,
            buy_sell_ratio,
            high_price: self.high_price,
            low_price: self.low_price,
            open_price: self.open_price,
            close_price: self.last_price,
            vwap: vwap as f32,
            large_trade_count: self.large_trades.len() as u32,
            largest_trade: self.large_trades
                .iter()
                .max_by(|a, b| a.size.partial_cmp(&b.size).unwrap())
                .cloned(),
        };

        // Reset for next period
        self.last_report = Instant::now();

        report
    }
}

#[derive(Debug, Clone)]
pub struct AnalyticsReport {
    pub symbol: String,
    pub period_seconds: u64,
    pub trade_count: u64,
    pub trades_per_second: f64,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub buy_sell_ratio: f64,
    pub high_price: f32,
    pub low_price: f32,
    pub open_price: f32,
    pub close_price: f32,
    pub vwap: f32,
    pub large_trade_count: u32,
    pub largest_trade: Option<LargeTrade>,
}

impl AnalyticsReport {
    pub fn to_log_string(&self) -> String {
        format!(
            "{}: {} trades ({:.1}/s), Vol: {:.2} (B:{:.2}/S:{:.2}, ratio:{:.2}), \
             Price: {:.2}-{:.2} (O:{:.2}/C:{:.2}), VWAP: {:.2}, \
             Large trades: {} {}",
            self.symbol,
            self.trade_count,
            self.trades_per_second,
            self.total_volume,
            self.buy_volume,
            self.sell_volume,
            self.buy_sell_ratio,
            self.low_price,
            self.high_price,
            self.open_price,
            self.close_price,
            self.vwap,
            self.large_trade_count,
            if let Some(ref trade) = self.largest_trade {
                format!("(largest: {:.2} @ {:.2})", trade.size, trade.price)
            } else {
                String::new()
            }
        )
    }
}

/// Manages analytics for all symbols
pub struct AnalyticsManager {
    pub analytics: HashMap<String, SimpleAnalytics>,
    pub report_interval: Duration,
    pub large_trade_threshold: f32,
}

impl AnalyticsManager {
    pub fn new(large_trade_threshold: f32) -> Self {
        Self {
            analytics: HashMap::new(),
            report_interval: Duration::from_secs(30), // Report every 30 seconds for testing
            large_trade_threshold,
        }
    }

    pub fn process_trade(&mut self, symbol: &str, trade: &MarketTradeData) {
        let analytics = self.analytics
            .entry(symbol.to_string())
            .or_insert_with(|| SimpleAnalytics::new(symbol.to_string()));
        
        analytics.process_trade(trade, self.large_trade_threshold);
    }

    pub fn generate_reports(&mut self) -> Vec<AnalyticsReport> {
        let mut reports = Vec::new();
        
        for analytics in self.analytics.values_mut() {
            if analytics.should_report(self.report_interval) {
                reports.push(analytics.generate_report());
            }
        }
        
        reports
    }
}