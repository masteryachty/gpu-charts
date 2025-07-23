//! Core DataManager implementation with zero-copy architecture
//!
//! The DataManager is responsible for:
//! - Managing GPU buffer lifecycle
//! - Implementing LRU caching with memory limits
//! - Coordinating data loading and parsing
//! - Providing handle-based access to buffers

use crate::cache::CacheKey;
use lru::LruCache;
use crate::direct_gpu_parser::DirectGpuParser;
use crate::handle::{BufferData, BufferHandle, BufferMetadata, HandleManager};
use gpu_charts_shared::{Error, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use wgpu::BufferUsages;

/// Configuration for the DataManager
#[derive(Clone)]
pub struct DataManagerConfig {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Cache time-to-live in seconds
    pub cache_ttl_seconds: u64,
    /// Enable prefetching
    pub enable_prefetching: bool,
    /// Enable speculative caching
    pub enable_speculative_caching: bool,
    /// Buffer pool reference
    pub buffer_pool: Option<Arc<gpu_charts_renderer::buffer_pool::RenderBufferPool>>,
}

impl std::fmt::Debug for DataManagerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataManagerConfig")
            .field("max_memory_bytes", &self.max_memory_bytes)
            .field("cache_ttl_seconds", &self.cache_ttl_seconds)
            .field("enable_prefetching", &self.enable_prefetching)
            .field("enable_speculative_caching", &self.enable_speculative_caching)
            .field("buffer_pool", &self.buffer_pool.is_some())
            .finish()
    }
}

impl Default for DataManagerConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            cache_ttl_seconds: 3600,                  // 1 hour
            enable_prefetching: true,
            enable_speculative_caching: true,
            buffer_pool: None,
        }
    }
}

/// The main DataManager struct
pub struct DataManager {
    /// Handle manager for buffer lifecycle
    handle_manager: Arc<HandleManager>,
    /// LRU cache for loaded data
    cache: Arc<RwLock<LruCache<CacheKey, BufferHandle>>>,
    /// Direct GPU parser
    parser: Arc<DirectGpuParser>,
    /// Configuration
    config: DataManagerConfig,
    /// Statistics
    stats: Arc<RwLock<DataManagerStats>>,
    /// Device reference
    device: Arc<wgpu::Device>,
    /// Queue reference
    queue: Arc<wgpu::Queue>,
}

