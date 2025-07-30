//! Resource pooling system for GPU resource management
//!
//! This module provides efficient pooling and reuse of GPU resources
//! including buffers, textures, bind groups, and pipelines.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use wgpu::{Buffer, BufferUsages, Device, Queue, Texture, TextureFormat, TextureUsages};

// Use web-time for WebAssembly compatibility
use web_time::{Instant, Duration};

/// A unique identifier for a resource type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceTypeId {
    /// Vertex buffer for a specific renderer
    VertexBuffer(String),
    /// Index buffer for a specific renderer
    IndexBuffer(String),
    /// Uniform buffer for a specific renderer
    UniformBuffer(String),
    /// Storage buffer for compute operations
    StorageBuffer(String),
    /// Staging buffer for data transfer
    StagingBuffer(String),
    /// Texture resource
    Texture {
        format: TextureFormat,
        label: String,
    },
}

/// Resource descriptor for buffer creation
#[derive(Debug, Clone)]
pub struct BufferDescriptor {
    pub size: u64,
    pub usage: BufferUsages,
    pub mapped_at_creation: bool,
    pub label: Option<String>,
}

/// Resource descriptor for texture creation
#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    pub size: wgpu::Extent3d,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: wgpu::TextureDimension,
    pub format: TextureFormat,
    pub usage: TextureUsages,
    pub label: Option<String>,
}

/// A pooled resource wrapper
#[derive(Debug)]
pub struct PooledResource<T> {
    pub resource: T,
    pub descriptor: ResourceDescriptor,
    pub last_used: Instant,
    pub use_count: u64,
}

/// Resource descriptor enum
#[derive(Debug, Clone)]
pub enum ResourceDescriptor {
    Buffer(BufferDescriptor),
    Texture(TextureDescriptor),
}

