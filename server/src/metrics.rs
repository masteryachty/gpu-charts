use anyhow::Result;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, Encoder, GaugeVec, HistogramVec, TextEncoder,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;

lazy_static::lazy_static! {
    // HTTP metrics
    static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_http_requests_total",
        "Total number of HTTP requests",
        &["method", "endpoint", "status"]
    ).unwrap();
    
    static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "gpu_charts_server_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "endpoint"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]
    ).unwrap();
    
    static ref HTTP_RESPONSE_SIZE_BYTES: HistogramVec = register_histogram_vec!(
        "gpu_charts_server_http_response_size_bytes",
        "HTTP response size in bytes",
        &["endpoint"],
        vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0, 100000000.0]
    ).unwrap();
    
    static ref CONCURRENT_CONNECTIONS: GaugeVec = register_gauge_vec!(
        "gpu_charts_server_concurrent_connections",
        "Number of concurrent connections",
        &["protocol"]
    ).unwrap();
    
    // Data query metrics
    static ref DATA_QUERIES_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_data_queries_total",
        "Total number of data queries",
        &["symbol", "type"]
    ).unwrap();
    
    static ref DATA_QUERY_DURATION: HistogramVec = register_histogram_vec!(
        "gpu_charts_server_data_query_duration_seconds",
        "Time taken to process data queries",
        &["symbol"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();
    
    static ref DATA_BYTES_SERVED: CounterVec = register_counter_vec!(
        "gpu_charts_server_data_bytes_served_total",
        "Total bytes of data served",
        &["symbol", "type"]
    ).unwrap();
    
    static ref DATA_RECORDS_SERVED: CounterVec = register_counter_vec!(
        "gpu_charts_server_data_records_served_total",
        "Total number of records served",
        &["symbol", "type"]
    ).unwrap();
    
    // Cache metrics
    static ref CACHE_HITS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_cache_hits_total",
        "Total number of cache hits",
        &["cache_type"]
    ).unwrap();
    
    static ref CACHE_MISSES_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_cache_misses_total",
        "Total number of cache misses",
        &["cache_type"]
    ).unwrap();
    
    static ref MEMORY_MAPPED_FILES: GaugeVec = register_gauge_vec!(
        "gpu_charts_server_memory_mapped_files",
        "Number of memory-mapped files currently cached",
        &["exchange"]
    ).unwrap();
    
    static ref CACHE_MEMORY_BYTES: GaugeVec = register_gauge_vec!(
        "gpu_charts_server_cache_memory_bytes",
        "Memory used by caches in bytes",
        &["cache_type"]
    ).unwrap();
    
    // Symbol search metrics
    static ref SYMBOL_SEARCHES_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_symbol_searches_total",
        "Total number of symbol searches",
        &["status"]
    ).unwrap();
    
    static ref SYMBOL_SEARCH_DURATION: HistogramVec = register_histogram_vec!(
        "gpu_charts_server_symbol_search_duration_seconds",
        "Time taken for symbol searches",
        &["query_length"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1]
    ).unwrap();
    
    // File I/O metrics
    static ref FILE_READ_DURATION: HistogramVec = register_histogram_vec!(
        "gpu_charts_server_file_read_duration_seconds",
        "Time taken to read files",
        &["operation"],
        vec![0.00001, 0.0001, 0.001, 0.01, 0.1, 1.0]
    ).unwrap();
    
    static ref FILE_OPEN_ERRORS: CounterVec = register_counter_vec!(
        "gpu_charts_server_file_open_errors_total",
        "Total number of file open errors",
        &["exchange", "reason"]
    ).unwrap();
    
    // Status endpoint metrics
    static ref STATUS_CHECKS_TOTAL: CounterVec = register_counter_vec!(
        "gpu_charts_server_status_checks_total",
        "Total number of status checks",
        &["exchange"]
    ).unwrap();
    
    static ref EXCHANGE_DATA_FRESHNESS: GaugeVec = register_gauge_vec!(
        "gpu_charts_server_exchange_data_freshness_seconds",
        "Seconds since last data update for each exchange",
        &["exchange", "symbol"]
    ).unwrap();
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
    
    pub fn observe_http_request(self, method: &str, endpoint: &str) {
        HTTP_REQUEST_DURATION
            .with_label_values(&[method, endpoint])
            .observe(self.start.elapsed().as_secs_f64());
    }
    
    pub fn observe_data_query(self, symbol: &str) {
        DATA_QUERY_DURATION
            .with_label_values(&[symbol])
            .observe(self.start.elapsed().as_secs_f64());
    }
    
    pub fn observe_symbol_search(self, query_length: usize) {
        let length_bucket = match query_length {
            0..=2 => "short",
            3..=5 => "medium",
            6..=10 => "long",
            _ => "very_long",
        };
        SYMBOL_SEARCH_DURATION
            .with_label_values(&[length_bucket])
            .observe(self.start.elapsed().as_secs_f64());
    }
    
    pub fn observe_file_read(self, operation: &str) {
        FILE_READ_DURATION
            .with_label_values(&[operation])
            .observe(self.start.elapsed().as_secs_f64());
    }
}

