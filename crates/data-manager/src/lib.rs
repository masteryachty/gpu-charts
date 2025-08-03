//! Data Manager crate for GPU Charts
//! Handles all data operations with focus on performance and GPU optimization

pub mod binary_parser;
pub mod data_retriever;
pub mod data_store;

use shared_types::{DataHandle, DataMetadata, GpuChartsError, GpuChartsResult};
use std::collections::HashMap;
use std::rc::Rc;
use uuid::Uuid;
use wgpu::{Device, Queue};

pub use data_retriever::{
    create_chunked_gpu_buffer_from_arraybuffer, create_gpu_buffer_from_vec, fetch_api_response,
    ApiHeader, ColumnMeta,
};
pub use data_store::{
    ChartType, DataSeries, DataStore, MetricRef, MetricSeries, ScreenDimensions, Vertex,
};

/// GPU buffer set for storing data
/// This is internal to data-manager and contains GPU resources
pub struct GpuBufferSet {
    pub buffers: HashMap<String, Vec<wgpu::Buffer>>,
    pub raw_buffers: HashMap<String, js_sys::ArrayBuffer>, // Store raw data for DataStore
    pub metadata: DataMetadata,
    pub data_type: String, // Track the data type (e.g., "MD", "trades")
}

/// Multi-type data handle containing multiple data handles
#[derive(Debug, Clone)]
pub struct MultiDataHandle {
    pub handles: HashMap<String, DataHandle>, // key is data type (e.g., "MD", "trades")
    pub symbol: String,
    pub start_time: u64,
    pub end_time: u64,
}

/// Main data manager that coordinates all data operations
pub struct DataManager {
    device: Rc<Device>,
    base_url: String,
    cache: DataCache,
    active_handles: HashMap<Uuid, GpuBufferSet>,
}

impl DataManager {
    /// Create a new data manager
    pub fn new(device: Rc<Device>, _queue: Rc<Queue>, base_url: String) -> Self {
        Self {
            device,
            base_url,
            cache: DataCache::new(100 * 1024 * 1024), // 100MB default cache
            active_handles: HashMap::new(),
        }
    }

