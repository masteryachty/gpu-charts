use anyhow::Result;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, 
    CounterVec, Encoder, GaugeVec, HistogramVec, TextEncoder,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

lazy_static::lazy_static! {
    // Exchange metrics
    static ref EXCHANGE_MESSAGES_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_exchange_messages_total",
        "Total number of messages received from exchanges",
        &["exchange", "message_type"]
    ).unwrap();
    
    static ref EXCHANGE_ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_exchange_errors_total", 
        "Total number of errors from exchanges",
        &["exchange", "error_type"]
    ).unwrap();
    
    static ref EXCHANGE_CONNECTION_STATUS: GaugeVec = register_gauge_vec!(
        "gpu_charts_exchange_connection_status",
        "Connection status for each exchange (1=connected, 0=disconnected)",
        &["exchange"]
    ).unwrap();
    
    static ref EXCHANGE_RECONNECTIONS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_exchange_reconnections_total",
        "Total number of reconnection attempts",
        &["exchange"]
    ).unwrap();
    
    static ref SYMBOLS_MONITORED: GaugeVec = register_gauge_vec!(
        "gpu_charts_symbols_monitored",
        "Number of symbols being monitored per exchange",
        &["exchange"]
    ).unwrap();
    
    static ref WEBSOCKET_CONNECTIONS: GaugeVec = register_gauge_vec!(
        "gpu_charts_websocket_connections_active",
        "Number of active WebSocket connections",
        &["exchange"]
    ).unwrap();
    
    // Data metrics
    static ref TRADES_PROCESSED_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_trades_processed_total",
        "Total number of trades processed",
        &["exchange", "symbol"]
    ).unwrap();
    
    static ref DATA_BYTES_WRITTEN_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_data_bytes_written_total",
        "Total bytes written to disk",
        &["exchange", "data_type"]
    ).unwrap();
    
    static ref BUFFER_SIZE_BYTES: GaugeVec = register_gauge_vec!(
        "gpu_charts_buffer_size_bytes",
        "Current buffer size in bytes",
        &["exchange", "buffer_type"]
    ).unwrap();
    
    static ref LARGE_TRADES_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_large_trades_total",
        "Total number of large trades detected (>$10k)",
        &["exchange", "symbol"]
    ).unwrap();
    
    // Performance metrics
    static ref MESSAGE_PROCESSING_DURATION: HistogramVec = register_histogram_vec!(
        "gpu_charts_message_processing_duration_seconds",
        "Time taken to process messages",
        &["exchange", "message_type"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();
    
    static ref FILE_WRITE_DURATION: HistogramVec = register_histogram_vec!(
        "gpu_charts_file_write_duration_seconds",
        "Time taken to write data to files",
        &["exchange", "data_type"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    ).unwrap();
    
    static ref WEBSOCKET_LATENCY: HistogramVec = register_histogram_vec!(
        "gpu_charts_websocket_latency_seconds",
        "WebSocket message round-trip latency",
        &["exchange"],
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).unwrap();
    
    // Business metrics
    static ref TRADE_VOLUME_USD: CounterVec = register_counter_vec!(
        "gpu_charts_trade_volume_usd_total",
        "Total trade volume in USD",
        &["exchange", "symbol", "side"]
    ).unwrap();
    
    static ref VWAP_PRICE: GaugeVec = register_gauge_vec!(
        "gpu_charts_vwap_price",
        "Volume-weighted average price",
        &["exchange", "symbol"]
    ).unwrap();
    
    static ref PRICE_HIGH: GaugeVec = register_gauge_vec!(
        "gpu_charts_price_high",
        "Highest price in reporting period",
        &["exchange", "symbol"]
    ).unwrap();
    
    static ref PRICE_LOW: GaugeVec = register_gauge_vec!(
        "gpu_charts_price_low",
        "Lowest price in reporting period", 
        &["exchange", "symbol"]
    ).unwrap();
    
    // Data quality metrics
    static ref DATA_FRESHNESS_SECONDS: GaugeVec = register_gauge_vec!(
        "gpu_charts_data_freshness_seconds",
        "Seconds since last data update",
        &["exchange", "symbol"]
    ).unwrap();
    
    static ref DATA_GAPS_DETECTED: CounterVec = register_counter_vec!(
        "gpu_charts_data_gaps_detected_total",
        "Number of data gaps detected",
        &["exchange", "symbol"]
    ).unwrap();
    
    static ref PARSE_ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_parse_errors_total",
        "Total number of parse errors",
        &["exchange", "data_type"]
    ).unwrap();
}

/// Tracks the last update time for data freshness calculations
#[derive(Clone)]
pub struct DataFreshnessTracker {
    last_updates: Arc<RwLock<HashMap<String, Instant>>>,
}

impl DataFreshnessTracker {
    pub fn new() -> Self {
        Self {
            last_updates: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn update(&self, exchange: &str, symbol: &str) {
        let key = format!("{}:{}", exchange, symbol);
        let mut updates = self.last_updates.write().await;
        updates.insert(key, Instant::now());
    }
    
    pub async fn get_freshness_seconds(&self, exchange: &str, symbol: &str) -> f64 {
        let key = format!("{}:{}", exchange, symbol);
        let updates = self.last_updates.read().await;
        updates.get(&key)
            .map(|instant| instant.elapsed().as_secs_f64())
            .unwrap_or(f64::INFINITY)
    }
    
    pub async fn update_metrics(&self) {
        let updates = self.last_updates.read().await;
        for (key, instant) in updates.iter() {
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() == 2 {
                DATA_FRESHNESS_SECONDS
                    .with_label_values(&[parts[0], parts[1]])
                    .set(instant.elapsed().as_secs_f64());
            }
        }
    }
}

/// Main metrics exporter that pushes to Prometheus Push Gateway
pub struct MetricsExporter {
    push_gateway_url: String,
    job_name: String,
    instance: String,
    push_interval: Duration,
    freshness_tracker: DataFreshnessTracker,
}

impl MetricsExporter {
    pub fn new(push_gateway_url: String, instance: String) -> Self {
        Self {
            push_gateway_url,
            job_name: "gpu_charts_logger".to_string(),
            instance,
            push_interval: Duration::from_secs(15),
            freshness_tracker: DataFreshnessTracker::new(),
        }
    }
    
    /// Start the metrics push loop
    pub async fn start(self: Arc<Self>) {
        let push_handle = {
            let exporter = self.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(exporter.push_interval);
                loop {
                    interval.tick().await;
                    
                    // Update data freshness metrics
                    exporter.freshness_tracker.update_metrics().await;
                    
                    if let Err(e) = exporter.push_metrics().await {
                        error!("Failed to push metrics: {}", e);
                    }
                }
            })
        };
        
        info!("Metrics exporter started, pushing to {}", self.push_gateway_url);
        
        // Keep the task running
        let _ = push_handle.await;
    }
    
    /// Push metrics to Prometheus Push Gateway
    async fn push_metrics(&self) -> Result<()> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        
        let url = format!(
            "{}/metrics/job/{}/instance/{}",
            self.push_gateway_url,
            self.job_name,
            self.instance
        );
        
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .body(buffer)
            .header("Content-Type", encoder.format_type())
            .timeout(Duration::from_secs(5))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            warn!("Push gateway returned error {}: {}", status, text);
        } else {
            debug!("Successfully pushed metrics to Prometheus");
        }
        
        Ok(())
    }
    
    pub fn freshness_tracker(&self) -> &DataFreshnessTracker {
        &self.freshness_tracker
    }
}