/// Helper functions for recording metrics
pub mod helpers {
    use super::*;
    
    pub fn record_http_request(method: &str, endpoint: &str, status: u16) {
        let status_str = status.to_string();
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[method, endpoint, &status_str])
            .inc();
    }
    
    pub fn record_response_size(endpoint: &str, size: usize) {
        HTTP_RESPONSE_SIZE_BYTES
            .with_label_values(&[endpoint])
            .observe(size as f64);
    }
    
    pub fn set_concurrent_connections(protocol: &str, count: usize) {
        CONCURRENT_CONNECTIONS
            .with_label_values(&[protocol])
            .set(count as f64);
    }
    
    pub fn record_data_query(symbol: &str, data_type: &str) {
        DATA_QUERIES_TOTAL
            .with_label_values(&[symbol, data_type])
            .inc();
    }
    
    pub fn record_data_served(symbol: &str, data_type: &str, bytes: usize, records: usize) {
        DATA_BYTES_SERVED
            .with_label_values(&[symbol, data_type])
            .inc_by(bytes as f64);
        
        DATA_RECORDS_SERVED
            .with_label_values(&[symbol, data_type])
            .inc_by(records as f64);
    }
    
    pub fn record_cache_hit(cache_type: &str) {
        CACHE_HITS_TOTAL
            .with_label_values(&[cache_type])
            .inc();
    }
    
    pub fn record_cache_miss(cache_type: &str) {
        CACHE_MISSES_TOTAL
            .with_label_values(&[cache_type])
            .inc();
    }
    
    pub fn set_memory_mapped_files(exchange: &str, count: usize) {
        MEMORY_MAPPED_FILES
            .with_label_values(&[exchange])
            .set(count as f64);
    }
    
    pub fn set_cache_memory(cache_type: &str, bytes: usize) {
        CACHE_MEMORY_BYTES
            .with_label_values(&[cache_type])
            .set(bytes as f64);
    }
    
    pub fn record_symbol_search(success: bool) {
        let status = if success { "success" } else { "failure" };
        SYMBOL_SEARCHES_TOTAL
            .with_label_values(&[status])
            .inc();
    }
    
    pub fn record_file_open_error(exchange: &str, reason: &str) {
        FILE_OPEN_ERRORS
            .with_label_values(&[exchange, reason])
            .inc();
    }
    
    pub fn record_status_check(exchange: &str) {
        STATUS_CHECKS_TOTAL
            .with_label_values(&[exchange])
            .inc();
    }
    
    pub fn set_data_freshness(exchange: &str, symbol: &str, seconds: f64) {
        EXCHANGE_DATA_FRESHNESS
            .with_label_values(&[exchange, symbol])
            .set(seconds);
    }
}

/// Metrics exporter that pushes to Prometheus Push Gateway
pub struct MetricsExporter {
    push_gateway_url: String,
    job_name: String,
    instance: String,
    push_interval: Duration,
}

impl MetricsExporter {
    pub fn new(push_gateway_url: String, instance: String) -> Self {
        Self {
            push_gateway_url,
            job_name: "gpu_charts_server".to_string(),
            instance,
            push_interval: Duration::from_secs(15),
        }
    }
    
    /// Start the metrics push loop
    pub async fn start(self: Arc<Self>) {
        let mut interval_timer = interval(self.push_interval);
        
        loop {
            interval_timer.tick().await;
            
            if let Err(e) = self.push_metrics().await {
                eprintln!("Failed to push metrics: {}", e);
            }
        }
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
            eprintln!("Push gateway returned error {}: {}", status, text);
        }
        
        Ok(())
    }
}

/// Middleware for tracking HTTP requests
pub struct MetricsMiddleware {
    pub timer: Timer,
    pub method: String,
    pub endpoint: String,
}

impl MetricsMiddleware {
    pub fn new(method: &str, endpoint: &str) -> Self {
        Self {
            timer: Timer::start(),
            method: method.to_string(),
            endpoint: endpoint.to_string(),
        }
    }
    
    pub fn complete(self, status: u16, response_size: usize) {
        helpers::record_http_request(&self.method, &self.endpoint, status);
        helpers::record_response_size(&self.endpoint, response_size);
        self.timer.observe_http_request(&self.method, &self.endpoint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(Duration::from_millis(10));
        timer.observe_http_request("GET", "/api/data");
        
        // Verify metrics are recorded
        let families = prometheus::gather();
        assert!(!families.is_empty());
    }
    
    #[test]
    fn test_metrics_recording() {
        helpers::record_http_request("GET", "/api/symbols", 200);
        helpers::record_data_query("BTC-USD", "MD");
        helpers::record_cache_hit("mmap");
        helpers::record_symbol_search(true);
        
        let families = prometheus::gather();
        assert!(!families.is_empty());
    }
}