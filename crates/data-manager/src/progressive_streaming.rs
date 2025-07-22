//! Progressive streaming for real-time data support
//!
//! This module implements progressive data streaming with backpressure,
//! allowing real-time visualization of incoming data streams.

use bytes::Bytes;
use futures_util::StreamExt;
use gpu_charts_shared::{Error, Result, TimeRange};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, RwLock};

/// Stream configuration
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Buffer size for incoming data
    pub buffer_size: usize,
    /// Maximum chunks in flight
    pub max_chunks_in_flight: usize,
    /// Chunk timeout duration
    pub chunk_timeout: Duration,
    /// Enable adaptive bitrate
    pub adaptive_bitrate: bool,
    /// Target latency in milliseconds
    pub target_latency_ms: u32,
    /// Maximum memory usage in bytes
    pub max_memory_usage: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 65536, // 64KB
            max_chunks_in_flight: 10,
            chunk_timeout: Duration::from_secs(5),
            adaptive_bitrate: true,
            target_latency_ms: 100,
            max_memory_usage: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Progressive data chunk
#[derive(Debug, Clone)]
pub struct DataChunk {
    pub id: u64,
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub metadata: ChunkMetadata,
}

/// Chunk metadata
#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub sequence_number: u64,
    pub time_range: TimeRange,
    pub row_count: u32,
    pub compressed: bool,
    pub compression_type: Option<String>,
}

/// Stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Connecting,
    Streaming,
    Paused,
    Buffering,
    Error,
    Closed,
}

/// Progressive streaming handler
pub struct ProgressiveStreamer {
    config: StreamConfig,
    state: Arc<RwLock<StreamState>>,

    /// Incoming data buffer
    buffer: Arc<RwLock<VecDeque<DataChunk>>>,

    /// Chunk processing channel
    chunk_tx: mpsc::Sender<DataChunk>,
    chunk_rx: Arc<RwLock<mpsc::Receiver<DataChunk>>>,

    /// State broadcast channel
    state_tx: broadcast::Sender<StreamState>,

    /// Performance metrics
    metrics: Arc<RwLock<StreamMetrics>>,

    /// Backpressure controller
    backpressure: Arc<BackpressureController>,
}

/// Stream performance metrics
#[derive(Debug, Default)]
struct StreamMetrics {
    chunks_received: u64,
    chunks_processed: u64,
    bytes_received: u64,
    bytes_processed: u64,
    average_latency_ms: f32,
    current_bitrate_bps: u64,
    buffer_utilization: f32,
    dropped_chunks: u64,
}

impl ProgressiveStreamer {
    /// Create new progressive streamer
    pub fn new(config: StreamConfig) -> Self {
        let (chunk_tx, chunk_rx) = mpsc::channel(config.max_chunks_in_flight);
        let (state_tx, _) = broadcast::channel(10);

        let backpressure = Arc::new(BackpressureController::new(
            config.buffer_size,
            config.max_memory_usage,
        ));

        Self {
            config,
            state: Arc::new(RwLock::new(StreamState::Connecting)),
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            chunk_tx,
            chunk_rx: Arc::new(RwLock::new(chunk_rx)),
            state_tx,
            metrics: Arc::new(RwLock::new(StreamMetrics::default())),
            backpressure,
        }
    }

    /// Start streaming from URL
    pub async fn start_stream(&self, url: &str) -> Result<()> {
        self.set_state(StreamState::Connecting).await;

        // Create HTTP client with streaming support
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Custom(format!("Failed to create client: {}", e)))?;

