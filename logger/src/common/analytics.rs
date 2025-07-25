use crate::common::data_types::{TradeSide, UnifiedTradeData};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeAnalytics {
    pub symbol: String,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub trade_count: u64,
    pub buy_count: u64,
    pub sell_count: u64,
    pub vwap: f64,
    pub high_price: f32,
    pub low_price: f32,
    pub last_price: f32,
    pub large_trades: Vec<LargeTrade>,
    #[serde(skip)]
    pub period_start: Option<Instant>,
    pub period_start_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeTrade {
    pub timestamp: u32,
    pub price: f32,
    pub size: f32,
    pub side: TradeSide,
    pub value: f64,
}

#[derive(Debug)]
pub struct AnalyticsEngine {
    analytics: Arc<DashMap<String, TradeAnalytics>>,
    large_trade_threshold: f64, // USD value
    report_interval: Duration,
    last_report_timestamp: Arc<AtomicU64>,
}

impl AnalyticsEngine {
    pub fn new(large_trade_threshold: f64, report_interval: Duration) -> Self {
        let now = Instant::now().elapsed().as_secs();
        Self {
            analytics: Arc::new(DashMap::new()),
            large_trade_threshold,
            report_interval,
            last_report_timestamp: Arc::new(AtomicU64::new(now)),
        }
    }

    pub fn process_trade(&self, trade: &UnifiedTradeData) {
        let mut analytics = self
            .analytics
            .entry(trade.symbol.clone())
            .or_insert_with(|| {
                let now = Instant::now();
                TradeAnalytics {
                    symbol: trade.symbol.clone(),
                    total_volume: 0.0,
                    buy_volume: 0.0,
                    sell_volume: 0.0,
                    trade_count: 0,
                    buy_count: 0,
                    sell_count: 0,
                    vwap: 0.0,
                    high_price: trade.price,
                    low_price: trade.price,
                    last_price: trade.price,
                    large_trades: Vec::new(),
                    period_start: Some(now),
                    period_start_secs: now.elapsed().as_secs(),
                }
            });

        // Update volume
        let volume = trade.size as f64;
        let value = volume * trade.price as f64;

        // Update VWAP before updating total_volume
        let old_total_value = analytics.vwap * analytics.total_volume;
        let new_total_volume = analytics.total_volume + volume;
        analytics.vwap = if new_total_volume > 0.0 {
            (old_total_value + value) / new_total_volume
        } else {
            trade.price as f64
        };

        analytics.total_volume = new_total_volume;
        analytics.trade_count += 1;

        match trade.side {
            TradeSide::Buy => {
                analytics.buy_volume += volume;
                analytics.buy_count += 1;
            }
            TradeSide::Sell => {
                analytics.sell_volume += volume;
                analytics.sell_count += 1;
            }
        }

        // Update high/low
        analytics.high_price = analytics.high_price.max(trade.price);
        analytics.low_price = analytics.low_price.min(trade.price);
        analytics.last_price = trade.price;

        // Track large trades
        if value >= self.large_trade_threshold {
            analytics.large_trades.push(LargeTrade {
                timestamp: trade.timestamp,
                price: trade.price,
                size: trade.size,
                side: trade.side,
                value,
            });

            // Keep only last 100 large trades
            if analytics.large_trades.len() > 100 {
                analytics.large_trades.remove(0);
            }
        }
    }

    pub fn should_report(&self) -> bool {
        let last = self.last_report_timestamp.load(Ordering::Relaxed);
        let now = Instant::now().elapsed().as_secs();
        now.saturating_sub(last) >= self.report_interval.as_secs()
    }

    pub fn generate_report(&self) -> Vec<TradeAnalytics> {
        let mut report = Vec::new();
        for entry in self.analytics.iter() {
            report.push(entry.value().clone());
        }

        // Sort by volume
        report.sort_by(|a, b| b.total_volume.partial_cmp(&a.total_volume).unwrap());

        report
    }

    pub fn print_report(&self) {
        if !self.should_report() {
            return;
        }

        // Update last report timestamp
        let now = Instant::now().elapsed().as_secs();
        self.last_report_timestamp.store(now, Ordering::Relaxed);

        let mut report = Vec::new();
        for entry in self.analytics.iter() {
            report.push(entry.value().clone());
        }

        // Sort by volume
        report.sort_by(|a, b| b.total_volume.partial_cmp(&a.total_volume).unwrap());

        info!("=== Trade Analytics Report ===");
        for analytics in report.iter().take(10) {
            let buy_ratio = if analytics.trade_count > 0 {
                (analytics.buy_count as f64 / analytics.trade_count as f64) * 100.0
            } else {
                0.0
            };

            info!(
                "{}: Vol={:.2} VWAP=${:.2} High=${:.2} Low=${:.2} Last=${:.2} Trades={} Buy%={:.1}% LargeTrades={}",
                analytics.symbol,
                analytics.total_volume,
                analytics.vwap,
                analytics.high_price,
                analytics.low_price,
                analytics.last_price,
                analytics.trade_count,
                buy_ratio,
                analytics.large_trades.len()
            );
        }
    }

    pub fn get_analytics(&self, symbol: &str) -> Option<TradeAnalytics> {
        self.analytics.get(symbol).map(|a| a.clone())
    }

    pub fn reset_period(&self) {
        self.analytics.clear();
    }
}

#[derive(Debug)]
pub struct MarketMetrics {
    pub messages_per_second: Arc<DashMap<String, u64>>,
    pub last_message_time: Arc<DashMap<String, Instant>>,
    pub connection_health: Arc<DashMap<String, ConnectionHealth>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHealth {
    pub connected: bool,
    #[serde(skip)]
    pub last_reconnect: Option<Instant>,
    pub last_reconnect_secs: Option<u64>,
    pub reconnect_count: u32,
    pub error_count: u32,
    pub last_error: Option<String>,
}

impl Default for MarketMetrics {
    fn default() -> Self {
        Self {
            messages_per_second: Arc::new(DashMap::new()),
            last_message_time: Arc::new(DashMap::new()),
            connection_health: Arc::new(DashMap::new()),
        }
    }
}

impl MarketMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_message(&self, exchange: &str) {
        let mut count = self
            .messages_per_second
            .entry(exchange.to_string())
            .or_insert(0);
        *count += 1;

        self.last_message_time
            .insert(exchange.to_string(), Instant::now());
    }

    pub fn record_connection_status(&self, exchange: &str, connected: bool) {
        self.connection_health
            .entry(exchange.to_string())
            .and_modify(|h| h.connected = connected)
            .or_insert(ConnectionHealth {
                connected,
                last_reconnect: None,
                last_reconnect_secs: None,
                reconnect_count: 0,
                error_count: 0,
                last_error: None,
            });
    }

    pub fn record_reconnect(&self, exchange: &str) {
        self.connection_health
            .entry(exchange.to_string())
            .and_modify(|h| {
                let now = Instant::now();
                h.last_reconnect = Some(now);
                h.last_reconnect_secs = Some(0); // Time since startup
                h.reconnect_count += 1;
            });
    }

    pub fn record_error(&self, exchange: &str, error: String) {
        self.connection_health
            .entry(exchange.to_string())
            .and_modify(|h| {
                h.error_count += 1;
                h.last_error = Some(error.clone());
            });
    }

    pub fn get_health_status(&self) -> HashMap<String, ConnectionHealth> {
        use std::collections::HashMap;

        let mut status = HashMap::new();
        for entry in self.connection_health.iter() {
            status.insert(entry.key().clone(), entry.value().clone());
        }
        status
    }

    pub fn reset_message_counts(&self) {
        self.messages_per_second.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::data_types::ExchangeId;

    #[test]
    fn test_trade_analytics() {
        let engine = AnalyticsEngine::new(10000.0, Duration::from_secs(30));

        let mut trade = UnifiedTradeData::new(ExchangeId::Coinbase, "BTC-USD".to_string(), 1);
        trade.price = 50000.0;
        trade.size = 0.5;
        trade.side = TradeSide::Buy;

        engine.process_trade(&trade);

        let analytics = engine.get_analytics("BTC-USD").unwrap();
        assert_eq!(analytics.total_volume, 0.5);
        assert_eq!(analytics.buy_volume, 0.5);
        assert_eq!(analytics.trade_count, 1);
        assert_eq!(analytics.vwap, 50000.0);
        assert_eq!(analytics.high_price, 50000.0);
        assert_eq!(analytics.low_price, 50000.0);
        assert!(analytics.period_start.is_some());
    }

    #[test]
    fn test_large_trade_detection() {
        let engine = AnalyticsEngine::new(10000.0, Duration::from_secs(30));

        let mut trade = UnifiedTradeData::new(ExchangeId::Binance, "ETH-USDT".to_string(), 1);
        trade.price = 3000.0;
        trade.size = 5.0; // $15,000 trade
        trade.side = TradeSide::Sell;

        engine.process_trade(&trade);

        let analytics = engine.get_analytics("ETH-USDT").unwrap();
        assert_eq!(analytics.large_trades.len(), 1);
        assert_eq!(analytics.large_trades[0].value, 15000.0);
    }

    #[test]
    fn test_market_metrics() {
        let metrics = MarketMetrics::new();

        metrics.record_message("coinbase");
        metrics.record_message("coinbase");
        metrics.record_message("binance");

        assert_eq!(*metrics.messages_per_second.get("coinbase").unwrap(), 2);
        assert_eq!(*metrics.messages_per_second.get("binance").unwrap(), 1);

        metrics.record_connection_status("coinbase", true);
        metrics.record_connection_status("binance", false); // Create entry first
        metrics.record_error("binance", "Connection timeout".to_string());

        let health = metrics.get_health_status();
        assert!(health.get("coinbase").unwrap().connected);
        assert_eq!(health.get("binance").unwrap().error_count, 1);
    }
}
