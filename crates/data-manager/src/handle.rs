//! Handle-based API for zero-copy buffer management
//!
//! This module provides a handle system that allows efficient sharing and tracking
//! of GPU buffers without copying data. Handles are lightweight references that
//! track buffer lifecycle and ownership.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Weak};
use uuid::Uuid;

/// A lightweight handle to a GPU buffer
#[derive(Clone, Debug)]
pub struct BufferHandle {
    /// Unique identifier for this handle
    id: Uuid,
    /// Weak reference to the actual buffer data
    buffer: Weak<BufferData>,
    /// Generation number to detect stale handles
    generation: u64,
}

impl BufferHandle {
    /// Check if the handle is still valid
    pub fn is_valid(&self) -> bool {
        self.buffer.strong_count() > 0
    }

    /// Try to access the buffer data
    pub fn access(&self) -> Option<Arc<BufferData>> {
        self.buffer.upgrade()
    }

    /// Get the handle ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the generation number
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// Actual buffer data with reference counting
pub struct BufferData {
    /// The GPU buffer
    pub buffer: wgpu::Buffer,
    /// Size in bytes
    pub size: u64,
    /// Usage flags
    pub usage: wgpu::BufferUsages,
    /// Reference count for tracking
    ref_count: AtomicUsize,
    /// Creation timestamp
    pub created_at: std::time::Instant,
    /// Last access timestamp
    last_access: AtomicU64,
    /// Metadata for cache management
    pub metadata: BufferMetadata,
}

impl BufferData {
    /// Create new buffer data
    pub fn new(
        buffer: wgpu::Buffer,
        size: u64,
        usage: wgpu::BufferUsages,
        metadata: BufferMetadata,
    ) -> Self {
        Self {
            buffer,
            size,
            usage,
            ref_count: AtomicUsize::new(1),
            created_at: std::time::Instant::now(),
            last_access: AtomicU64::new(0),
            metadata,
        }
    }

    /// Update last access time
    pub fn touch(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_access.store(now, Ordering::Relaxed);
    }

    /// Get last access time
    pub fn last_access_time(&self) -> u64 {
        self.last_access.load(Ordering::Relaxed)
    }

    /// Increment reference count
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Decrement reference count
    pub fn dec_ref(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::AcqRel) - 1
    }

    /// Get current reference count
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::Acquire)
    }
}

/// Metadata for buffer management
#[derive(Clone, Debug)]
pub struct BufferMetadata {
    /// Data type (e.g., "time_series", "ohlc", "volume")
    pub data_type: String,
    /// Symbol or identifier
    pub symbol: String,
    /// Time range if applicable
    pub time_range: Option<(u64, u64)>,
    /// Column name if applicable
    pub column: Option<String>,
    /// Compression type if any
    pub compression: Option<CompressionType>,
    /// Custom tags for flexible categorization
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompressionType {
    None,
    Gzip,
    Brotli,
    Custom(String),
}

/// Handle manager for creating and tracking handles
pub struct HandleManager {
    /// Map of handle ID to buffer data
    buffers: Arc<parking_lot::RwLock<HashMap<Uuid, Arc<BufferData>>>>,
    /// Generation counter for detecting stale handles
    generation: AtomicU64,
    /// Statistics
    stats: HandleStats,
}

impl HandleManager {
    /// Create a new handle manager
    pub fn new() -> Self {
        Self {
            buffers: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            generation: AtomicU64::new(0),
            stats: HandleStats::new(),
        }
    }

    /// Create a new handle for a buffer
    pub fn create_handle(&self, buffer_data: BufferData) -> BufferHandle {
        let id = Uuid::new_v4();
        let generation = self.generation.fetch_add(1, Ordering::AcqRel);
        let buffer_arc = Arc::new(buffer_data);

        // Store in the map
        {
            let mut buffers = self.buffers.write();
            buffers.insert(id, buffer_arc.clone());
        }

        // Update stats
        self.stats.handles_created.fetch_add(1, Ordering::Relaxed);
        self.stats
            .total_memory
            .fetch_add(buffer_arc.size, Ordering::Relaxed);

        BufferHandle {
            id,
            buffer: Arc::downgrade(&buffer_arc),
            generation,
        }
    }