/// Helper functions for recording metrics
pub mod helpers {
    use super::*;
    use std::time::Instant;
    
    pub fn record_message(exchange: &str, message_type: &str) {
        EXCHANGE_MESSAGES_TOTAL
            .with_label_values(&[exchange, message_type])
            .inc();
    }
    
    pub fn record_error(exchange: &str, error_type: &str) {
        EXCHANGE_ERRORS_TOTAL
            .with_label_values(&[exchange, error_type])
            .inc();
    }
    
    pub fn set_connection_status(exchange: &str, connected: bool) {
        EXCHANGE_CONNECTION_STATUS
            .with_label_values(&[exchange])
            .set(if connected { 1.0 } else { 0.0 });
    }
    
    pub fn record_reconnection(exchange: &str) {
        EXCHANGE_RECONNECTIONS_TOTAL
            .with_label_values(&[exchange])
            .inc();
    }
    
    pub fn set_symbols_monitored(exchange: &str, count: usize) {
        SYMBOLS_MONITORED
            .with_label_values(&[exchange])
            .set(count as f64);
    }
    
    pub fn set_websocket_connections(exchange: &str, count: usize) {
        WEBSOCKET_CONNECTIONS
            .with_label_values(&[exchange])
            .set(count as f64);
    }
    
    pub fn record_trade(exchange: &str, symbol: &str) {
        TRADES_PROCESSED_TOTAL
            .with_label_values(&[exchange, symbol])
            .inc();
    }
    