/// Buffer pool for efficient buffer reuse
pub struct BufferPool {
    device: Arc<Device>,
    queue: Arc<Queue>,
    available: HashMap<ResourceTypeId, VecDeque<PooledResource<Buffer>>>,
    in_use: HashMap<ResourceTypeId, Vec<PooledResource<Buffer>>>,
    max_pool_size: usize,
    total_allocated: u64,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, max_pool_size: usize) -> Self {
        Self {
            device,
            queue,
            available: HashMap::new(),
            in_use: HashMap::new(),
            max_pool_size,
            total_allocated: 0,
        }
    }

    /// Get or create a buffer from the pool
    pub fn get_or_create(
        &mut self,
        type_id: ResourceTypeId,
        descriptor: BufferDescriptor,
    ) -> Arc<Buffer> {
        // Try to find a suitable buffer in the available pool
        if let Some(available_buffers) = self.available.get_mut(&type_id) {
            // Find a buffer with matching or larger size
            if let Some(pos) = available_buffers.iter().position(|pooled| {
                if let ResourceDescriptor::Buffer(ref desc) = pooled.descriptor {
                    desc.size >= descriptor.size && desc.usage == descriptor.usage
                } else {
                    false
                }
            }) {
                // Remove from available and add to in_use
                let mut pooled = available_buffers.remove(pos).unwrap();
                pooled.last_used = Instant::now();
                pooled.use_count += 1;

                let buffer = Arc::new(pooled.resource);
                self.in_use
                    .entry(type_id.clone())
                    .or_insert_with(Vec::new)
                    .push(PooledResource {
                        resource: (*buffer).clone(),
                        descriptor: pooled.descriptor,
                        last_used: pooled.last_used,
                        use_count: pooled.use_count,
                    });

                log::debug!("Reusing buffer from pool for {:?}", type_id);
                return buffer;
            }
        }

        // Create a new buffer
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: descriptor.label.as_deref(),
            size: descriptor.size,
            usage: descriptor.usage,
            mapped_at_creation: descriptor.mapped_at_creation,
        });

        self.total_allocated += descriptor.size;
        log::debug!(
            "Created new buffer for {:?}, size: {}, total allocated: {}",
            type_id,
            descriptor.size,
            self.total_allocated
        );

        let buffer = Arc::new(buffer);
        self.in_use
            .entry(type_id.clone())
            .or_insert_with(Vec::new)
            .push(PooledResource {
                resource: (*buffer).clone(),
                descriptor: ResourceDescriptor::Buffer(descriptor),
                last_used: Instant::now(),
                use_count: 1,
            });

        buffer
    }

    /// Return a buffer to the pool
    pub fn return_buffer(&mut self, type_id: ResourceTypeId, buffer: Buffer) {
        // Find the buffer in the in_use list
        if let Some(in_use_buffers) = self.in_use.get_mut(&type_id) {
            if let Some(pos) = in_use_buffers.iter().position(|pooled| {
                // Compare buffer IDs (this is a simplification)
                std::ptr::eq(&pooled.resource as *const _, &buffer as *const _)
            }) {
                let pooled = in_use_buffers.remove(pos);

                // Add to available pool
                let available_buffers = self
                    .available
                    .entry(type_id.clone())
                    .or_insert_with(VecDeque::new);

                // Maintain pool size limit
                if available_buffers.len() < self.max_pool_size {
                    available_buffers.push_back(pooled);
                    log::debug!("Returned buffer to pool for {:?}", type_id);
                } else {
                    // Pool is full, let the buffer be dropped
                    if let ResourceDescriptor::Buffer(ref desc) = pooled.descriptor {
                        self.total_allocated -= desc.size;
                    }
                    log::debug!("Pool full, dropping buffer for {:?}", type_id);
                }
            }
        }
    }

    /// Clean up old unused buffers
    pub fn cleanup_unused(&mut self, max_age_secs: u64) {
        let now = Instant::now();
        let max_age = Duration::from_secs(max_age_secs);

        for (type_id, buffers) in self.available.iter_mut() {
            let mut removed_size = 0u64;

            buffers.retain(|pooled| {
                let age = now.duration_since(pooled.last_used);
                if age > max_age {
                    if let ResourceDescriptor::Buffer(ref desc) = pooled.descriptor {
                        removed_size += desc.size;
                    }
                    false
                } else {
                    true
                }
            });

            if removed_size > 0 {
                self.total_allocated -= removed_size;
                log::debug!(
                    "Cleaned up {} bytes of unused buffers for {:?}",
                    removed_size,
                    type_id
                );
            }
        }
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        let mut available_count = 0;
        let mut available_size = 0;
        let mut in_use_count = 0;
        let mut in_use_size = 0;

        for buffers in self.available.values() {
            available_count += buffers.len();
            for pooled in buffers {
                if let ResourceDescriptor::Buffer(ref desc) = pooled.descriptor {
                    available_size += desc.size;
                }
            }
        }

        for buffers in self.in_use.values() {
            in_use_count += buffers.len();
            for pooled in buffers {
                if let ResourceDescriptor::Buffer(ref desc) = pooled.descriptor {
                    in_use_size += desc.size;
                }
            }
        }

        PoolStats {
            available_count,
            available_size,
            in_use_count,
            in_use_size,
            total_allocated: self.total_allocated,
        }
    }
}

/// Texture pool for efficient texture reuse
pub struct TexturePool {
    device: Arc<Device>,
    available: HashMap<ResourceTypeId, VecDeque<PooledResource<Texture>>>,
    in_use: HashMap<ResourceTypeId, Vec<PooledResource<Texture>>>,
    max_pool_size: usize,
}

impl TexturePool {
    /// Create a new texture pool
    pub fn new(device: Arc<Device>, max_pool_size: usize) -> Self {
        Self {
            device,
            available: HashMap::new(),
            in_use: HashMap::new(),
            max_pool_size,
        }
    }

