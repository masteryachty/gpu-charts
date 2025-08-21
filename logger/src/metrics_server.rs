use prometheus::{Encoder, TextEncoder, register_counter_vec, register_gauge_vec, register_histogram_vec};
use prometheus::{CounterVec, GaugeVec, HistogramVec};
use lazy_static::lazy_static;
use warp::Filter;
use tracing::info;

lazy_static! {
    // Exchange metrics
    pub static ref EXCHANGE_CONNECTION_STATUS: GaugeVec = register_gauge_vec!(
        "gpu_charts_exchange_connection_status",
        "Connection status for each exchange (1=connected, 0=disconnected)",
        &["exchange"]
    ).unwrap();
    
    pub static ref EXCHANGE_MESSAGES_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_exchange_messages_total",
        "Total messages received from each exchange",
        &["exchange", "message_type"]
    ).unwrap();
    
    pub static ref EXCHANGE_ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_exchange_errors_total",
        "Total errors by exchange and type",
        &["exchange", "error_type"]
    ).unwrap();
    
    pub static ref EXCHANGE_RECONNECTIONS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_exchange_reconnections_total",
        "Total reconnection attempts",
        &["exchange"]
    ).unwrap();
    
    pub static ref WEBSOCKET_LATENCY: HistogramVec = register_histogram_vec!(
        "gpu_charts_websocket_latency_seconds",
        "WebSocket message latency",
        &["exchange"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    ).unwrap();
    
    pub static ref SYMBOLS_MONITORED: GaugeVec = register_gauge_vec!(
        "gpu_charts_symbols_monitored",
        "Number of symbols being monitored per exchange",
        &["exchange"]
    ).unwrap();
    
    // Data quality metrics
    pub static ref LAST_UPDATE_TIMESTAMP: GaugeVec = register_gauge_vec!(
        "gpu_charts_last_update_timestamp",
        "Unix timestamp of last data update",
        &["exchange", "symbol"]
    ).unwrap();
    
    pub static ref DATA_GAPS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_data_gaps_total",
        "Total data gaps detected",
        &["exchange", "symbol"]
    ).unwrap();
    
    pub static ref TRADE_VOLUME_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_trade_volume_total",
        "Total trade volume in USD",
        &["exchange", "symbol", "side"]
    ).unwrap();
    
    pub static ref BID_ASK_SPREAD: GaugeVec = register_gauge_vec!(
        "gpu_charts_bid_ask_spread",
        "Current bid-ask spread as percentage",
        &["exchange", "symbol"]
    ).unwrap();
    
    pub static ref VWAP_DEVIATION: GaugeVec = register_gauge_vec!(
        "gpu_charts_vwap_deviation",
        "Deviation of current price from VWAP",
        &["exchange", "symbol"]
    ).unwrap();
}

// Helper functions to update metrics
pub fn set_connection_status(exchange: &str, connected: bool) {
    EXCHANGE_CONNECTION_STATUS
        .with_label_values(&[exchange])
        .set(if connected { 1.0 } else { 0.0 });
}

pub fn increment_message_count(exchange: &str, message_type: &str) {
    EXCHANGE_MESSAGES_TOTAL
        .with_label_values(&[exchange, message_type])
        .inc();
}

pub fn increment_error_count(exchange: &str, error_type: &str) {
    EXCHANGE_ERRORS_TOTAL
        .with_label_values(&[exchange, error_type])
        .inc();
}

pub fn increment_reconnection_count(exchange: &str) {
    EXCHANGE_RECONNECTIONS_TOTAL
        .with_label_values(&[exchange])
        .inc();
}

pub fn record_websocket_latency(exchange: &str, latency: f64) {
    WEBSOCKET_LATENCY
        .with_label_values(&[exchange])
        .observe(latency);
}

pub fn set_symbols_monitored(exchange: &str, count: f64) {
    SYMBOLS_MONITORED
        .with_label_values(&[exchange])
        .set(count);
}

pub fn update_last_timestamp(exchange: &str, symbol: &str, timestamp: f64) {
    LAST_UPDATE_TIMESTAMP
        .with_label_values(&[exchange, symbol])
        .set(timestamp);
}

pub fn increment_data_gap(exchange: &str, symbol: &str) {
    DATA_GAPS_TOTAL
        .with_label_values(&[exchange, symbol])
        .inc();
}

pub fn record_trade_volume(exchange: &str, symbol: &str, side: &str, volume: f64) {
    TRADE_VOLUME_TOTAL
        .with_label_values(&[exchange, symbol, side])
        .inc_by(volume);
}

pub fn set_bid_ask_spread(exchange: &str, symbol: &str, spread: f64) {
    BID_ASK_SPREAD
        .with_label_values(&[exchange, symbol])
        .set(spread);
}

pub fn set_vwap_deviation(exchange: &str, symbol: &str, deviation: f64) {
    VWAP_DEVIATION
        .with_label_values(&[exchange, symbol])
        .set(deviation);
}

// Additional helper functions used by metrics bridge
pub fn record_message(exchange: &str, message_type: &str) {
    increment_message_count(exchange, message_type);
}

pub fn record_error(exchange: &str, error_type: &str) {
    increment_error_count(exchange, error_type);
}

pub fn record_reconnection(exchange: &str) {
    increment_reconnection_count(exchange);
}

pub fn record_trade(exchange: &str, _symbol: &str) {
    // Track trade count (could add a specific metric for this)
    increment_message_count(exchange, "trade");
}

pub fn record_large_trade(exchange: &str, _symbol: &str) {
    // Track large trades (could add a specific metric for this)
    increment_message_count(exchange, "large_trade");
}

pub fn set_price_high(_exchange: &str, _symbol: &str, _price: f64) {
    // Could track price highs if needed
}

pub fn set_price_low(_exchange: &str, _symbol: &str, _price: f64) {
    // Could track price lows if needed
}

pub fn set_buffer_size(_exchange: &str, _buffer_type: &str, _size: f64) {
    // Could track buffer sizes if needed
}

pub fn record_data_written(_exchange: &str, _data_type: &str, _bytes: u64) {
    // Track data written (could add a specific metric for this)
}

pub fn set_websocket_connections(_exchange: &str, _count: f64) {
    // Track WebSocket connection count
}

/// Start the metrics HTTP server
pub async fn start_metrics_server(port: u16) {
    info!("Starting metrics server on port {}", port);
    
    // GET /metrics endpoint
    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .map(|| {
            let encoder = TextEncoder::new();
            let metric_families = prometheus::gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer).unwrap();
            
            warp::reply::with_header(
                buffer,
                "Content-Type",
                encoder.format_type(),
            )
        });
    
    // GET /health endpoint (for health checks)
    let health_route = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "ok",
                "service": "gpu-charts-logger"
            }))
        });
    
    let routes = metrics_route.or(health_route);
    
    info!("Metrics server listening on http://0.0.0.0:{}/metrics", port);
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}