//! High-performance HTTP/2 client with connection pooling
//!
//! This module implements an optimized HTTP/2 client with connection pooling,
//! multiplexing, and intelligent request management for ultra-low latency data fetching.

use futures_util::StreamExt;
use gpu_charts_shared::{Error, Result};
use hyper::client::HttpConnector;
use hyper::{Body, Client, Method, Request, Response};
use hyper_tls::HttpsConnector;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};

/// HTTP/2 client configuration
#[derive(Debug, Clone)]
pub struct Http2Config {
    /// Maximum connections per host
    pub max_connections_per_host: usize,
    /// Maximum idle connections
    pub max_idle_connections: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
    /// Enable HTTP/2 server push
    pub enable_push: bool,
    /// Maximum concurrent streams per connection
    pub max_concurrent_streams: u32,
    /// Connection pool size
    pub pool_size: usize,
    /// Enable request pipelining
    pub enable_pipelining: bool,
}

impl Default for Http2Config {
    fn default() -> Self {
        Self {
            max_connections_per_host: 10,
            max_idle_connections: 100,
            idle_timeout: Duration::from_secs(90),
            request_timeout: Duration::from_secs(30),
            enable_push: true,
            max_concurrent_streams: 100,
            pool_size: 50,
            enable_pipelining: true,
        }
    }
}

/// Connection pool entry
struct PooledConnection {
    client: Client<HttpsConnector<HttpConnector>>,
    created_at: Instant,
    last_used: Instant,
    active_streams: Arc<Semaphore>,
    host: String,
}

/// HTTP/2 client with connection pooling
pub struct Http2Client {
    config: Http2Config,
    connection_pool: Arc<RwLock<HashMap<String, Vec<Arc<PooledConnection>>>>>,
    metrics: Arc<RwLock<ClientMetrics>>,
}

/// Client performance metrics
#[derive(Debug, Default)]
struct ClientMetrics {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    total_bytes_received: u64,
    total_latency_ms: u64,
    connection_reuse_count: u64,
    new_connections_created: u64,
}

impl Http2Client {
    /// Create new HTTP/2 client
    pub fn new(config: Http2Config) -> Self {
        Self {
            config,
            connection_pool: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ClientMetrics::default())),
        }
    }

    /// Get or create connection for host
    async fn get_connection(&self, host: &str) -> Result<Arc<PooledConnection>> {
        // Try to get existing connection
        {
            let pool = self.connection_pool.read().await;
            if let Some(connections) = pool.get(host) {
                // Find connection with available streams
                for conn in connections {
                    if conn.active_streams.available_permits() > 0 {
                        let mut metrics = self.metrics.write().await;
                        metrics.connection_reuse_count += 1;

                        return Ok(Arc::clone(conn));
                    }
                }
            }
        }

        // Create new connection
        self.create_new_connection(host).await
    }

    /// Create new HTTP/2 connection
    async fn create_new_connection(&self, host: &str) -> Result<Arc<PooledConnection>> {
        let https = HttpsConnector::new();
        let mut http = HttpConnector::new();
        http.set_nodelay(true);
        http.set_keepalive(Some(Duration::from_secs(60)));

        let client = Client::builder()
            .pool_idle_timeout(self.config.idle_timeout)
            .pool_max_idle_per_host(self.config.max_idle_connections)
            .http2_only(true)
            .http2_initial_stream_window_size(65536)
            .http2_initial_connection_window_size(1048576)
            .http2_max_concurrent_reset_streams(self.config.max_concurrent_streams)
            .http2_adaptive_window(true)
            .build::<_, hyper::Body>(https);

        let connection = Arc::new(PooledConnection {
            client,
            created_at: Instant::now(),
            last_used: Instant::now(),
            active_streams: Arc::new(Semaphore::new(self.config.max_concurrent_streams as usize)),
            host: host.to_string(),
        });

        // Add to pool
        let mut pool = self.connection_pool.write().await;
        pool.entry(host.to_string())
            .or_insert_with(Vec::new)
            .push(Arc::clone(&connection));

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.new_connections_created += 1;

        Ok(connection)
    }

    /// Execute HTTP/2 request
    pub async fn request(&self, url: &str) -> Result<Vec<u8>> {
        let start_time = Instant::now();

        // Parse URL
        let uri = url
            .parse::<hyper::Uri>()
            .map_err(|e| Error::Custom(format!("Invalid URL: {}", e)))?;

        let host = uri
            .host()
            .ok_or_else(|| Error::Custom("No host in URL".into()))?;

        // Get connection
        let connection = self.get_connection(host).await?;

        // Acquire stream permit
        let _permit = connection
            .active_streams
            .acquire()
            .await
            .map_err(|_| Error::Custom("Failed to acquire stream permit".into()))?;

        // Build request
        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("accept", "application/octet-stream")
            .header("accept-encoding", "gzip, deflate, br")
            .body(Body::empty())
            .map_err(|e| Error::Custom(format!("Failed to build request: {}", e)))?;

        // Execute request with timeout
        let response =
            tokio::time::timeout(self.config.request_timeout, connection.client.request(req))
                .await
                .map_err(|_| Error::Custom("Request timeout".into()))?
                .map_err(|e| Error::Custom(format!("Request failed: {}", e)))?;

        // Check status
        if !response.status().is_success() {
            return Err(Error::Custom(format!("HTTP error: {}", response.status())));
        }

        // Read body
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|e| Error::Custom(format!("Failed to read body: {}", e)))?;

        // Update metrics
        let elapsed = start_time.elapsed();
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        metrics.successful_requests += 1;
        metrics.total_bytes_received += body.len() as u64;
        metrics.total_latency_ms += elapsed.as_millis() as u64;

        Ok(body.to_vec())
    }

    /// Execute multiple requests concurrently
    pub async fn request_multiple(&self, urls: Vec<String>) -> Result<Vec<Vec<u8>>> {
        use futures_util::future::join_all;

        let futures = urls
            .into_iter()
            .map(|url| self.request(&url))
            .collect::<Vec<_>>();

        let results = join_all(futures).await;

        // Collect results, propagating first error
        let mut responses = Vec::new();
        for result in results {
            responses.push(result?);
        }

        Ok(responses)
    }

    /// Stream large response
    pub async fn stream_response<F, Fut>(&self, url: &str, mut handler: F) -> Result<()>
    where
        F: FnMut(bytes::Bytes) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let uri = url
            .parse::<hyper::Uri>()
            .map_err(|e| Error::Custom(format!("Invalid URL: {}", e)))?;

        let host = uri
            .host()
            .ok_or_else(|| Error::Custom("No host in URL".into()))?;

        let connection = self.get_connection(host).await?;
        let _permit = connection
            .active_streams
            .acquire()
            .await
            .map_err(|_| Error::Custom("Failed to acquire stream permit".into()))?;

        let req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())
            .map_err(|e| Error::Custom(format!("Failed to build request: {}", e)))?;

        let response = connection
            .client
            .request(req)
            .await
            .map_err(|e| Error::Custom(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Custom(format!("HTTP error: {}", response.status())));
        }

        // Stream body chunks
        let mut body = response.into_body();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| Error::Custom(format!("Stream error: {}", e)))?;
            handler(chunk).await?;
        }

        Ok(())
    }

    /// Clean up idle connections
    pub async fn cleanup_idle_connections(&self) {
        let mut pool = self.connection_pool.write().await;
        let now = Instant::now();

        for (_, connections) in pool.iter_mut() {
            connections
                .retain(|conn| now.duration_since(conn.last_used) < self.config.idle_timeout);
        }

        // Remove empty entries
        pool.retain(|_, connections| !connections.is_empty());
    }

    /// Get client statistics
    pub async fn get_stats(&self) -> serde_json::Value {
        let metrics = self.metrics.read().await;
        let pool = self.connection_pool.read().await;

        let total_connections: usize = pool.values().map(|v| v.len()).sum();
        let avg_latency = if metrics.total_requests > 0 {
            metrics.total_latency_ms as f64 / metrics.total_requests as f64
        } else {
            0.0
        };

        serde_json::json!({
            "total_requests": metrics.total_requests,
            "successful_requests": metrics.successful_requests,
            "failed_requests": metrics.failed_requests,
            "total_bytes_received": metrics.total_bytes_received,
            "avg_latency_ms": avg_latency,
            "connection_reuse_count": metrics.connection_reuse_count,
            "new_connections_created": metrics.new_connections_created,
            "active_connections": total_connections,
            "hosts_connected": pool.len(),
        })
    }
}