    /// Get or create a texture from the pool
    pub fn get_or_create(
        &mut self,
        type_id: ResourceTypeId,
        descriptor: TextureDescriptor,
    ) -> Arc<Texture> {
        // Try to find a suitable texture in the available pool
        if let Some(available_textures) = self.available.get_mut(&type_id) {
            if let Some(pos) = available_textures.iter().position(|pooled| {
                if let ResourceDescriptor::Texture(ref desc) = pooled.descriptor {
                    desc.size == descriptor.size
                        && desc.format == descriptor.format
                        && desc.usage == descriptor.usage
                        && desc.mip_level_count == descriptor.mip_level_count
                        && desc.sample_count == descriptor.sample_count
                } else {
                    false
                }
            }) {
                // Remove from available and add to in_use
                let mut pooled = available_textures.remove(pos).unwrap();
                pooled.last_used = Instant::now();
                pooled.use_count += 1;

                let texture = Arc::new(pooled.resource);
                self.in_use
                    .entry(type_id.clone())
                    .or_insert_with(Vec::new)
                    .push(PooledResource {
                        resource: (*texture).clone(),
                        descriptor: pooled.descriptor,
                        last_used: pooled.last_used,
                        use_count: pooled.use_count,
                    });

                log::debug!("Reusing texture from pool for {:?}", type_id);
                return texture;
            }
        }

        // Create a new texture
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: descriptor.label.as_deref(),
            size: descriptor.size,
            mip_level_count: descriptor.mip_level_count,
            sample_count: descriptor.sample_count,
            dimension: descriptor.dimension,
            format: descriptor.format,
            usage: descriptor.usage,
            view_formats: &[],
        });

        log::debug!("Created new texture for {:?}", type_id);

        let texture = Arc::new(texture);
        self.in_use
            .entry(type_id.clone())
            .or_insert_with(Vec::new)
            .push(PooledResource {
                resource: (*texture).clone(),
                descriptor: ResourceDescriptor::Texture(descriptor),
                last_used: Instant::now(),
                use_count: 1,
            });

        texture
    }
}

/// Statistics for resource pools
#[derive(Debug)]
pub struct PoolStats {
    pub available_count: usize,
    pub available_size: u64,
    pub in_use_count: usize,
    pub in_use_size: u64,
    pub total_allocated: u64,
}

/// Main resource pool manager
pub struct ResourcePoolManager {
    buffer_pool: BufferPool,
    texture_pool: TexturePool,
    cleanup_interval: Duration,
    last_cleanup: Instant,
}

impl ResourcePoolManager {
    /// Create a new resource pool manager
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        buffer_pool_size: usize,
        texture_pool_size: usize,
    ) -> Self {
        Self {
            buffer_pool: BufferPool::new(device.clone(), queue, buffer_pool_size),
            texture_pool: TexturePool::new(device, texture_pool_size),
            cleanup_interval: Duration::from_secs(30),
            last_cleanup: Instant::now(),
        }
    }

    /// Get or create a buffer
    pub fn get_buffer(
        &mut self,
        type_id: ResourceTypeId,
        descriptor: BufferDescriptor,
    ) -> Arc<Buffer> {
        self.maybe_cleanup();
        self.buffer_pool.get_or_create(type_id, descriptor)
    }

    /// Get or create a texture
    pub fn get_texture(
        &mut self,
        type_id: ResourceTypeId,
        descriptor: TextureDescriptor,
    ) -> Arc<Texture> {
        self.maybe_cleanup();
        self.texture_pool.get_or_create(type_id, descriptor)
    }

    /// Return a buffer to the pool
    pub fn return_buffer(&mut self, type_id: ResourceTypeId, buffer: Buffer) {
        self.buffer_pool.return_buffer(type_id, buffer);
    }

    /// Perform cleanup if needed
    fn maybe_cleanup(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_cleanup) > self.cleanup_interval {
            self.buffer_pool.cleanup_unused(60); // Clean up buffers older than 60 seconds
            self.last_cleanup = now;
        }
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> PoolStats {
        self.buffer_pool.get_stats()
    }

    /// Force cleanup of all unused resources
    pub fn force_cleanup(&mut self) {
        self.buffer_pool.cleanup_unused(0);
        self.last_cleanup = Instant::now();
    }
}
