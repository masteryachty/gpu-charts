//! Vertex compression for ultra-compact GPU data
//!
//! This module implements aggressive vertex compression techniques to achieve
//! <8 byte vertex formats for maximum GPU bandwidth efficiency.

use gpu_charts_shared::Result;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Compressed vertex format (8 bytes total)
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CompressedVertex {
    /// Packed time (16 bits) + value (16 bits)
    time_value: u32,
    /// Packed metadata: color index (8 bits) + flags (8 bits) + extra data (16 bits)
    metadata: u32,
}

impl CompressedVertex {
    /// Pack time and value into compressed format
    pub fn pack(time: f32, value: f32, time_range: (f32, f32), value_range: (f32, f32)) -> Self {
        // Normalize time to 0-1 range
        let normalized_time = (time - time_range.0) / (time_range.1 - time_range.0);
        let time_u16 = (normalized_time.clamp(0.0, 1.0) * 65535.0) as u16;

        // Normalize value to 0-1 range
        let normalized_value = (value - value_range.0) / (value_range.1 - value_range.0);
        let value_u16 = (normalized_value.clamp(0.0, 1.0) * 65535.0) as u16;

        Self {
            time_value: ((time_u16 as u32) << 16) | (value_u16 as u32),
            metadata: 0,
        }
    }

    /// Unpack time from compressed format
    pub fn unpack_time(&self, time_range: (f32, f32)) -> f32 {
        let time_u16 = (self.time_value >> 16) as u16;
        let normalized = (time_u16 as f32) / 65535.0;
        time_range.0 + normalized * (time_range.1 - time_range.0)
    }

    /// Unpack value from compressed format
    pub fn unpack_value(&self, value_range: (f32, f32)) -> f32 {
        let value_u16 = (self.time_value & 0xFFFF) as u16;
        let normalized = (value_u16 as f32) / 65535.0;
        value_range.0 + normalized * (value_range.1 - value_range.0)
    }
}

/// Ultra-compressed vertex format (4 bytes total)
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UltraCompressedVertex {
    /// Packed data: time (12 bits) + value (12 bits) + flags (8 bits)
    packed_data: u32,
}

impl UltraCompressedVertex {
    /// Pack with reduced precision for extreme compression
    pub fn pack(time: f32, value: f32, time_range: (f32, f32), value_range: (f32, f32)) -> Self {
        // 12-bit precision for time and value
        let normalized_time = (time - time_range.0) / (time_range.1 - time_range.0);
        let time_u12 = (normalized_time.clamp(0.0, 1.0) * 4095.0) as u32;

        let normalized_value = (value - value_range.0) / (value_range.1 - value_range.0);
        let value_u12 = (normalized_value.clamp(0.0, 1.0) * 4095.0) as u32;

        Self {
            packed_data: (time_u12 << 20) | (value_u12 << 8),
        }
    }
}

/// Vertex compression system
pub struct VertexCompressionSystem {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    /// Compression compute pipeline
    compression_pipeline: wgpu::ComputePipeline,
    compression_bind_group_layout: wgpu::BindGroupLayout,

    /// Decompression shader module for runtime use
    decompression_shader: wgpu::ShaderModule,
}

