//! Intelligent request batching and coalescing
//!
//! This module implements smart request batching to reduce network overhead
//! by coalescing multiple requests into efficient batches.

use crate::http2_client::Http2Client;
use gpu_charts_shared::{Error, Result, TimeRange};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot, RwLock};

/// Request priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Batched request
#[derive(Debug, Clone)]
pub struct BatchedRequest {
    pub id: uuid::Uuid,
    pub url: String,
    pub priority: Priority,
    pub created_at: Instant,
    pub response_tx: oneshot::Sender<Result<Vec<u8>>>,
}

/// Batch configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum requests per batch
    pub max_batch_size: usize,
    /// Maximum wait time before flushing batch
    pub max_wait_time: Duration,
    /// Minimum wait time to accumulate requests
    pub min_wait_time: Duration,
    /// Enable request deduplication
    pub enable_deduplication: bool,
    /// Maximum concurrent batches
    pub max_concurrent_batches: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 50,
            max_wait_time: Duration::from_millis(100),
            min_wait_time: Duration::from_millis(10),
            enable_deduplication: true,
            max_concurrent_batches: 5,
        }
    }
}

/// Request batcher for intelligent coalescing
pub struct RequestBatcher {
    config: BatchConfig,
    client: Arc<Http2Client>,

    /// Pending requests queue
    pending: Arc<RwLock<VecDeque<BatchedRequest>>>,

    /// Active request tracking for deduplication
    active_requests: Arc<RwLock<HashMap<String, Vec<oneshot::Sender<Result<Vec<u8>>>>>>>,

    /// Batch processing channel
    batch_tx: mpsc::Sender<Vec<BatchedRequest>>,

    /// Statistics
    stats: Arc<RwLock<BatcherStats>>,
}

/// Batcher statistics
#[derive(Debug, Default)]
struct BatcherStats {
    total_requests: u64,
    batches_created: u64,
    requests_deduplicated: u64,
    total_batch_time_ms: u64,
    largest_batch: usize,
}

impl RequestBatcher {
    /// Create new request batcher
    pub fn new(config: BatchConfig, client: Arc<Http2Client>) -> Self {
        let (batch_tx, mut batch_rx) = mpsc::channel::<Vec<BatchedRequest>>(100);

        let pending = Arc::new(RwLock::new(VecDeque::new()));
        let active_requests = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(BatcherStats::default()));

        // Spawn batch processor
        let client_clone = Arc::clone(&client);
        let active_clone = Arc::clone(&active_requests);
        let stats_clone = Arc::clone(&stats);
        let max_concurrent = config.max_concurrent_batches;

        tokio::spawn(async move {
            let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

            while let Some(batch) = batch_rx.recv().await {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let client = Arc::clone(&client_clone);
                let active = Arc::clone(&active_clone);
                let stats = Arc::clone(&stats_clone);

                tokio::spawn(async move {
                    Self::process_batch(batch, client, active, stats).await;
                    drop(permit);
                });
            }
        });

        // Spawn batch coordinator
        let pending_clone = Arc::clone(&pending);
        let batch_tx_clone = batch_tx.clone();
        let config_clone = config.clone();

        tokio::spawn(async move {
            Self::batch_coordinator(pending_clone, batch_tx_clone, config_clone).await;
        });

