use prometheus::{Encoder, TextEncoder, register_counter_vec, register_gauge_vec, register_histogram_vec};
use prometheus::{CounterVec, GaugeVec, HistogramVec};
use lazy_static::lazy_static;
use warp::Filter;
use tracing::{info, error};

lazy_static! {
    // HTTP metrics
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_http_requests_total",
        "Total HTTP requests by method, endpoint, and status",
        &["method", "endpoint", "status"]
    ).unwrap();
    
    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "gpu_charts_server_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "endpoint"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
    ).unwrap();
    
    pub static ref CONCURRENT_CONNECTIONS: GaugeVec = register_gauge_vec!(
        "gpu_charts_server_concurrent_connections",
        "Number of concurrent connections",
        &["protocol"]
    ).unwrap();
    
    // Cache metrics
    pub static ref CACHE_HITS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_cache_hits_total",
        "Total cache hits",
        &["cache_type"]
    ).unwrap();
    
    pub static ref CACHE_MISSES_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_cache_misses_total",
        "Total cache misses",
        &["cache_type"]
    ).unwrap();
    
    pub static ref MEMORY_MAPPED_FILES: GaugeVec = register_gauge_vec!(
        "gpu_charts_server_memory_mapped_files",
        "Number of memory-mapped files",
        &["data_type"]
    ).unwrap();
    
    // Data serving metrics
    pub static ref DATA_BYTES_SERVED_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_data_bytes_served_total",
        "Total bytes served",
        &["symbol", "type"]
    ).unwrap();
    
    pub static ref DATA_QUERY_DURATION: HistogramVec = register_histogram_vec!(
        "gpu_charts_server_data_query_duration_seconds",
        "Data query duration in seconds",
        &["symbol"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5]
    ).unwrap();
}

// Helper functions
pub fn record_http_request(method: &str, endpoint: &str, status: u16, duration: f64) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, endpoint, &status.to_string()])
        .inc();
    
    HTTP_REQUEST_DURATION
        .with_label_values(&[method, endpoint])
        .observe(duration);
}

pub fn set_concurrent_connections(protocol: &str, count: f64) {
    CONCURRENT_CONNECTIONS
        .with_label_values(&[protocol])
        .set(count);
}

pub fn increment_cache_hit(cache_type: &str) {
    CACHE_HITS_TOTAL
        .with_label_values(&[cache_type])
        .inc();
}

pub fn increment_cache_miss(cache_type: &str) {
    CACHE_MISSES_TOTAL
        .with_label_values(&[cache_type])
        .inc();
}

pub fn set_memory_mapped_files(data_type: &str, count: f64) {
    MEMORY_MAPPED_FILES
        .with_label_values(&[data_type])
        .set(count);
}

pub fn record_data_served(symbol: &str, data_type: &str, bytes: u64) {
    DATA_BYTES_SERVED_TOTAL
        .with_label_values(&[symbol, data_type])
        .inc_by(bytes as f64);
}

pub fn record_data_query_duration(symbol: &str, duration: f64) {
    DATA_QUERY_DURATION
        .with_label_values(&[symbol])
        .observe(duration);
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
    
    // GET /health endpoint
    let health_route = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&serde_json::json!({
                "status": "ok",
                "service": "gpu-charts-server"
            }))
        });
    
    let routes = metrics_route.or(health_route);
    
    info!("Metrics server listening on http://0.0.0.0:{}/metrics", port);
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}