impl VertexCompressionSystem {
    /// Create new vertex compression system
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self> {
        // Create compression compute shader
        let compression_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Compression Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/vertex_compression.wgsl").into(),
            ),
        });

        // Create decompression shader for runtime
        let decompression_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Decompression Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/vertex_decompression.wgsl").into(),
            ),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compression Bind Group Layout"),
            entries: &[
                // Input vertices
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output compressed vertices
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Compression parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compression Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create compute pipeline
        let compression_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Vertex Compression Pipeline"),
                layout: Some(&pipeline_layout),
                module: &compression_shader,
                entry_point: Some("compress_vertices"),
                compilation_options: Default::default(),
                cache: None,
            });

        Ok(Self {
            device,
            queue,
            compression_pipeline,
            compression_bind_group_layout: bind_group_layout,
            decompression_shader,
        })
    }

    /// Compress vertex data on GPU
    pub fn compress_vertices(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        input_buffer: &wgpu::Buffer,
        vertex_count: u32,
        time_range: (f32, f32),
        value_range: (f32, f32),
    ) -> Result<wgpu::Buffer> {
        // Create output buffer for compressed vertices
        let compressed_size = vertex_count as u64 * std::mem::size_of::<CompressedVertex>() as u64;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compressed Vertex Buffer"),
            size: compressed_size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create compression parameters
        let params = CompressionParams {
            vertex_count,
            time_min: time_range.0,
            time_max: time_range.1,
            value_min: value_range.0,
            value_max: value_range.1,
            compression_mode: 0, // Standard compression
            _padding: [0; 2],
        };

        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Compression Parameters"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compression Bind Group"),
            layout: &self.compression_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch compression compute shader
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Vertex Compression Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compression_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_size = 256;
        let dispatch_x = (vertex_count + workgroup_size - 1) / workgroup_size;
        compute_pass.dispatch_workgroups(dispatch_x, 1, 1);

        drop(compute_pass);

        Ok(output_buffer)
    }

    /// Get decompression shader module for use in render pipelines
    pub fn get_decompression_shader(&self) -> &wgpu::ShaderModule {
        &self.decompression_shader
    }
}

/// Compression parameters
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CompressionParams {
    vertex_count: u32,
    time_min: f32,
    time_max: f32,
    value_min: f32,
    value_max: f32,
    compression_mode: u32,
    _padding: [u32; 2],
}

/// Advanced compression with delta encoding
pub struct DeltaCompressionSystem {
    base: VertexCompressionSystem,
    delta_pipeline: wgpu::ComputePipeline,
}

impl DeltaCompressionSystem {
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self> {
        let base = VertexCompressionSystem::new(device.clone(), queue)?;

        // Create delta compression pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Delta Compression Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/delta_compression.wgsl").into()),
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Delta Compression Pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("delta_compress"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            base,
            delta_pipeline: pipeline,
        })
    }
}

/// Compression statistics
#[derive(Debug, Default, Clone)]
pub struct CompressionStats {
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f32,
    pub quality_loss: f32,
}

impl CompressionStats {
    pub fn calculate(
        original_vertices: u32,
        compressed_vertices: u32,
        bytes_per_vertex: u32,
    ) -> Self {
        let original_size = original_vertices as u64 * 8; // Assuming f32 pairs
        let compressed_size = compressed_vertices as u64 * bytes_per_vertex as u64;

        Self {
            original_size,
            compressed_size,
            compression_ratio: original_size as f32 / compressed_size as f32,
            quality_loss: 0.0, // Would need to calculate actual precision loss
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_compression() {
        let time_range = (0.0, 1000.0);
        let value_range = (-100.0, 100.0);

        // Test packing
        let vertex = CompressedVertex::pack(500.0, 0.0, time_range, value_range);

        // Test unpacking
        let unpacked_time = vertex.unpack_time(time_range);
        let unpacked_value = vertex.unpack_value(value_range);

        // Check precision (16-bit quantization)
        assert!((unpacked_time - 500.0).abs() < 0.1);
        assert!((unpacked_value - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_vertex_sizes() {
        assert_eq!(std::mem::size_of::<CompressedVertex>(), 8);
        assert_eq!(std::mem::size_of::<UltraCompressedVertex>(), 4);
    }

    #[test]
    fn test_compression_stats() {
        let stats = CompressionStats::calculate(1_000_000, 1_000_000, 8);

        assert_eq!(stats.original_size, 8_000_000);
        assert_eq!(stats.compressed_size, 8_000_000);
        assert_eq!(stats.compression_ratio, 1.0);
    }
}
