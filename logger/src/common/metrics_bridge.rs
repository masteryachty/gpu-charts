/// Bridge between the existing MarketMetrics system and Prometheus metrics
use crate::common::analytics::MarketMetrics;
use crate::common::data_types::{TradeSide, UnifiedMarketData, UnifiedTradeData};
use crate::metrics_server::*;
use std::sync::Arc;
use tracing::debug;

/// Wrapper around MarketMetrics that also records to Prometheus
pub struct MetricsBridge {
    market_metrics: Arc<MarketMetrics>,
}

impl MetricsBridge {
    pub fn new(market_metrics: Arc<MarketMetrics>) -> Self {
        Self { market_metrics }
    }
    
    /// Record a message and update both systems
    pub fn record_message(&self, exchange: &str) {
        // Update existing MarketMetrics
        self.market_metrics.record_message(exchange);
        
        // Update Prometheus metrics
        record_message(exchange, "data");
        debug!("Recorded message for exchange: {}", exchange);
    }
    
    /// Record connection status
    pub fn record_connection_status(&self, exchange: &str, connected: bool) {
        // Update existing MarketMetrics
        self.market_metrics.record_connection_status(exchange, connected);
        
        // Update Prometheus metrics
        set_connection_status(exchange, connected);
        
        if connected {
            debug!("Exchange {} connected", exchange);
        } else {
            debug!("Exchange {} disconnected", exchange);
        }
    }
    
    /// Record a reconnection
    pub fn record_reconnect(&self, exchange: &str) {
        // Update existing MarketMetrics
        self.market_metrics.record_reconnect(exchange);
        
        // Update Prometheus metrics
        record_reconnection(exchange);
        debug!("Exchange {} reconnected", exchange);
    }
    
    /// Record an error
    pub fn record_error(&self, exchange: &str, error: String) {
        // Update existing MarketMetrics
        self.market_metrics.record_error(exchange, error.clone());
        
        // Categorize the error for Prometheus
        let error_type = if error.contains("Connection") || error.contains("connect") {
            "connection"
        } else if error.contains("parse") || error.contains("Parse") {
            "parse"
        } else if error.contains("timeout") || error.contains("Timeout") {
            "timeout"
        } else if error.contains("subscribe") || error.contains("Subscribe") {
            "subscription"
        } else {
            "other"
        };
        
        // Update Prometheus metrics
        record_error(exchange, error_type);
        debug!("Recorded {} error for exchange {}: {}", error_type, exchange, error);
    }
    
    /// Get the underlying MarketMetrics for compatibility
    pub fn inner(&self) -> &Arc<MarketMetrics> {
        &self.market_metrics
    }
}

/// Process market data and record metrics
pub fn process_market_data(exchange: &str, data: &UnifiedMarketData) {
    // Record the message
    record_message(exchange, "market_data");
    
    // If we have price data, update price metrics
    if data.price > 0.0 {
        set_price_high(exchange, &data.symbol, data.price as f64);
        set_price_low(exchange, &data.symbol, data.price as f64);
    }
    
    // Record data freshness
    // This would be called by the DataFreshnessTracker in practice
    debug!("Processed market data for {} {}", exchange, data.symbol);
}

/// Process trade data and record metrics
pub fn process_trade_data(exchange: &str, data: &UnifiedTradeData) {
    // Record the trade
    record_trade(exchange, &data.symbol);
    record_message(exchange, "trade");
    
    // Calculate trade value
    let trade_value = data.price as f64 * data.size as f64;
    
    // Record trade volume
    let side = match data.side {
        TradeSide::Buy => "buy",
        TradeSide::Sell => "sell",
    };
    record_trade_volume(exchange, &data.symbol, side, trade_value);
    
    // Check for large trades (>$10,000)
    if trade_value > 10000.0 {
        record_large_trade(exchange, &data.symbol);
        debug!("Large trade detected on {} {}: ${:.2}", exchange, data.symbol, trade_value);
    }
    
    debug!("Processed trade for {} {}: {} {} @ {}", 
        exchange, data.symbol, side, data.size, data.price);
}

/// Record buffer metrics
pub fn record_buffer_metrics(exchange: &str, buffer_type: &str, size: usize) {
    set_buffer_size(exchange, buffer_type, size);
    debug!("Buffer {} for {} has {} bytes", buffer_type, exchange, size);
}

/// Record data write metrics
pub fn record_data_write(exchange: &str, data_type: &str, bytes: usize) {
    record_data_written(exchange, data_type, bytes);
    debug!("Wrote {} bytes of {} data for {}", bytes, data_type, exchange);
}

/// Set the number of symbols being monitored
pub fn set_monitored_symbols(exchange: &str, count: usize) {
    set_symbols_monitored(exchange, count);
    debug!("Monitoring {} symbols on {}", count, exchange);
}

/// Set the number of active WebSocket connections
pub fn set_active_connections(exchange: &str, count: usize) {
    set_websocket_connections(exchange, count);
    debug!("{} active WebSocket connections for {}", count, exchange);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_metrics_bridge() {
        let market_metrics = Arc::new(MarketMetrics::new());
        let bridge = MetricsBridge::new(market_metrics);
        
        // Test recording messages
        bridge.record_message("test_exchange");
        bridge.record_connection_status("test_exchange", true);
        bridge.record_reconnect("test_exchange");
        bridge.record_error("test_exchange", "Test error".to_string());
        
        // Verify the underlying MarketMetrics was updated
        let health = bridge.inner().get_health_status();
        assert!(health.contains_key("test_exchange"));
    }
    
    #[test]
    fn test_trade_processing() {
        use crate::common::data_types::ExchangeId;
        
        let trade = UnifiedTradeData {
            exchange: ExchangeId::Binance,
            symbol: "BTC-USD".to_string(),
            trade_id: 12345,
            timestamp: 1234567890,
            nanos: 0,
            price: 50000.0,
            size: 0.5,
            side: TradeSide::Buy,
            maker_order_id: [0; 16],
            taker_order_id: [0; 16],
        };
        
        process_trade_data("binance", &trade);
        // Metrics should be recorded (check prometheus registry in integration tests)
    }
}