    /// Fetch data and create GPU buffers
    pub async fn fetch_data(
        &mut self,
        symbol: &str,
        data_type: &str,
        start_time: u32,
        end_time: u32,
        columns: &[&str],
    ) -> GpuChartsResult<DataHandle> {
        // Check cache first
        let cache_key = format!("{symbol}-{data_type}-{start_time}-{end_time}-{columns:?}");
        if let Some(handle) = self.cache.get(&cache_key) {
            return Ok(handle);
        }

        // Build the API URL with proper encoding
        let columns_str = columns.join(",");
        let encoded_symbol = urlencoding::encode(symbol);
        let encoded_columns = urlencoding::encode(&columns_str);

        let url = format!(
            "{}/api/data?symbol={}&type={}&start={}&end={}&columns={}&exchange=coinbase",
            self.base_url, encoded_symbol, data_type, start_time, end_time, encoded_columns
        );

        // Fetch from server
        let (api_header, binary_buffer) =
            fetch_api_response(&url)
                .await
                .map_err(|e| GpuChartsError::DataFetch {
                    message: format!("{e:?} (URL: {url})"),
                })?;

        // Parse the binary data into columnar format
        let mut column_buffers = HashMap::new();
        let mut raw_buffers = HashMap::new();
        let mut offset = 0u32;

        for column in &api_header.columns {
            let data_length = column.data_length as u32;
            let start = offset;
            let end = offset + data_length;
            offset = end;

            let col_buffer = binary_buffer.slice_with_end(start, end);
            let gpu_buffers =
                create_chunked_gpu_buffer_from_arraybuffer(&self.device, &col_buffer, &column.name);
            column_buffers.insert(column.name.clone(), gpu_buffers);
            raw_buffers.insert(column.name.clone(), col_buffer);
        }

        // Create GPU buffer set
        let gpu_buffers = GpuBufferSet {
            buffers: column_buffers,
            raw_buffers,
            metadata: DataMetadata {
                symbol: symbol.to_string(),
                start_time,
                end_time,
                columns: api_header.columns.iter().map(|c| c.name.clone()).collect(),
                row_count: api_header
                    .columns
                    .iter()
                    .map(|c| c.data_length / 4)
                    .max()
                    .unwrap_or(0),
            },
            data_type: data_type.to_string(),
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

    /// Update configuration
    pub fn update_cache_size(&mut self, size_bytes: usize) {
        self.cache.resize(size_bytes);
    }

    /// Clear all cached data
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.active_handles.clear();
    }

    pub async fn fetch_data_for_preset(
        &mut self,
        data_store: &mut DataStore,
    ) -> Result<(), shared_types::GpuChartsError> {
        let symbol = data_store.symbol.as_ref().unwrap().to_string();
        let preset = data_store.preset.clone().unwrap();
        let start_time = data_store.start_x;
        let end_time = data_store.end_x;

        let mut data_requirements: std::collections::HashMap<
            String,
            std::collections::HashSet<String>,
        > = std::collections::HashMap::new();
        for chart_type in &preset.chart_types {
            for (data_type, column) in &chart_type.data_columns {
                data_requirements
                    .entry(data_type.clone())
                    .or_default()
                    .insert(column.clone());
            }
            if let Some(additional_cols) = &chart_type.additional_data_columns {
                for (data_type, column) in additional_cols {
                    data_requirements
                        .entry(data_type.clone())
                        .or_default()
                        .insert(column.clone());
                }
            }
        }

        // Removed unused fetch_results variable
        for (data_type, columns) in data_requirements {
            let mut all_columns = vec!["time"];
            let columns_vec: Vec<String> = columns.into_iter().collect();
            all_columns.extend(columns_vec.iter().map(|s| s.as_str()));

            // let instance_opt = InstanceManager::take_instance(&instance_id);
            let result = self
                .fetch_data(
                    &symbol,
                    data_type.as_str(),
                    start_time,
                    end_time,
                    &all_columns,
                )
                .await;
            match result {
                Ok(data_handle) => {
                    let _ = self.process_data_handle(&data_handle, data_store);
                }
                Err(_e) => {}
            }
        }

        // After loading all data, create computed metrics if needed
        self.create_computed_metrics_for_preset(data_store);

        Ok(())
    }

    fn process_data_handle(
        &self,
        data_handle: &DataHandle,
        data_store: &mut DataStore,
    ) -> Result<(), shared_types::GpuChartsError> {
        // Get the GPU buffer set from the data manager
        let gpu_buffer_set = self.get_buffers(data_handle).ok_or_else(|| {
            shared_types::GpuChartsError::DataNotFound {
                resource: "GPU buffers for data handle".to_string(),
            }
        })?;

        // Extract the time column (shared x-axis for all metrics)
        let time_buffer = gpu_buffer_set.raw_buffers.get("time").ok_or_else(|| {
            shared_types::GpuChartsError::DataNotFound {
                resource: "Time column in data".to_string(),
            }
        })?;

        let time_gpu_buffers = gpu_buffer_set.buffers.get("time").ok_or_else(|| {
            shared_types::GpuChartsError::DataNotFound {
                resource: "Time GPU buffers".to_string(),
            }
        })?;

        // Add a new data group for this data type
        // Each data type has its own time series, so needs its own group
        data_store.add_data_group((time_buffer.clone(), time_gpu_buffers.clone()), true);
        let data_group_index = data_store.data_groups.len() - 1;

        for column_name in &gpu_buffer_set.metadata.columns {
            if column_name == "time" {
                continue; // Skip time column as it's already the x-axis
            }

            if let (Some(raw_buffer), Some(gpu_buffers)) = (
                gpu_buffer_set.raw_buffers.get(column_name),
                gpu_buffer_set.buffers.get(column_name),
            ) {
                // Get color from preset if available
                let color = if let Some(preset) = &data_store.preset {
                    // Find the color for this metric in the preset
                    preset
                        .chart_types
                        .iter()
                        .find(|chart_type| {
                            chart_type
                                .data_columns
                                .iter()
                                .any(|(_, col)| col == column_name)
                                || chart_type
                                    .additional_data_columns
                                    .as_ref()
                                    .map(|cols| cols.iter().any(|(_, col)| col == column_name))
                                    .unwrap_or(false)
                        })
                        .and_then(|chart_type| chart_type.style.color)
                        .map(|c| [c[0], c[1], c[2]])
                        .unwrap_or([0.0, 0.5, 1.0])
                } else {
                    [0.0, 0.5, 1.0] // Default blue
                };

                data_store.add_metric_to_group(
                    data_group_index,
                    (raw_buffer.clone(), gpu_buffers.clone()),
                    color,
                    column_name.clone(),
                );
            }
        }

        Ok(())
    }

    fn create_computed_metrics_for_preset(&self, data_store: &mut DataStore) {
        if let Some(preset) = &data_store.preset.clone() {
            // Check each chart type for compute operations
            for chart_type in &preset.chart_types {
                if let Some(compute_op) = &chart_type.compute_op {
                    // Find dependencies based on additional_data_columns for computed metrics
                    let mut dependencies = Vec::new();

                    // Use additional_data_columns if present (for computed metrics), otherwise fall back to data_columns
                    let dependency_columns = chart_type
                        .additional_data_columns
                        .as_ref()
                        .unwrap_or(&chart_type.data_columns);

                    // Since we don't have named groups, we need to find metrics by name
                    // across all data groups
                    for (_, column_name) in dependency_columns {
                        for (group_idx, group) in data_store.data_groups.iter().enumerate() {
                            for (metric_idx, metric) in group.metrics.iter().enumerate() {
                                if metric.name == *column_name {
                                    dependencies.push(MetricRef {
                                        group_index: group_idx,
                                        metric_index: metric_idx,
                                    });
                                    break; // Found the metric, move to next column
                                }
                            }
                        }
                    }

                    // If we have all dependencies, create the computed metric
                    if dependencies.len() == dependency_columns.len() && !dependencies.is_empty() {
                        log::debug!(
                            "[DataManager] Creating computed metric '{}' with {} dependencies",
                            chart_type.label,
                            dependencies.len()
                        );

                        // Add the computed metric to the first data group (or create one if needed)
                        let group_index = if data_store.data_groups.is_empty() {
                            // This shouldn't happen, but handle it gracefully
                            log::warn!(
                                "[DataManager] No data groups available for computed metric"
                            );
                            return;
                        } else {
                            0 // Use the first data group
                        };

                        // Get color from chart type
                        let color = chart_type.style.color.unwrap_or([0.5, 0.5, 0.5, 1.0]);
                        data_store.add_computed_metric_to_group(
                            group_index,
                            chart_type.label.clone(),
                            [color[0], color[1], color[2]],
                            compute_op.clone(),
                            dependencies,
                        );

                        log::debug!(
                            "[DataManager] Added computed metric '{}' to group {}",
                            chart_type.label,
                            group_index
                        );
                    } else {
                        log::debug!(
                            "[DataManager] Could not create computed metric '{}': found {} of {} dependencies",
                            chart_type.label,
                            dependencies.len(),
                            chart_type.data_columns.len()
                        );
                    }
                }
            }
        }
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