        Self {
            config,
            client,
            pending,
            active_requests,
            batch_tx,
            stats,
        }
    }

    /// Submit request for batching
    pub async fn submit(&self, url: String, priority: Priority) -> Result<Vec<u8>> {
        let (tx, rx) = oneshot::channel();

        // Check for deduplication
        if self.config.enable_deduplication {
            let mut active = self.active_requests.write().await;
            if let Some(waiters) = active.get_mut(&url) {
                // Request already in flight, add to waiters
                waiters.push(tx);

                let mut stats = self.stats.write().await;
                stats.requests_deduplicated += 1;

                return rx
                    .await
                    .map_err(|_| Error::Custom("Request cancelled".into()))?;
            }

            // Add as new active request
            active.insert(url.clone(), vec![tx]);
        }

        // Create batched request
        let request = BatchedRequest {
            id: uuid::Uuid::new_v4(),
            url,
            priority,
            created_at: Instant::now(),
            response_tx: if self.config.enable_deduplication {
                // Create dummy sender since we already added to active
                let (dummy_tx, _) = oneshot::channel();
                dummy_tx
            } else {
                tx
            },
        };

        // Add to pending queue
        let mut pending = self.pending.write().await;

        // Insert based on priority
        let insert_pos = pending
            .iter()
            .position(|r| r.priority < priority)
            .unwrap_or(pending.len());
        pending.insert(insert_pos, request);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;

        drop(pending);
        drop(stats);

        // Wait for response
        rx.await
            .map_err(|_| Error::Custom("Request cancelled".into()))?
    }

    /// Batch coordinator task
    async fn batch_coordinator(
        pending: Arc<RwLock<VecDeque<BatchedRequest>>>,
        batch_tx: mpsc::Sender<Vec<BatchedRequest>>,
        config: BatchConfig,
    ) {
        let mut last_batch_time = Instant::now();

        loop {
            tokio::time::sleep(config.min_wait_time).await;

            let mut pending_guard = pending.write().await;

            if pending_guard.is_empty() {
                continue;
            }

            let elapsed = last_batch_time.elapsed();
            let should_flush =
                elapsed >= config.max_wait_time || pending_guard.len() >= config.max_batch_size;

            if should_flush {
                // Create batch
                let batch_size = pending_guard.len().min(config.max_batch_size);
                let batch: Vec<_> = pending_guard.drain(..batch_size).collect();

                drop(pending_guard);

                // Send batch for processing
                if batch_tx.send(batch).await.is_err() {
                    break; // Receiver dropped
                }

                last_batch_time = Instant::now();
            }
        }
    }

    /// Process a batch of requests
    async fn process_batch(
        batch: Vec<BatchedRequest>,
        client: Arc<Http2Client>,
        active_requests: Arc<RwLock<HashMap<String, Vec<oneshot::Sender<Result<Vec<u8>>>>>>>,
        stats: Arc<RwLock<BatcherStats>>,
    ) {
        let start_time = Instant::now();
        let batch_size = batch.len();

        // Execute requests concurrently
        let futures: Vec<_> = batch
            .into_iter()
            .map(|request| {
                let client = Arc::clone(&client);
                let url = request.url.clone();

                async move {
                    let result = client.request(&url).await;
                    (url, result)
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        // Distribute results
        let mut active = active_requests.write().await;

        for (url, result) in results {
            if let Some(mut waiters) = active.remove(&url) {
                // Send to all waiters
                let result_ref = Arc::new(result);

                for waiter in waiters.drain(..) {
                    let _ = waiter.send(match &**result_ref {
                        Ok(data) => Ok(data.clone()),
                        Err(e) => Err(Error::Custom(e.to_string())),
                    });
                }
            }
        }

        // Update statistics
        let mut stats = stats.write().await;
        stats.batches_created += 1;
        stats.total_batch_time_ms += start_time.elapsed().as_millis() as u64;
        if batch_size > stats.largest_batch {
            stats.largest_batch = batch_size;
        }
    }

    /// Get batcher statistics
    pub async fn get_stats(&self) -> serde_json::Value {
        let stats = self.stats.read().await;
        let pending_count = self.pending.read().await.len();
        let active_count = self.active_requests.read().await.len();

        let avg_batch_time = if stats.batches_created > 0 {
            stats.total_batch_time_ms as f64 / stats.batches_created as f64
        } else {
            0.0
        };

        serde_json::json!({
            "total_requests": stats.total_requests,
            "batches_created": stats.batches_created,
            "requests_deduplicated": stats.requests_deduplicated,
            "avg_batch_time_ms": avg_batch_time,
            "largest_batch": stats.largest_batch,
            "pending_requests": pending_count,
            "active_requests": active_count,
            "deduplication_rate": if stats.total_requests > 0 {
                stats.requests_deduplicated as f64 / stats.total_requests as f64
            } else {
                0.0
            },
        })
    }
}

/// Smart request coalescer for time-series data
pub struct TimeSeriesCoalescer {
    batcher: Arc<RequestBatcher>,
    /// Minimum time range overlap for coalescing
    min_overlap_ratio: f32,
}

impl TimeSeriesCoalescer {
    pub fn new(batcher: Arc<RequestBatcher>) -> Self {
        Self {
            batcher,
            min_overlap_ratio: 0.5,
        }
    }

    /// Submit time-series request with coalescing
    pub async fn request_time_range(
        &self,
        symbol: &str,
        time_range: TimeRange,
        columns: Vec<String>,
        priority: Priority,
    ) -> Result<Vec<u8>> {
        // Build URL
        let url = format!(
            "/api/data?symbol={}&start={}&end={}&columns={}",
            symbol,
            time_range.start,
            time_range.end,
            columns.join(",")
        );

        // Submit through batcher
        self.batcher.submit(url, priority).await
    }

    /// Analyze requests for coalescing opportunities
    pub fn analyze_coalescing_opportunity(
        &self,
        requests: &[(String, TimeRange)],
    ) -> Vec<Vec<usize>> {
        let mut groups = Vec::new();
        let mut processed = vec![false; requests.len()];

        for i in 0..requests.len() {
            if processed[i] {
                continue;
            }

            let mut group = vec![i];
            processed[i] = true;

            let (symbol_i, range_i) = &requests[i];

            for j in (i + 1)..requests.len() {
                if processed[j] {
                    continue;
                }

                let (symbol_j, range_j) = &requests[j];

                // Check if same symbol and overlapping
                if symbol_i == symbol_j && self.ranges_overlap(range_i, range_j) {
                    group.push(j);
                    processed[j] = true;
                }
            }

            if group.len() > 1 {
                groups.push(group);
            }
        }

        groups
    }

    /// Check if two time ranges overlap sufficiently
    fn ranges_overlap(&self, a: &TimeRange, b: &TimeRange) -> bool {
        let overlap_start = a.start.max(b.start);
        let overlap_end = a.end.min(b.end);

        if overlap_start >= overlap_end {
            return false;
        }

        let overlap_duration = overlap_end - overlap_start;
        let min_duration = (a.duration().min(b.duration()) as f32 * self.min_overlap_ratio) as u64;

        overlap_duration >= min_duration
    }
}

/// Request predictor for prefetching
pub struct RequestPredictor {
    /// History of requests
    history: Arc<RwLock<VecDeque<PredictorEntry>>>,
    /// Maximum history size
    max_history: usize,
}

#[derive(Debug, Clone)]
struct PredictorEntry {
    symbol: String,
    time_range: TimeRange,
    timestamp: Instant,
}

impl RequestPredictor {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
        }
    }

    /// Record a request
    pub async fn record_request(&self, symbol: String, time_range: TimeRange) {
        let mut history = self.history.write().await;

        if history.len() >= self.max_history {
            history.pop_front();
        }

        history.push_back(PredictorEntry {
            symbol,
            time_range,
            timestamp: Instant::now(),
        });
    }

    /// Predict next likely requests
    pub async fn predict_next(&self, current_range: &TimeRange) -> Vec<TimeRange> {
        let history = self.history.read().await;

        if history.len() < 3 {
            return Vec::new();
        }

        // Simple prediction: look for patterns in range progression
        let mut predictions = Vec::new();

        // Check for panning pattern
        let recent: Vec<_> = history.iter().rev().take(5).collect();
        if recent.len() >= 2 {
            let delta = recent[0].time_range.start as i64 - recent[1].time_range.start as i64;

            if delta != 0 {
                // Predict continuation of pan
                let next_start = (current_range.start as i64 + delta).max(0) as u64;
                let next_end = (current_range.end as i64 + delta).max(0) as u64;

                predictions.push(TimeRange::new(next_start, next_end));
            }
        }

        // Check for zoom pattern
        if recent.len() >= 2 {
            let duration_ratio =
                recent[0].time_range.duration() as f64 / recent[1].time_range.duration() as f64;

            if (duration_ratio - 1.0).abs() > 0.1 {
                // Predict continuation of zoom
                let next_duration = (current_range.duration() as f64 * duration_ratio) as u64;
                let center = (current_range.start + current_range.end) / 2;
                let next_start = center.saturating_sub(next_duration / 2);
                let next_end = center + next_duration / 2;

                predictions.push(TimeRange::new(next_start, next_end));
            }
        }

        predictions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn test_range_overlap() {
        let coalescer = TimeSeriesCoalescer {
            batcher: Arc::new(RequestBatcher::new(
                BatchConfig::default(),
                Arc::new(Http2Client::new(Default::default())),
            )),
            min_overlap_ratio: 0.5,
        };

        let a = TimeRange::new(0, 100);
        let b = TimeRange::new(50, 150);
        assert!(coalescer.ranges_overlap(&a, &b));

        let c = TimeRange::new(0, 100);
        let d = TimeRange::new(101, 200);
        assert!(!coalescer.ranges_overlap(&c, &d));
    }

    #[tokio::test]
    async fn test_request_prediction() {
        let predictor = RequestPredictor::new(10);

        // Record panning pattern
        predictor
            .record_request("BTC".to_string(), TimeRange::new(0, 100))
            .await;
        predictor
            .record_request("BTC".to_string(), TimeRange::new(10, 110))
            .await;
        predictor
            .record_request("BTC".to_string(), TimeRange::new(20, 120))
            .await;

        let predictions = predictor.predict_next(&TimeRange::new(20, 120)).await;
        assert!(!predictions.is_empty());

        // Should predict next pan to 30-130
        assert_eq!(predictions[0].start, 30);
        assert_eq!(predictions[0].end, 130);
    }
}
