//! High-performance data management for GPU Charts
//!
//! This crate handles all data fetching, parsing, and GPU buffer management
//! with zero JavaScript boundary crossings for maximum performance.

use gpu_charts_shared::{DataHandle, DataMetadata, DataRequest, Error, Result, TimeRange};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;

pub mod buffer_pool;
pub mod cache;
pub mod fetcher;
pub mod parser;

use buffer_pool::BufferPool;
use cache::{CacheKey, DataCache};

/// Main data manager that handles all data operations
#[wasm_bindgen]
pub struct DataManager {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    cache: Arc<RwLock<DataCache>>,
    buffer_pool: Arc<RwLock<BufferPool>>,
    active_handles: Arc<RwLock<HashMap<Uuid, GpuBufferSet>>>,
    base_url: String,
}

/// Set of GPU buffers for a dataset
pub struct GpuBufferSet {
    pub x_buffers: Vec<wgpu::Buffer>,
    pub y_buffers: HashMap<String, Vec<wgpu::Buffer>>,
    pub metadata: DataMetadata,
}

#[wasm_bindgen]
impl DataManager {
    /// Create a new data manager instance
    #[wasm_bindgen(constructor)]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, base_url: String) -> Self {
        // Note: In real implementation, we'd need to handle device/queue sharing properly
        // For now, this is a placeholder structure
        Self {
            device: Arc::new(unsafe { std::mem::transmute_copy(device) }),
            queue: Arc::new(unsafe { std::mem::transmute_copy(queue) }),
            cache: Arc::new(RwLock::new(DataCache::new(1024 * 1024 * 1024))), // 1GB cache
            buffer_pool: Arc::new(RwLock::new(BufferPool::new(512 * 1024 * 1024))), // 512MB pool
            active_handles: Arc::new(RwLock::new(HashMap::new())),
            base_url,
        }
    }

    /// Fetch data and return a handle
    #[wasm_bindgen]
    pub async fn fetch_data(&mut self, request_json: &str) -> Result<String> {
        let request: DataRequest =
            serde_json::from_str(request_json).map_err(|e| Error::ParseError(e.to_string()))?;

        // Check cache first
        let cache_key = CacheKey::from_request(&request);
        if let Some(handle) = self.cache.read().get(&cache_key) {
            log::info!("Cache hit for {:?}", cache_key);
            return Ok(serde_json::to_string(&handle).unwrap());
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
    #[wasm_bindgen]
    pub fn release_handle(&mut self, handle_id: &str) {
        if let Ok(uuid) = Uuid::parse_str(handle_id) {
            if let Some(buffer_set) = self.active_handles.write().remove(&uuid) {
                // Return buffers to pool
                self.return_buffers_to_pool(buffer_set);
            }
        }
    }

    /// Get statistics about cache and memory usage
    #[wasm_bindgen]
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
        // TODO: Implement actual fetching and parsing
        // This is a placeholder that will be implemented with high-performance
        // HTTP/2 fetching and zero-copy binary parsing

        log::info!("Fetching data for symbol: {}", request.symbol);

        // For now, create dummy buffers
        let metadata = DataMetadata {
            symbol: request.symbol.clone(),
            time_range: request.time_range,
            columns: request.columns.clone(),
            row_count: 1000, // dummy
            byte_size: 8000, // dummy
            creation_time: js_sys::Date::now() as u64,
        };

        Ok(GpuBufferSet {
            x_buffers: vec![],
            y_buffers: HashMap::new(),
            metadata,
        })
    }

    fn return_buffers_to_pool(&self, buffer_set: GpuBufferSet) {
        // TODO: Implement buffer pool return logic
        // This will return buffers to the pool for reuse
    }
}

/// Get buffer info for a handle (used by renderer)
#[wasm_bindgen]
pub fn get_buffer_info(handle_json: &str) -> Result<String> {
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