impl DataManager {
    /// Create a new DataManager
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: DataManagerConfig,
    ) -> Self {
        let handle_manager = Arc::new(HandleManager::new());
        // LruCache expects NonZeroUsize
        let cache_size = std::num::NonZeroUsize::new(1000).unwrap(); // 1000 entries
        let cache = Arc::new(RwLock::new(LruCache::new(cache_size)));
        let parser = Arc::new(DirectGpuParser::new(device.clone(), queue.clone()));

        Self {
            handle_manager,
            cache,
            parser,
            config,
            stats: Arc::new(RwLock::new(DataManagerStats::default())),
            device,
            queue,
        }
    }

    /// Load data and return a handle
    pub async fn load_data(
        &self,
        source: DataSource,
        metadata: BufferMetadata,
    ) -> Result<BufferHandle> {
        // Generate cache key
        let cache_key = CacheKey::from_metadata(&metadata);

        // Check cache first
        if let Some(handle) = self.get_from_cache(&cache_key) {
            self.stats.write().cache_hits += 1;
            return Ok(handle);
        }

        self.stats.write().cache_misses += 1;

        // Load data based on source
        let buffer_data = match source {
            DataSource::File(path) => {
                #[cfg(feature = "native")]
                {
                    self.load_from_file(path, metadata.clone()).await?
                }
                #[cfg(not(feature = "native"))]
                {
                    return Err(Error::GpuError("File access not available in WASM".to_string()));
                }
            }
            DataSource::Url(url) => self.load_from_url(url, metadata.clone()).await?,
            DataSource::Memory(data) => self.create_from_memory(data, metadata.clone())?,
        };

        // Create handle
        let handle = self.handle_manager.create_handle(buffer_data);

        // Add to cache
        self.add_to_cache(cache_key, handle.clone());

        // Trigger prefetching if enabled
        if self.config.enable_prefetching {
            self.trigger_prefetch(&metadata);
        }

        Ok(handle)
    }

    /// Load data from file
    #[cfg(feature = "native")]
    async fn load_from_file(
        &self,
        path: std::path::PathBuf,
        metadata: BufferMetadata,
    ) -> Result<BufferData> {
        let start = std::time::Instant::now();

        // Use direct GPU parser with buffer pool if available
        let buffer = if let Some(_pool) = &self.config.buffer_pool {
            // For WASM, we don't have file access, so skip file parsing
            #[cfg(feature = "native")]
            {
                // TODO: Fix buffer pool integration
                return Err(Error::GpuError("File parsing with buffer pool not yet implemented".to_string()));
            }
            #[cfg(not(feature = "native"))]
            {
                return Err(Error::GpuError("File access not available in WASM".to_string()));
            }
        } else {
            // Fallback to creating new buffer
            let file_size = std::fs::metadata(&path)?.len();
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Data: {}", metadata.symbol)),
                size: file_size,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::STORAGE,
                mapped_at_creation: false,
            });

            // Load and copy data
            let data = std::fs::read(&path)?;
            self.queue.write_buffer(&buffer, 0, &data);

            buffer
        };

        let elapsed = start.elapsed();
        self.stats.write().total_load_time_ms += elapsed.as_millis() as u64;

        Ok(BufferData::new(
            buffer,
            std::fs::metadata(&path)?.len(),
            BufferUsages::VERTEX | BufferUsages::STORAGE,
            metadata,
        ))
    }

    /// Load data from URL
    async fn load_from_url(&self, url: String, metadata: BufferMetadata) -> Result<BufferData> {
        // TODO: Implement HTTP/2 client with compression support
        // For now, use a simple implementation
        let response = reqwest::get(&url)
            .await
            .map_err(|e| Error::NetworkError(e.to_string()))?;

        let data = response
            .bytes()
            .await
            .map_err(|e| Error::NetworkError(e.to_string()))?;

        self.create_from_memory(data.to_vec(), metadata)
    }

    /// Create buffer from memory
    fn create_from_memory(&self, data: Vec<u8>, metadata: BufferMetadata) -> Result<BufferData> {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Data: {}", metadata.symbol)),
            size: data.len() as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        self.queue.write_buffer(&buffer, 0, &data);

        Ok(BufferData::new(
            buffer,
            data.len() as u64,
            BufferUsages::VERTEX | BufferUsages::STORAGE,
            metadata,
        ))
    }

    /// Get handle from cache
    fn get_from_cache(&self, key: &CacheKey) -> Option<BufferHandle> {
        let mut cache = self.cache.write();
        cache.get(key).cloned()
    }

    /// Add handle to cache
    fn add_to_cache(&self, key: CacheKey, handle: BufferHandle) {
        let mut cache = self.cache.write();

        // LruCache in v0.12 has different API - just use put which auto-evicts
        cache.put(key, handle);
    }

    /// Trigger prefetching for related data
    fn trigger_prefetch(&self, _metadata: &BufferMetadata) {
        // Prefetching disabled for WASM - would need to be reimplemented
        // without tokio::spawn for browser compatibility
    }

    /// Get current statistics
    pub fn stats(&self) -> DataManagerStats {
        self.stats.read().clone()
    }

    /// Clean up stale handles and expired cache entries
    pub fn cleanup(&self) -> CleanupResult {
        let handles_cleaned = self.handle_manager.cleanup_stale_handles();
        // LruCache v0.12 doesn't have cleanup_expired - entries are auto-evicted
        let cache_entries_expired = 0;

        CleanupResult {
            handles_cleaned,
            cache_entries_expired,
        }
    }

    /// Get total memory usage
    pub fn memory_usage(&self) -> MemoryUsage {
        let handle_memory = self.handle_manager.total_memory();
        // LruCache v0.12 doesn't expose total_memory - estimate based on entry count
        let cache_len = self.cache.read().len() as u64;
        let estimated_cache_memory = cache_len * 1024 * 1024; // Rough estimate: 1MB per entry
        
        MemoryUsage {
            handle_memory,
            cache_memory: estimated_cache_memory,
            total_memory: handle_memory + estimated_cache_memory,
        }
    }

    /// Warm the cache with frequently accessed data
    pub async fn warm_cache(&self, patterns: Vec<CacheWarmPattern>) -> Result<()> {
        for pattern in patterns {
            match pattern {
                CacheWarmPattern::TimeSeriesRange {
                    symbol,
                    start,
                    end,
                    columns,
                } => {
                    for column in columns {
                        let metadata = BufferMetadata {
                            data_type: "time_series".to_string(),
                            symbol: symbol.clone(),
                            time_range: Some((start, end)),
                            column: Some(column),
                            compression: None,
                            tags: Default::default(),
                        };

                        // Try to load but don't fail on error
                        let _ = self
                            .load_data(
                                DataSource::File(std::path::PathBuf::from("cache_warm")), // TODO: Implement
                                metadata,
                            )
                            .await;
                    }
                }
                CacheWarmPattern::LatestData {
                    symbol,
                    duration_seconds,
                } => {
                    let end = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let start = end - duration_seconds;

                    let metadata = BufferMetadata {
                        data_type: "time_series".to_string(),
                        symbol,
                        time_range: Some((start, end)),
                        column: None,
                        compression: None,
                        tags: Default::default(),
                    };

                    let _ = self
                        .load_data(
                            DataSource::File(std::path::PathBuf::from("cache_warm")), // TODO: Implement
                            metadata,
                        )
                        .await;
                }
            }
        }

        Ok(())
    }
}