    /// Transfer ownership of a handle
    pub fn transfer_handle(&self, handle: &BufferHandle) -> Option<BufferHandle> {
        if let Some(buffer) = handle.access() {
            buffer.inc_ref();
            self.stats
                .handles_transferred
                .fetch_add(1, Ordering::Relaxed);

            Some(BufferHandle {
                id: Uuid::new_v4(), // New ID for the transferred handle
                buffer: Arc::downgrade(&buffer),
                generation: self.generation.fetch_add(1, Ordering::AcqRel),
            })
        } else {
            None
        }
    }

    /// Release a handle
    pub fn release_handle(&self, handle: BufferHandle) {
        if let Some(buffer) = handle.access() {
            let remaining = buffer.dec_ref();

            if remaining == 0 {
                // Remove from map when no more references
                let mut buffers = self.buffers.write();
                if let Some(removed) = buffers.remove(&handle.id) {
                    self.stats
                        .total_memory
                        .fetch_sub(removed.size, Ordering::Relaxed);
                    self.stats.handles_released.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// Clean up stale handles
    pub fn cleanup_stale_handles(&self) -> usize {
        let mut buffers = self.buffers.write();
        let before = buffers.len();

        buffers.retain(|_, buffer| {
            let should_keep = buffer.ref_count() > 0;
            if !should_keep {
                self.stats
                    .total_memory
                    .fetch_sub(buffer.size, Ordering::Relaxed);
            }
            should_keep
        });

        let removed = before - buffers.len();
        self.stats
            .stale_handles_cleaned
            .fetch_add(removed, Ordering::Relaxed);
        removed
    }

    /// Get handle statistics
    pub fn stats(&self) -> HandleStatsSnapshot {
        self.stats.snapshot()
    }

    /// Get total active handles
    pub fn active_handles(&self) -> usize {
        self.buffers.read().len()
    }

    /// Get total memory usage
    pub fn total_memory(&self) -> u64 {
        self.stats.total_memory.load(Ordering::Relaxed)
    }
}

/// Statistics tracking for handle operations
struct HandleStats {
    handles_created: AtomicU64,
    handles_transferred: AtomicU64,
    handles_released: AtomicU64,
    stale_handles_cleaned: AtomicU64,
    total_memory: AtomicU64,
}

impl HandleStats {
    fn new() -> Self {
        Self {
            handles_created: AtomicU64::new(0),
            handles_transferred: AtomicU64::new(0),
            handles_released: AtomicU64::new(0),
            stale_handles_cleaned: AtomicU64::new(0),
            total_memory: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> HandleStatsSnapshot {
        HandleStatsSnapshot {
            handles_created: self.handles_created.load(Ordering::Relaxed),
            handles_transferred: self.handles_transferred.load(Ordering::Relaxed),
            handles_released: self.handles_released.load(Ordering::Relaxed),
            stale_handles_cleaned: self.stale_handles_cleaned.load(Ordering::Relaxed),
            total_memory_bytes: self.total_memory.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HandleStatsSnapshot {
    pub handles_created: u64,
    pub handles_transferred: u64,
    pub handles_released: u64,
    pub stale_handles_cleaned: u64,
    pub total_memory_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Test Buffer"),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    #[test]
    fn test_handle_creation() {
        // This would require a GPU context in real tests
        // For now, we test the logic without actual GPU buffers
        let manager = HandleManager::new();
        assert_eq!(manager.active_handles(), 0);
        assert_eq!(manager.total_memory(), 0);
    }

    #[test]
    fn test_handle_transfer() {
        let manager = HandleManager::new();
        let stats = manager.stats();
        assert_eq!(stats.handles_created, 0);
        assert_eq!(stats.handles_transferred, 0);
    }

    #[test]
    fn test_metadata() {
        let metadata = BufferMetadata {
            data_type: "time_series".to_string(),
            symbol: "BTC-USD".to_string(),
            time_range: Some((1000, 2000)),
            column: Some("price".to_string()),
            compression: Some(CompressionType::Gzip),
            tags: HashMap::new(),
        };

        assert_eq!(metadata.data_type, "time_series");
        assert_eq!(metadata.symbol, "BTC-USD");
        assert_eq!(metadata.compression, Some(CompressionType::Gzip));
    }
}