        // Start streaming request
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Custom(format!("Stream request failed: {}", e)))?;

        if !response.status().is_success() {
            self.set_state(StreamState::Error).await;
            return Err(Error::Custom(format!(
                "Stream error: {}",
                response.status()
            )));
        }

        // Get response stream
        let mut stream = response.bytes_stream();

        self.set_state(StreamState::Streaming).await;

        // Spawn stream processor
        let buffer = Arc::clone(&self.buffer);
        let chunk_tx = self.chunk_tx.clone();
        let metrics = Arc::clone(&self.metrics);
        let backpressure = Arc::clone(&self.backpressure);
        let state = Arc::clone(&self.state);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut sequence_number = 0u64;
            let mut accumulated_data = Vec::new();

            while let Some(result) = stream.next().await {
                match result {
                    Ok(bytes) => {
                        // Apply backpressure
                        if backpressure.should_throttle().await {
                            *state.write().await = StreamState::Buffering;
                            backpressure.wait_for_capacity().await;
                            *state.write().await = StreamState::Streaming;
                        }

                        // Accumulate data
                        accumulated_data.extend_from_slice(&bytes);

                        // Check if we have a complete chunk
                        if let Some(chunk) =
                            Self::parse_chunk(&mut accumulated_data, sequence_number)
                        {
                            sequence_number += 1;

                            // Update metrics
                            let mut metrics = metrics.write().await;
                            metrics.chunks_received += 1;
                            metrics.bytes_received += chunk.data.len() as u64;

                            // Buffer chunk
                            let mut buffer = buffer.write().await;
                            if buffer.len() >= config.buffer_size {
                                // Drop oldest chunk
                                buffer.pop_front();
                                metrics.dropped_chunks += 1;
                            }
                            buffer.push_back(chunk.clone());

                            // Send for processing
                            let _ = chunk_tx.send(chunk).await;
                        }
                    }
                    Err(e) => {
                        log::error!("Stream error: {}", e);
                        *state.write().await = StreamState::Error;
                        break;
                    }
                }
            }

            *state.write().await = StreamState::Closed;
        });

        Ok(())
    }

    /// Process incoming chunks
    pub async fn process_chunks<F, Fut>(&self, mut handler: F) -> Result<()>
    where
        F: FnMut(DataChunk) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut rx = self.chunk_rx.write().await;

        while let Some(chunk) = rx.recv().await {
            let start_time = Instant::now();

            // Process chunk
            handler(chunk.clone()).await?;

            // Update metrics
            let mut metrics = self.metrics.write().await;
            metrics.chunks_processed += 1;
            metrics.bytes_processed += chunk.data.len() as u64;

            let latency_ms = start_time.elapsed().as_millis() as f32;
            metrics.average_latency_ms = (metrics.average_latency_ms * 0.9) + (latency_ms * 0.1);

            // Update backpressure
            self.backpressure.report_processing_time(latency_ms).await;
        }

        Ok(())
    }

    /// Parse chunk from accumulated data
    fn parse_chunk(data: &mut Vec<u8>, sequence_number: u64) -> Option<DataChunk> {
        // Simple protocol: [4 bytes length][8 bytes timestamp][data]
        if data.len() < 12 {
            return None;
        }

        // Read length
        let length = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

        if data.len() < 12 + length {
            return None; // Not enough data yet
        }

        // Read timestamp
        let timestamp = u64::from_le_bytes([
            data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
        ]);

        // Extract chunk data
        let chunk_data = data[12..12 + length].to_vec();

        // Remove processed data
        data.drain(..12 + length);

        Some(DataChunk {
            id: sequence_number,
            timestamp,
            data: chunk_data,
            metadata: ChunkMetadata {
                sequence_number,
                time_range: TimeRange::new(timestamp, timestamp + 1000), // Placeholder
                row_count: 0, // Would be parsed from data
                compressed: false,
                compression_type: None,
            },
        })
    }

    /// Get current stream state
    pub async fn get_state(&self) -> StreamState {
        *self.state.read().await
    }

    /// Set stream state
    async fn set_state(&self, state: StreamState) {
        *self.state.write().await = state;
        let _ = self.state_tx.send(state);
    }

    /// Subscribe to state changes
    pub fn subscribe_state(&self) -> broadcast::Receiver<StreamState> {
        self.state_tx.subscribe()
    }

    /// Get stream statistics
    pub async fn get_stats(&self) -> serde_json::Value {
        let metrics = self.metrics.read().await;
        let buffer_size = self.buffer.read().await.len();
        let state = self.get_state().await;

        serde_json::json!({
            "state": format!("{:?}", state),
            "chunks_received": metrics.chunks_received,
            "chunks_processed": metrics.chunks_processed,
            "bytes_received": metrics.bytes_received,
            "bytes_processed": metrics.bytes_processed,
            "average_latency_ms": metrics.average_latency_ms,
            "current_bitrate_bps": metrics.current_bitrate_bps,
            "buffer_size": buffer_size,
            "buffer_utilization": metrics.buffer_utilization,
            "dropped_chunks": metrics.dropped_chunks,
        })
    }
}

/// Backpressure controller for stream flow control
struct BackpressureController {
    max_buffer_size: usize,
    max_memory_usage: usize,
    current_memory: Arc<RwLock<usize>>,
    processing_times: Arc<RwLock<VecDeque<f32>>>,
    throttle_threshold: f32,
}

impl BackpressureController {
    fn new(max_buffer_size: usize, max_memory_usage: usize) -> Self {
        Self {
            max_buffer_size,
            max_memory_usage,
            current_memory: Arc::new(RwLock::new(0)),
            processing_times: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            throttle_threshold: 100.0, // 100ms
        }
    }

    /// Check if throttling is needed
    async fn should_throttle(&self) -> bool {
        let memory = *self.current_memory.read().await;
        let times = self.processing_times.read().await;

        // Check memory pressure
        if memory > self.max_memory_usage * 8 / 10 {
            return true;
        }

        // Check processing latency
        if !times.is_empty() {
            let avg_time: f32 = times.iter().sum::<f32>() / times.len() as f32;
            if avg_time > self.throttle_threshold {
                return true;
            }
        }

        false
    }

