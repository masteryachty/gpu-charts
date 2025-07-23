//! Data Manager crate for GPU Charts
//! Handles all data operations with focus on performance and GPU optimization

pub mod data_retriever;
pub mod data_store;
pub mod cache;
pub mod binary_parser;

use std::collections::HashMap;
use std::sync::Arc;
use wgpu::{Device, Queue};
use wgpu::util::DeviceExt;
use shared_types::{DataHandle, DataMetadata, GpuBufferSet, ParsedData, GpuChartsError, GpuChartsResult};
use uuid::Uuid;

pub use data_retriever::{ApiHeader, ColumnMeta, create_gpu_buffer_from_vec, fetch_api_response, create_chunked_gpu_buffer_from_arraybuffer};
pub use data_store::{DataStore, DataSeries, MetricSeries, ScreenDimensions, ChartType, Vertex};

/// Main data manager that coordinates all data operations
pub struct DataManager {
    device: Arc<Device>,
    queue: Arc<Queue>,
    base_url: String,
    cache: DataCache,
    active_handles: HashMap<Uuid, GpuBufferSet>,
}

impl DataManager {
    /// Create a new data manager
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, base_url: String) -> Self {
        Self {
            device,
            queue,
            base_url,
            cache: DataCache::new(100 * 1024 * 1024), // 100MB default cache
            active_handles: HashMap::new(),
        }
    }

    /// Fetch data and create GPU buffers
    pub async fn fetch_data(
        &mut self,
        symbol: &str,
        start_time: u64,
        end_time: u64,
        columns: &[&str],
    ) -> GpuChartsResult<DataHandle> {
        // Check cache first
        let cache_key = format!("{}-{}-{}-{:?}", symbol, start_time, end_time, columns);
        if let Some(handle) = self.cache.get(&cache_key) {
            return Ok(handle);
        }

        // Build the API URL
        let columns_str = columns.join(",");
        let url = format!(
            "{}/api/data?symbol={}&type=MD&start={}&end={}&columns={}",
            self.base_url, symbol, start_time, end_time, columns_str
        );

        // Fetch from server
        let (api_header, binary_buffer) = fetch_api_response(&url).await
            .map_err(|e| GpuChartsError::DataFetch { 
                message: format!("{:?} (URL: {})", e, url)
            })?;

        // Parse the binary data into columnar format
        let mut column_buffers = HashMap::new();
        let mut offset = 0u32;

        for column in &api_header.columns {
            let data_length = column.data_length as u32;
            let start = offset;
            let end = offset + data_length;
            offset = end;

            let col_buffer = binary_buffer.slice_with_end(start, end);
            let gpu_buffers = create_chunked_gpu_buffer_from_arraybuffer(
                &self.device,
                &col_buffer,
                &column.name,
            );
            column_buffers.insert(column.name.clone(), gpu_buffers);
        }

        // Create GPU buffer set
        let gpu_buffers = GpuBufferSet {
            buffers: column_buffers,
            metadata: DataMetadata {
                symbol: symbol.to_string(),
                start_time,
                end_time,
                columns: api_header.columns.iter().map(|c| c.name.clone()).collect(),
                row_count: api_header.columns.iter().map(|c| c.data_length / 4).max().unwrap_or(0),
            },
        };

        // Create handle
        let handle = DataHandle {
            id: Uuid::new_v4(),
            metadata: gpu_buffers.metadata.clone(),
        };

        // Store in cache and active handles
        self.cache.insert(cache_key, handle.clone());
        self.active_handles.insert(handle.id, gpu_buffers);

        Ok(handle)
    }


    /// Get GPU buffers for a data handle
    pub fn get_buffers(&self, handle: &DataHandle) -> Option<&GpuBufferSet> {
        self.active_handles.get(&handle.id)
    }

    /// Create GPU buffers from parsed data
    fn create_gpu_buffers(&self, data: &ParsedData) -> GpuChartsResult<GpuBufferSet> {
        let mut buffers = HashMap::new();

        // Create time buffer
        if !data.time_data.is_empty() {
            let time_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Time Data Buffer"),
                contents: bytemuck::cast_slice(&data.time_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            buffers.insert("time".to_string(), vec![time_buffer]);
        }

        // Create value buffers
        for (column, values) in &data.value_data {
            if !values.is_empty() {
                let value_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{} Data Buffer", column)),
                    contents: bytemuck::cast_slice(values),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
                buffers.insert(column.clone(), vec![value_buffer]);
            }
        }

        Ok(GpuBufferSet {
            buffers,
            metadata: data.metadata.clone(),
        })
    }

    /// Update configuration
    pub fn update_cache_size(&mut self, size_bytes: usize) {
        self.cache.resize(size_bytes);
    }

    /// Clear all cached data
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.active_handles.clear();
    }
}

/// LRU cache for data
pub struct DataCache {
    capacity: usize,
    entries: HashMap<String, DataHandle>,
    access_order: Vec<String>,
}

impl DataCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            access_order: Vec::new(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<DataHandle> {
        if let Some(handle) = self.entries.get(key) {
            // Update access order
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.to_string());
            Some(handle.clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, handle: DataHandle) {
        // Remove if already exists
        if self.entries.contains_key(&key) {
            self.access_order.retain(|k| k != &key);
        }

        // Evict if at capacity
        while self.entries.len() >= self.capacity && !self.access_order.is_empty() {
            if let Some(oldest) = self.access_order.first() {
                let oldest = oldest.clone();
                self.entries.remove(&oldest);
                self.access_order.remove(0);
            }
        }

        // Insert new entry
        self.entries.insert(key.clone(), handle);
        self.access_order.push(key);
    }

    pub fn resize(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        // Evict entries if necessary
        while self.entries.len() > self.capacity && !self.access_order.is_empty() {
            if let Some(oldest) = self.access_order.first() {
                let oldest = oldest.clone();
                self.entries.remove(&oldest);
                self.access_order.remove(0);
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.access_order.clear();
    }
}

// Re-export for convenience
pub use wgpu::util::BufferInitDescriptor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_lru() {
        let mut cache = DataCache::new(2);
        
        let handle1 = DataHandle {
            id: Uuid::new_v4(),
            metadata: DataMetadata {
                symbol: "BTC-USD".to_string(),
                start_time: 1000,
                end_time: 2000,
                columns: vec!["time".to_string(), "price".to_string()],
                row_count: 100,
            },
        };

        let handle2 = DataHandle {
            id: Uuid::new_v4(),
            metadata: DataMetadata {
                symbol: "ETH-USD".to_string(),
                start_time: 1000,
                end_time: 2000,
                columns: vec!["time".to_string(), "price".to_string()],
                row_count: 100,
            },
        };

        let handle3 = DataHandle {
            id: Uuid::new_v4(),
            metadata: DataMetadata {
                symbol: "SOL-USD".to_string(),
                start_time: 1000,
                end_time: 2000,
                columns: vec!["time".to_string(), "price".to_string()],
                row_count: 100,
            },
        };

        cache.insert("btc".to_string(), handle1.clone());
        cache.insert("eth".to_string(), handle2.clone());
        
        // Cache is full, inserting another should evict btc
        cache.insert("sol".to_string(), handle3.clone());
        
        assert!(cache.get("btc").is_none());
        assert!(cache.get("eth").is_some());
        assert!(cache.get("sol").is_some());
    }
}