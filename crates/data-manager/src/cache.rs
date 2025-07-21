//! LRU cache for GPU data with intelligent eviction

use gpu_charts_shared::{DataHandle, DataRequest, TimeRange};
use lru::LruCache;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

/// Cache key for data requests
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    symbol: String,
    time_range: TimeRange,
    columns_hash: u64,
    aggregation: Option<(String, u32)>,
}

impl CacheKey {
    pub fn from_request(request: &DataRequest) -> Self {
        use std::collections::hash_map::DefaultHasher;

        // Hash the columns to handle order differences
        let mut hasher = DefaultHasher::new();
        let mut sorted_columns = request.columns.clone();
        sorted_columns.sort();
        for col in &sorted_columns {
            col.hash(&mut hasher);
        }
        let columns_hash = hasher.finish();

        Self {
            symbol: request.symbol.clone(),
            time_range: request.time_range,
            columns_hash,
            aggregation: request
                .aggregation
                .as_ref()
                .map(|a| (format!("{:?}", a.aggregation_type), a.timeframe)),
        }
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            self.symbol, self.time_range.start, self.time_range.end, self.columns_hash
        )
    }
}

impl fmt::Debug for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

/// High-performance data cache with LRU eviction
pub struct DataCache {
    cache: LruCache<CacheKey, DataHandle>,
    total_size: u64,
    max_size: u64,
    hit_count: u64,
    miss_count: u64,
}

impl DataCache {
    pub fn new(max_size: u64) -> Self {
        let capacity = NonZeroUsize::new(1000).unwrap(); // Max 1000 entries
        Self {
            cache: LruCache::new(capacity),
            total_size: 0,
            max_size,
            hit_count: 0,
            miss_count: 0,
        }
    }

    pub fn get(&mut self, key: &CacheKey) -> Option<&DataHandle> {
        if let Some(handle) = self.cache.get(key) {
            self.hit_count += 1;
            Some(handle)
        } else {
            self.miss_count += 1;
            None
        }
    }

    pub fn insert(&mut self, key: CacheKey, handle: DataHandle) {
        let size = handle.metadata.byte_size;

        // Evict if needed
        while self.total_size + size > self.max_size && self.cache.len() > 0 {
            if let Some((_, evicted)) = self.cache.pop_lru() {
                self.total_size -= evicted.metadata.byte_size;
            }
        }

        self.total_size += size;
        self.cache.put(key, handle);
    }

    pub fn get_stats(&self) -> serde_json::Value {
        let hit_rate = if self.hit_count + self.miss_count > 0 {
            self.hit_count as f64 / (self.hit_count + self.miss_count) as f64
        } else {
            0.0
        };

        serde_json::json!({
            "entries": self.cache.len(),
            "total_size_mb": self.total_size as f64 / (1024.0 * 1024.0),
            "max_size_mb": self.max_size as f64 / (1024.0 * 1024.0),
            "hit_count": self.hit_count,
            "miss_count": self.miss_count,
            "hit_rate": hit_rate,
        })
    }

    /// Check if a time range can be served from cache
    pub fn can_serve_range(&self, symbol: &str, requested: TimeRange) -> Option<Vec<&DataHandle>> {
        // TODO: Implement intelligent range checking
        // This would check if we have overlapping cached ranges that
        // can serve the requested range without fetching
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpu_charts_shared::DataMetadata;

    #[test]
    fn test_cache_eviction() {
        let mut cache = DataCache::new(1024 * 1024); // 1MB cache

        // Add items that exceed cache size
        for i in 0..10 {
            let key = CacheKey {
                symbol: format!("TEST{}", i),
                time_range: TimeRange::new(0, 1000),
                columns_hash: i,
                aggregation: None,
            };

            let handle = DataHandle {
                id: uuid::Uuid::new_v4(),
                metadata: DataMetadata {
                    symbol: format!("TEST{}", i),
                    time_range: TimeRange::new(0, 1000),
                    columns: vec![],
                    row_count: 1000,
                    byte_size: 200 * 1024, // 200KB each
                    creation_time: 0,
                },
            };

            cache.insert(key, handle);
        }

        // Should have evicted oldest entries
        assert!(cache.cache.len() <= 5);
        assert!(cache.total_size <= cache.max_size);
    }
}
