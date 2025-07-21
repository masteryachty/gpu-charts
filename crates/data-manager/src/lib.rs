//! High-performance data management for GPU Charts
//!
//! This crate handles all data fetching, parsing, and GPU buffer management
//! with zero JavaScript boundary crossings for maximum performance.

use gpu_charts_shared::{DataHandle, DataMetadata, DataRequest, Error, Result, TimeRange};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub mod aggregation;
pub mod buffer_pool;
pub mod cache;
pub mod direct_gpu_parser;
pub mod fetcher;
pub mod parser;
pub mod wasm_api;

use buffer_pool::BufferPool;
use cache::{CacheKey, DataCache};
use fetcher::DataFetcher;
use parser::{BinaryParser, GpuBufferSet};

pub use wasm_api::WasmDataManager;

/// Main data manager that handles all data operations
pub struct DataManager {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    cache: Arc<RwLock<DataCache>>,
    buffer_pool: Arc<RwLock<BufferPool>>,
    active_handles: Arc<RwLock<HashMap<Uuid, GpuBufferSet>>>,
    fetcher: DataFetcher,
    base_url: String,
}

impl DataManager {
    /// Create a new data manager instance with shared device/queue
    pub fn new_with_device(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        base_url: String,
    ) -> Self {
        let fetcher = DataFetcher::new(base_url.clone());

        Self {
            device,
            queue,
            cache: Arc::new(RwLock::new(DataCache::new(1024 * 1024 * 1024))), // 1GB cache
            buffer_pool: Arc::new(RwLock::new(BufferPool::new(512 * 1024 * 1024))), // 512MB pool
            active_handles: Arc::new(RwLock::new(HashMap::new())),
            fetcher,
            base_url,
        }
    }

    /// Fetch data and return a handle
    pub async fn fetch_data(&mut self, request_json: &str) -> Result<String> {
        let request: DataRequest =
            serde_json::from_str(request_json).map_err(|e| Error::ParseError(e.to_string()))?;

        // Check cache first
        let cache_key = CacheKey::from_request(&request);
        {
            let mut cache = self.cache.write();
            if let Some(handle) = cache.get(&cache_key) {
                log::info!("Cache hit for {:?}", cache_key);
                return Ok(serde_json::to_string(&handle).unwrap());
            }
        }

        // Fetch and parse data
        let gpu_buffers = self.fetch_and_parse(&request).await?;

        // Create handle
        let handle = DataHandle {
            id: Uuid::new_v4(),
            metadata: gpu_buffers.metadata.clone(),
        };

        // Store in active handles
        self.active_handles.write().insert(handle.id, gpu_buffers);

        // Add to cache
        self.cache.write().insert(cache_key, handle.clone());

        Ok(serde_json::to_string(&handle).unwrap())
    }

    /// Release a data handle and free GPU resources
    pub fn release_handle(&mut self, handle_id: &str) {
        if let Ok(uuid) = Uuid::parse_str(handle_id) {
            if let Some(buffer_set) = self.active_handles.write().remove(&uuid) {
                // Return buffers to pool
                self.return_buffers_to_pool(buffer_set);
            }
        }
    }

    /// Get statistics about cache and memory usage
    pub fn get_stats(&self) -> String {
        let cache_stats = self.cache.read().get_stats();
        let pool_stats = self.buffer_pool.read().get_stats();

        serde_json::json!({
            "cache": cache_stats,
            "buffer_pool": pool_stats,
            "active_handles": self.active_handles.read().len(),
        })
        .to_string()
    }
}

impl DataManager {
    async fn fetch_and_parse(&self, request: &DataRequest) -> Result<GpuBufferSet> {
        let start_time = js_sys::Date::now();

        // Build URL with query parameters
        let url = format!(
            "{}/api/data?symbol={}&start={}&end={}&columns={}",
            self.base_url,
            request.symbol,
            request.time_range.start,
            request.time_range.end,
            request.columns.join(",")
        );

        // Add aggregation parameters if requested
        let url = if let Some(agg) = &request.aggregation {
            format!(
                "{}&aggregation={:?}&timeframe={}",
                url, agg.aggregation_type, agg.timeframe
            )
        } else {
            url
        };

        log::info!("Fetching data from: {}", url);

        // Fetch binary data
        let binary_data = self.fetcher.fetch_binary(&url).await?;

        let fetch_time = js_sys::Date::now() - start_time;
        log::info!("Fetched {} bytes in {}ms", binary_data.len(), fetch_time);

        // Parse header
        let (header, header_size) = BinaryParser::parse_header(&binary_data)?;

        // Validate data
        BinaryParser::validate_data(&binary_data, &header, header_size)?;

        // Parse directly to GPU buffers
        let parse_start = js_sys::Date::now();
        let buffers = BinaryParser::parse_to_gpu_buffers(
            &self.device,
            &self.queue,
            &binary_data,
            &header,
            header_size,
            &mut self.buffer_pool.write(),
        )?;

        let parse_time = js_sys::Date::now() - parse_start;
        log::info!("Parsed to GPU buffers in {}ms", parse_time);

        // Create metadata
        let metadata = DataMetadata {
            symbol: request.symbol.clone(),
            time_range: request.time_range,
            columns: header.columns.clone(),
            row_count: header.row_count,
            byte_size: binary_data.len() as u64,
            creation_time: js_sys::Date::now() as u64,
        };

        Ok(GpuBufferSet { buffers, metadata })
    }

    fn return_buffers_to_pool(&self, buffer_set: GpuBufferSet) {
        let mut pool = self.buffer_pool.write();

        // Return all buffers to the pool
        for (_, column_buffers) in buffer_set.buffers {
            for buffer in column_buffers {
                pool.release(buffer);
            }
        }
    }
}

/// Get buffer info for a handle (used by renderer)
pub fn get_buffer_info(_handle_json: &str) -> Result<String> {
    // TODO: Implement buffer info retrieval
    // This will be used by the renderer to get GPU buffer references
    Ok("{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let request = DataRequest {
            symbol: "BTC-USD".to_string(),
            time_range: TimeRange::new(1000, 2000),
            columns: vec!["time".to_string(), "price".to_string()],
            aggregation: None,
            max_points: None,
        };

        let key = CacheKey::from_request(&request);
        assert!(key.to_string().contains("BTC-USD"));
    }
}