// Clone implementation for Arc sharing
impl Clone for DataManager {
    fn clone(&self) -> Self {
        Self {
            handle_manager: self.handle_manager.clone(),
            cache: self.cache.clone(),
            parser: self.parser.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
            device: self.device.clone(),
            queue: self.queue.clone(),
        }
    }
}

/// Data source types
#[derive(Debug, Clone)]
pub enum DataSource {
    File(std::path::PathBuf),
    Url(String),
    Memory(Vec<u8>),
}

/// Statistics for the DataManager
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DataManagerStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub evictions: u64,
    pub total_load_time_ms: u64,
    pub prefetch_attempts: u64,
    pub prefetch_successes: u64,
}

/// Result of cleanup operation
#[derive(Debug, Clone, serde::Serialize)]
pub struct CleanupResult {
    pub handles_cleaned: usize,
    pub cache_entries_expired: usize,
}

/// Memory usage information
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryUsage {
    pub handle_memory: u64,
    pub cache_memory: u64,
    pub total_memory: u64,
}

/// Patterns for warming the cache
#[derive(Debug, Clone)]
pub enum CacheWarmPattern {
    TimeSeriesRange {
        symbol: String,
        start: u64,
        end: u64,
        columns: Vec<String>,
    },
    LatestData {
        symbol: String,
        duration_seconds: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_data_manager_creation() {
        // This would require GPU context
        // For now, test the structure
        let config = DataManagerConfig::default();
        assert_eq!(config.max_memory_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(config.cache_ttl_seconds, 3600);
        assert!(config.enable_prefetching);
    }

    #[test]
    fn test_data_source() {
        let file_source = DataSource::File(std::path::PathBuf::from("/data/test.bin"));
        let url_source = DataSource::Url("https://example.com/data".to_string());
        let memory_source = DataSource::Memory(vec![1, 2, 3, 4]);

        match file_source {
            DataSource::File(path) => assert_eq!(path.to_str().unwrap(), "/data/test.bin"),
            _ => panic!("Wrong type"),
        }

        match url_source {
            DataSource::Url(url) => assert_eq!(url, "https://example.com/data"),
            _ => panic!("Wrong type"),
        }

        match memory_source {
            DataSource::Memory(data) => assert_eq!(data, vec![1, 2, 3, 4]),
            _ => panic!("Wrong type"),
        }
    }
}
