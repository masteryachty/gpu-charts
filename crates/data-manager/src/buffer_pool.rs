//! High-performance GPU buffer pool for zero-allocation data processing

use std::collections::VecDeque;

/// Buffer pool for reusing GPU allocations
pub struct BufferPool {
    small_buffers: VecDeque<wgpu::Buffer>,  // < 1MB
    medium_buffers: VecDeque<wgpu::Buffer>, // 1MB - 16MB
    large_buffers: VecDeque<wgpu::Buffer>,  // 16MB - 128MB
    huge_buffers: VecDeque<wgpu::Buffer>,   // > 128MB
    total_allocated: u64,
    max_size: u64,
}

impl BufferPool {
    pub fn new(max_size: u64) -> Self {
        Self {
            small_buffers: VecDeque::with_capacity(100),
            medium_buffers: VecDeque::with_capacity(50),
            large_buffers: VecDeque::with_capacity(20),
            huge_buffers: VecDeque::with_capacity(10),
            total_allocated: 0,
            max_size,
        }
    }

    /// Acquire a buffer of at least the requested size
    pub fn acquire(&mut self, device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        let pool = match size {
            s if s < 1024 * 1024 => &mut self.small_buffers,
            s if s < 16 * 1024 * 1024 => &mut self.medium_buffers,
            s if s < 128 * 1024 * 1024 => &mut self.large_buffers,
            _ => &mut self.huge_buffers,
        };

        // Try to reuse existing buffer
        if let Some(buffer) = pool.iter().position(|b| b.size() >= size) {
            return pool.remove(buffer).unwrap();
        }

        // Create new buffer if needed
        self.create_buffer(device, size)
    }

    /// Return a buffer to the pool
    pub fn release(&mut self, buffer: wgpu::Buffer) {
        let size = buffer.size();

        // Don't keep too many buffers
        if self.total_allocated > self.max_size {
            // Let it drop
            self.total_allocated -= size;
            return;
        }

        let pool = match size {
            s if s < 1024 * 1024 => &mut self.small_buffers,
            s if s < 16 * 1024 * 1024 => &mut self.medium_buffers,
            s if s < 128 * 1024 * 1024 => &mut self.large_buffers,
            _ => &mut self.huge_buffers,
        };

        pool.push_back(buffer);
    }

    fn create_buffer(&mut self, device: &wgpu::Device, size: u64) -> wgpu::Buffer {
        // Round up to next power of 2 for better reuse
        let actual_size = size.next_power_of_two();

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("Pooled Buffer {}B", actual_size)),
            size: actual_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        self.total_allocated += actual_size;
        buffer
    }

    pub fn get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "small_buffers": self.small_buffers.len(),
            "medium_buffers": self.medium_buffers.len(),
            "large_buffers": self.large_buffers.len(),
            "huge_buffers": self.huge_buffers.len(),
            "total_allocated_mb": self.total_allocated as f64 / (1024.0 * 1024.0),
            "max_size_mb": self.max_size as f64 / (1024.0 * 1024.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_size_rounding() {
        assert_eq!(1000u64.next_power_of_two(), 1024);
        assert_eq!((1024 * 1024 + 1).next_power_of_two(), 2 * 1024 * 1024);
    }
}