/// Multiplexed request manager
pub struct MultiplexedRequests {
    client: Arc<Http2Client>,
    pending_requests: Arc<RwLock<HashMap<uuid::Uuid, PendingRequest>>>,
}

struct PendingRequest {
    url: String,
    created_at: Instant,
    priority: RequestPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl MultiplexedRequests {
    pub fn new(client: Arc<Http2Client>) -> Self {
        Self {
            client,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Queue request with priority
    pub async fn queue_request(&self, url: String, priority: RequestPriority) -> uuid::Uuid {
        let id = uuid::Uuid::new_v4();
        let request = PendingRequest {
            url,
            created_at: Instant::now(),
            priority,
        };

        let mut pending = self.pending_requests.write().await;
        pending.insert(id, request);

        id
    }

    /// Execute queued requests in priority order
    pub async fn execute_batch(&self) -> Result<HashMap<uuid::Uuid, Vec<u8>>> {
        let mut pending = self.pending_requests.write().await;

        // Sort by priority
        let mut requests: Vec<_> = pending.drain().collect();
        requests.sort_by_key(|(_, req)| std::cmp::Reverse(req.priority));

        // Execute in batches
        let mut results = HashMap::new();

        for (id, request) in requests {
            match self.client.request(&request.url).await {
                Ok(data) => {
                    results.insert(id, data);
                }
                Err(e) => {
                    log::error!("Request {} failed: {}", id, e);
                }
            }
        }

        Ok(results)
    }
}

/// Connection warmup for reducing initial latency
pub struct ConnectionWarmer {
    client: Arc<Http2Client>,
}

impl ConnectionWarmer {
    pub fn new(client: Arc<Http2Client>) -> Self {
        Self { client }
    }

    /// Warm up connections to specified hosts
    pub async fn warmup(&self, hosts: Vec<String>) -> Result<()> {
        use futures_util::future::join_all;

        let futures = hosts.into_iter().map(|host| {
            let client = Arc::clone(&self.client);
            async move {
                // Make a lightweight HEAD request to establish connection
                let url = format!("https://{}/", host);
                let _ = client.request(&url).await;
            }
        });

        join_all(futures).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Http2Config::default();
        assert_eq!(config.max_connections_per_host, 10);
        assert_eq!(config.max_concurrent_streams, 100);
    }

    #[tokio::test]
    async fn test_client_creation() {
        let config = Http2Config::default();
        let client = Http2Client::new(config);

        let stats = client.get_stats().await;
        assert_eq!(stats["total_requests"], 0);
        assert_eq!(stats["active_connections"], 0);
    }
}