    /// Wait for capacity
    async fn wait_for_capacity(&self) {
        while self.should_throttle().await {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Report processing time
    async fn report_processing_time(&self, time_ms: f32) {
        let mut times = self.processing_times.write().await;
        if times.len() >= 100 {
            times.pop_front();
        }
        times.push_back(time_ms);
    }
}

/// Adaptive bitrate controller
pub struct AdaptiveBitrateController {
    target_latency_ms: u32,
    current_bitrate: Arc<RwLock<u64>>,
    quality_levels: Vec<BitrateLevel>,
}

#[derive(Debug, Clone)]
struct BitrateLevel {
    bitrate: u64,
    quality: f32,
    name: String,
}

impl AdaptiveBitrateController {
    pub fn new(target_latency_ms: u32) -> Self {
        let quality_levels = vec![
            BitrateLevel {
                bitrate: 500_000,
                quality: 0.5,
                name: "Low".to_string(),
            },
            BitrateLevel {
                bitrate: 1_000_000,
                quality: 0.7,
                name: "Medium".to_string(),
            },
            BitrateLevel {
                bitrate: 2_000_000,
                quality: 0.9,
                name: "High".to_string(),
            },
            BitrateLevel {
                bitrate: 5_000_000,
                quality: 1.0,
                name: "Ultra".to_string(),
            },
        ];

        Self {
            target_latency_ms,
            current_bitrate: Arc::new(RwLock::new(quality_levels[1].bitrate)),
            quality_levels,
        }
    }

    /// Adjust bitrate based on network conditions
    pub async fn adjust_bitrate(&self, current_latency_ms: f32, packet_loss: f32) {
        let mut bitrate = self.current_bitrate.write().await;

        // Find current level
        let current_level = self
            .quality_levels
            .iter()
            .position(|l| l.bitrate == *bitrate)
            .unwrap_or(1);

        // Decide adjustment
        if current_latency_ms > self.target_latency_ms as f32 * 1.5 || packet_loss > 0.05 {
            // Decrease quality
            if current_level > 0 {
                *bitrate = self.quality_levels[current_level - 1].bitrate;
                log::info!("Decreasing bitrate to {}", bitrate);
            }
        } else if current_latency_ms < self.target_latency_ms as f32 * 0.7 && packet_loss < 0.01 {
            // Increase quality
            if current_level < self.quality_levels.len() - 1 {
                *bitrate = self.quality_levels[current_level + 1].bitrate;
                log::info!("Increasing bitrate to {}", bitrate);
            }
        }
    }

    /// Get current bitrate
    pub async fn get_bitrate(&self) -> u64 {
        *self.current_bitrate.read().await
    }
}

/// Real-time data buffer with ring buffer semantics
pub struct RealTimeBuffer {
    capacity: usize,
    data: Arc<RwLock<VecDeque<DataChunk>>>,
    latest_timestamp: Arc<RwLock<u64>>,
}

impl RealTimeBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            data: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            latest_timestamp: Arc::new(RwLock::new(0)),
        }
    }

    /// Add chunk to buffer
    pub async fn add_chunk(&self, chunk: DataChunk) {
        let mut data = self.data.write().await;

        // Update latest timestamp
        *self.latest_timestamp.write().await = chunk.timestamp;

        // Add to buffer
        if data.len() >= self.capacity {
            data.pop_front(); // Remove oldest
        }
        data.push_back(chunk);
    }

    /// Get chunks in time range
    pub async fn get_range(&self, time_range: TimeRange) -> Vec<DataChunk> {
        let data = self.data.read().await;

        data.iter()
            .filter(|chunk| {
                chunk.timestamp >= time_range.start && chunk.timestamp <= time_range.end
            })
            .cloned()
            .collect()
    }

    /// Get latest N chunks
    pub async fn get_latest(&self, count: usize) -> Vec<DataChunk> {
        let data = self.data.read().await;

        data.iter().rev().take(count).rev().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_parsing() {
        let mut data = Vec::new();

        // Add chunk: length=4, timestamp=12345, data=[1,2,3,4]
        data.extend_from_slice(&4u32.to_le_bytes());
        data.extend_from_slice(&12345u64.to_le_bytes());
        data.extend_from_slice(&[1, 2, 3, 4]);

        let chunk = ProgressiveStreamer::parse_chunk(&mut data, 0).unwrap();
        assert_eq!(chunk.timestamp, 12345);
        assert_eq!(chunk.data, vec![1, 2, 3, 4]);
        assert!(data.is_empty());
    }

    #[tokio::test]
    async fn test_real_time_buffer() {
        let buffer = RealTimeBuffer::new(3);

        // Add chunks
        for i in 0..5 {
            buffer
                .add_chunk(DataChunk {
                    id: i,
                    timestamp: i * 1000,
                    data: vec![i as u8],
                    metadata: ChunkMetadata {
                        sequence_number: i,
                        time_range: TimeRange::new(i * 1000, (i + 1) * 1000),
                        row_count: 1,
                        compressed: false,
                        compression_type: None,
                    },
                })
                .await;
        }

        // Should only have last 3 chunks
        let latest = buffer.get_latest(10).await;
        assert_eq!(latest.len(), 3);
        assert_eq!(latest[0].id, 2);
        assert_eq!(latest[2].id, 4);
    }
}
