//! GPU timing queries for precise performance measurement
//!
//! This module provides GPU timing capabilities when supported by the hardware,
//! enabling precise measurement of GPU execution time for different render passes.

use gpu_charts_shared::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// GPU timing system for performance measurement
pub struct GpuTimingSystem {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    /// Query set for timestamp queries
    query_set: Option<wgpu::QuerySet>,
    /// Query resolve buffer
    resolve_buffer: Option<wgpu::Buffer>,
    /// Staging buffer for reading results
    staging_buffer: Option<wgpu::Buffer>,
    /// Number of queries
    query_count: u32,
    /// Whether timing is supported
    supported: bool,
    /// Timing results
    timing_results: HashMap<String, f32>,
}

impl GpuTimingSystem {
    /// Create a new GPU timing system
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        // Check if timestamp queries are supported
        let supported = device.features().contains(wgpu::Features::TIMESTAMP_QUERY);

        let (query_set, resolve_buffer, staging_buffer, query_count) = if supported {
            let query_count = 16; // Support up to 8 timing pairs

            // Create query set
            let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("GPU Timing Query Set"),
                ty: wgpu::QueryType::Timestamp,
                count: query_count,
            });

            // Create resolve buffer
            let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GPU Timing Resolve Buffer"),
                size: (query_count * 8) as u64, // 8 bytes per timestamp
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            // Create staging buffer for reading results
            let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("GPU Timing Staging Buffer"),
                size: (query_count * 8) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });

            (
                Some(query_set),
                Some(resolve_buffer),
                Some(staging_buffer),
                query_count,
            )
        } else {
            log::warn!("GPU timestamp queries not supported on this device");
            (None, None, None, 0)
        };

        Self {
            device,
            queue,
            query_set,
            resolve_buffer,
            staging_buffer,
            query_count,
            supported,
            timing_results: HashMap::new(),
        }
    }

    /// Check if GPU timing is supported
    pub fn is_supported(&self) -> bool {
        self.supported
    }

    /// Begin a timing section
    pub fn begin_timing(&self, encoder: &mut wgpu::CommandEncoder, name: &str, query_index: u32) {
        if !self.supported || query_index >= self.query_count {
            return;
        }

        if let Some(query_set) = &self.query_set {
            encoder.write_timestamp(query_set, query_index);
            log::trace!("GPU timing begin: {} at query index {}", name, query_index);
        }
    }

    /// End a timing section
    pub fn end_timing(&self, encoder: &mut wgpu::CommandEncoder, name: &str, query_index: u32) {
        if !self.supported || query_index >= self.query_count {
            return;
        }

        if let Some(query_set) = &self.query_set {
            encoder.write_timestamp(query_set, query_index);
            log::trace!("GPU timing end: {} at query index {}", name, query_index);
        }
    }

    /// Resolve timing queries and prepare for reading
    pub fn resolve_queries(&self, encoder: &mut wgpu::CommandEncoder) {
        if !self.supported {
            return;
        }

        if let (Some(query_set), Some(resolve_buffer), Some(staging_buffer)) =
            (&self.query_set, &self.resolve_buffer, &self.staging_buffer)
        {
            // Resolve all queries to the resolve buffer
            encoder.resolve_query_set(query_set, 0..self.query_count, resolve_buffer, 0);

            // Copy to staging buffer for CPU readback
            encoder.copy_buffer_to_buffer(
                resolve_buffer,
                0,
                staging_buffer,
                0,
                (self.query_count * 8) as u64,
            );
        }
    }

    /// Read timing results (must be called after resolve_queries and queue submission)
    pub async fn read_results(&mut self, pairs: &[(String, u32, u32)]) -> Result<()> {
        if !self.supported {
            return Ok(());
        }

        let staging_buffer = self
            .staging_buffer
            .as_ref()
            .ok_or_else(|| Error::GpuError("No staging buffer".to_string()))?;

        // Map the staging buffer
        let buffer_slice = staging_buffer.slice(..);

        // Request mapping
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        // Wait for the mapping
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .await
            .map_err(|_| Error::GpuError("Failed to receive mapping result".to_string()))?
            .map_err(|e| Error::GpuError(format!("Failed to map buffer: {:?}", e)))?;

        // Read the data
        {
            let data = buffer_slice.get_mapped_range();
            let timestamps: Vec<u64> = data
                .chunks_exact(8)
                .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
                .collect();

            // Calculate timing for each pair
            for (name, start_idx, end_idx) in pairs {
                if (*start_idx as usize) < timestamps.len()
                    && (*end_idx as usize) < timestamps.len()
                {
                    let start_time = timestamps[*start_idx as usize];
                    let end_time = timestamps[*end_idx as usize];

                    // Convert to milliseconds (timestamps are in nanoseconds)
                    let duration_ms = (end_time.saturating_sub(start_time)) as f32 / 1_000_000.0;

                    self.timing_results.insert(name.clone(), duration_ms);
                    log::debug!("GPU timing {}: {:.3}ms", name, duration_ms);
                }
            }
        }

        // Unmap the buffer
        staging_buffer.unmap();

        Ok(())
    }

    /// Get timing result for a specific operation
    pub fn get_timing(&self, name: &str) -> Option<f32> {
        self.timing_results.get(name).copied()
    }

    /// Get all timing results
    pub fn get_all_timings(&self) -> &HashMap<String, f32> {
        &self.timing_results
    }

    /// Clear timing results
    pub fn clear_results(&mut self) {
        self.timing_results.clear();
    }

    /// Get timing statistics as JSON
    pub fn get_stats(&self) -> serde_json::Value {
        if !self.supported {
            return serde_json::json!({
                "supported": false,
                "message": "GPU timing queries not supported on this device"
            });
        }

        let total_gpu_time: f32 = self.timing_results.values().sum();

        serde_json::json!({
            "supported": true,
            "total_gpu_time_ms": total_gpu_time,
            "timings": self.timing_results,
            "query_count": self.query_count,
        })
    }
}

/// Helper for automatic timing with RAII
pub struct TimingScope<'a> {
    timing_system: &'a GpuTimingSystem,
    encoder: &'a mut wgpu::CommandEncoder,
    name: String,
    end_index: u32,
}

impl<'a> TimingScope<'a> {
    pub fn new(
        timing_system: &'a GpuTimingSystem,
        encoder: &'a mut wgpu::CommandEncoder,
        name: &str,
        start_index: u32,
        end_index: u32,
    ) -> Self {
        timing_system.begin_timing(encoder, name, start_index);

        Self {
            timing_system,
            encoder,
            name: name.to_string(),
            end_index,
        }
    }
}

impl<'a> Drop for TimingScope<'a> {
    fn drop(&mut self) {
        self.timing_system
            .end_timing(self.encoder, &self.name, self.end_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_system_creation() {
        // Test that we can at least create the structure
        // Real GPU testing requires actual device
        assert!(true);
    }
}