    pub fn record_data_written(exchange: &str, data_type: &str, bytes: usize) {
        DATA_BYTES_WRITTEN_TOTAL
            .with_label_values(&[exchange, data_type])
            .inc_by(bytes as f64);
    }
    
    pub fn set_buffer_size(exchange: &str, buffer_type: &str, bytes: usize) {
        BUFFER_SIZE_BYTES
            .with_label_values(&[exchange, buffer_type])
            .set(bytes as f64);
    }
    
    pub fn record_large_trade(exchange: &str, symbol: &str) {
        LARGE_TRADES_TOTAL
            .with_label_values(&[exchange, symbol])
            .inc();
    }
    
    pub fn record_trade_volume(exchange: &str, symbol: &str, side: &str, volume_usd: f64) {
        TRADE_VOLUME_USD
            .with_label_values(&[exchange, symbol, side])
            .inc_by(volume_usd);
    }
    
    pub fn set_vwap(exchange: &str, symbol: &str, price: f64) {
        VWAP_PRICE
            .with_label_values(&[exchange, symbol])
            .set(price);
    }
    
    pub fn set_price_high(exchange: &str, symbol: &str, price: f64) {
        PRICE_HIGH
            .with_label_values(&[exchange, symbol])
            .set(price);
    }
    
    pub fn set_price_low(exchange: &str, symbol: &str, price: f64) {
        PRICE_LOW
            .with_label_values(&[exchange, symbol])
            .set(price);
    }
    
    pub fn record_data_gap(exchange: &str, symbol: &str) {
        DATA_GAPS_DETECTED
            .with_label_values(&[exchange, symbol])
            .inc();
    }
    
    pub fn record_parse_error(exchange: &str, data_type: &str) {
        PARSE_ERRORS_TOTAL
            .with_label_values(&[exchange, data_type])
            .inc();
    }
    
    /// Timer for measuring operation duration
    pub struct Timer {
        start: Instant,
    }
    
    impl Timer {
        pub fn start() -> Self {
            Self {
                start: Instant::now(),
            }
        }
        
        pub fn observe_message_processing(self, exchange: &str, message_type: &str) {
            MESSAGE_PROCESSING_DURATION
                .with_label_values(&[exchange, message_type])
                .observe(self.start.elapsed().as_secs_f64());
        }
        
        pub fn observe_file_write(self, exchange: &str, data_type: &str) {
            FILE_WRITE_DURATION
                .with_label_values(&[exchange, data_type])
                .observe(self.start.elapsed().as_secs_f64());
        }
        
        pub fn observe_websocket_latency(self, exchange: &str) {
            WEBSOCKET_LATENCY
                .with_label_values(&[exchange])
                .observe(self.start.elapsed().as_secs_f64());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_freshness_tracker() {
        let tracker = DataFreshnessTracker::new();
        
        // Update a symbol
        tracker.update("binance", "BTC-USD").await;
        
        // Should be very fresh
        tokio::time::sleep(Duration::from_millis(100)).await;
        let freshness = tracker.get_freshness_seconds("binance", "BTC-USD").await;
        assert!(freshness < 1.0);
        
        // Non-existent symbol should return infinity
        let freshness = tracker.get_freshness_seconds("binance", "ETH-USD").await;
        assert_eq!(freshness, f64::INFINITY);
    }
    
    #[test]
    fn test_metrics_recording() {
        helpers::record_message("binance", "ticker");
        helpers::set_connection_status("binance", true);
        helpers::record_trade("binance", "BTC-USD");
        helpers::record_data_written("binance", "trades", 1024);
        
        // Verify metrics are recorded (would need to check prometheus registry)
        let families = prometheus::gather();
        assert!(!families.is_empty());
    }
}