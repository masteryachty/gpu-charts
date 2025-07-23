//! Advanced caching system for data management

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use shared_types::DataHandle;

/// Cache entry with metadata
#[derive(Clone)]
pub struct CacheEntry {
    pub handle: DataHandle,
    pub size_bytes: usize,
    pub last_accessed: Instant,
    pub access_count: u32,
}

/// Advanced LRU cache with size tracking and TTL
pub struct AdvancedCache {
    capacity_bytes: usize,
    current_size: usize,
    entries: HashMap<String, CacheEntry>,
    ttl: Option<Duration>,
}

impl AdvancedCache {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            capacity_bytes,
            current_size: 0,
            entries: HashMap::new(),
            ttl: None,
        }
    }

    pub fn with_ttl(capacity_bytes: usize, ttl: Duration) -> Self {
        Self {
            capacity_bytes,
            current_size: 0,
            entries: HashMap::new(),
            ttl: Some(ttl),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<DataHandle> {
        // Check if entry exists and is not expired
        if let Some(entry) = self.entries.get_mut(key) {
            if let Some(ttl) = self.ttl {
                if entry.last_accessed.elapsed() > ttl {
                    // Entry expired
                    self.current_size -= entry.size_bytes;
                    self.entries.remove(key);
                    return None;
                }
            }
            
            // Update access metadata
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            Some(entry.handle.clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, handle: DataHandle, size_bytes: usize) {
        // Remove if already exists
        if let Some(existing) = self.entries.remove(&key) {
            self.current_size -= existing.size_bytes;
        }

        // Evict entries if necessary
        while self.current_size + size_bytes > self.capacity_bytes && !self.entries.is_empty() {
            self.evict_lru();
        }

        // Insert new entry
        let entry = CacheEntry {
            handle,
            size_bytes,
            last_accessed: Instant::now(),
            access_count: 1,
        };

        self.current_size += size_bytes;
        self.entries.insert(key, entry);
    }

    fn evict_lru(&mut self) {
        // Find least recently used entry
        if let Some((key, _)) = self.entries
            .iter()
            .min_by_key(|(_, entry)| (entry.last_accessed, entry.access_count))
            .map(|(k, v)| (k.clone(), v.clone()))
        {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_size -= entry.size_bytes;
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
    }

    pub fn resize(&mut self, new_capacity: usize) {
        self.capacity_bytes = new_capacity;
        while self.current_size > self.capacity_bytes && !self.entries.is_empty() {
            self.evict_lru();
        }
    }

    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            capacity_bytes: self.capacity_bytes,
            used_bytes: self.current_size,
            entry_count: self.entries.len(),
            hit_rate: 0.0, // Would need to track hits/misses
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub capacity_bytes: usize,
    pub used_bytes: usize,
    pub entry_count: usize,
    pub hit_rate: f32,
}

/// Thread-safe cache wrapper
pub struct ThreadSafeCache {
    inner: Arc<Mutex<AdvancedCache>>,
}

impl ThreadSafeCache {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AdvancedCache::new(capacity_bytes))),
        }
    }

    pub fn get(&self, key: &str) -> Option<DataHandle> {
        self.inner.lock().unwrap().get(key)
    }

    pub fn insert(&self, key: String, handle: DataHandle, size_bytes: usize) {
        self.inner.lock().unwrap().insert(key, handle, size_bytes);
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    pub fn get_stats(&self) -> CacheStats {
        self.inner.lock().unwrap().get_stats()
    }
}