//! GPU-based indirect draw call generation
//!
//! This module implements GPU-driven rendering using indirect draw calls,
//! allowing the GPU to determine draw parameters without CPU intervention.

use crate::{GpuBufferSet, Viewport};
use gpu_charts_shared::Result;
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Configuration for indirect drawing
#[derive(Debug, Clone)]
pub struct IndirectDrawConfig {
    /// Maximum number of draw calls per frame
    pub max_draw_calls: u32,
    /// Enable multi-draw indirect
    pub enable_multi_draw: bool,
    /// Batch size for draw calls
    pub batch_size: u32,
}

impl Default for IndirectDrawConfig {
    fn default() -> Self {
        Self {
            max_draw_calls: 1000,
            enable_multi_draw: true,
            batch_size: 64,
        }
    }
}

/// Indirect draw system for GPU-driven rendering
pub struct IndirectDrawSystem {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: IndirectDrawConfig,

    // Compute pipeline for draw call generation
    draw_gen_pipeline: wgpu::ComputePipeline,
    draw_gen_bind_group_layout: wgpu::BindGroupLayout,

    // Buffers
    indirect_commands_buffer: wgpu::Buffer,
    draw_count_buffer: wgpu::Buffer,
    draw_params_buffer: wgpu::Buffer,

    // Multi-draw indirect support
    multi_draw_supported: bool,
}

impl IndirectDrawSystem {
    /// Create a new indirect draw system
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: IndirectDrawConfig,
    ) -> Result<Self> {
        // Check for multi-draw indirect support
        let features = device.features();
        let multi_draw_supported = features.contains(wgpu::Features::MULTI_DRAW_INDIRECT);

        // Create compute shader for draw call generation
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Indirect Draw Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/indirect_draw_gen.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Draw Gen Bind Group Layout"),
            entries: &[
                // Input data info buffer
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
                // Output indirect commands buffer
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
                // Draw count buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Draw generation parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
            label: Some("Draw Gen Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Indirect Draw Generation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "generate_draw_calls",
            compilation_options: Default::default(),
        });

        // Create buffers
        let indirect_command_size = if multi_draw_supported {
            std::mem::size_of::<DrawIndexedIndirectArgs>()
        } else {
            std::mem::size_of::<DrawIndirectArgs>()
        };

        let indirect_commands_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Indirect Commands Buffer"),
            size: (indirect_command_size * config.max_draw_calls as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let draw_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Draw Count Buffer"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let draw_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Draw Generation Parameters"),
            size: std::mem::size_of::<DrawGenParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            device,
            queue,
            config,
            draw_gen_pipeline: pipeline,
            draw_gen_bind_group_layout: bind_group_layout,
            indirect_commands_buffer,
            draw_count_buffer,
            draw_params_buffer,
            multi_draw_supported,
        })
    }

    /// Generate indirect draw calls based on data
    pub fn generate_draw_calls(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        buffer_set: &GpuBufferSet,
        viewport: &Viewport,
        render_mode: RenderMode,
    ) -> Result<IndirectDrawResult> {
        // Prepare parameters
        let params = DrawGenParams {
            total_vertices: buffer_set.metadata.row_count,
            viewport_start: viewport.time_range.start as f32,
            viewport_end: viewport.time_range.end as f32,
            batch_size: self.config.batch_size,
            max_draw_calls: self.config.max_draw_calls,
            render_mode: render_mode as u32,
            enable_culling: 1,
            _padding: 0,
        };

        // Update parameters buffer
        self.queue
            .write_buffer(&self.draw_params_buffer, 0, bytemuck::cast_slice(&[params]));

        // Create data info buffer
        let data_info = self.create_data_info_buffer(buffer_set)?;

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Draw Gen Bind Group"),
            layout: &self.draw_gen_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: data_info.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.indirect_commands_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.draw_count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.draw_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch compute shader
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Generate Draw Calls Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.draw_gen_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Calculate dispatch size
        let workgroup_size = 64;
        let dispatch_x = (params.total_vertices + workgroup_size - 1) / workgroup_size;
        compute_pass.dispatch_workgroups(dispatch_x, 1, 1);

        drop(compute_pass);

        Ok(IndirectDrawResult {
            commands_buffer: &self.indirect_commands_buffer,
            count_buffer: &self.draw_count_buffer,
            multi_draw_supported: self.multi_draw_supported,
            max_draw_calls: self.config.max_draw_calls,
        })
    }

    /// Create buffer with data layout information
    fn create_data_info_buffer(&self, buffer_set: &GpuBufferSet) -> Result<wgpu::Buffer> {
        let mut data_infos = Vec::new();

        // Collect information about each data buffer
        for (column_name, buffers) in &buffer_set.buffers {
            for (i, _buffer) in buffers.iter().enumerate() {
                data_infos.push(DataInfo {
                    offset: 0, // Offset within the buffer
                    stride: 4, // Assuming f32 data
                    count: buffer_set.metadata.row_count,
                    buffer_index: i as u32,
                    column_type: match column_name.as_str() {
                        "time" => 0,
                        "price" => 1,
                        "volume" => 2,
                        _ => 3,
                    },
                    _padding: [0; 3],
                });
            }
        }

        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Data Info Buffer"),
                contents: bytemuck::cast_slice(&data_infos),
                usage: wgpu::BufferUsages::STORAGE,
            });

        Ok(buffer)
    }

    /// Execute indirect draw calls
    pub fn execute_draw<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        result: &'a IndirectDrawResult,
        vertex_buffer: &'a wgpu::Buffer,
    ) {
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));

        if result.multi_draw_supported && self.config.enable_multi_draw {
            // Use multi-draw indirect
            render_pass.multi_draw_indirect(&result.commands_buffer, 0, result.max_draw_calls);
        } else {
            // Fall back to single draw indirect
            render_pass.draw_indirect(&result.commands_buffer, 0);
        }
    }
}

/// Parameters for draw call generation
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawGenParams {
    total_vertices: u32,
    viewport_start: f32,
    viewport_end: f32,
    batch_size: u32,
    max_draw_calls: u32,
    render_mode: u32,
    enable_culling: u32,
    _padding: u32,
}

/// Information about data layout
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DataInfo {
    offset: u32,
    stride: u32,
    count: u32,
    buffer_index: u32,
    column_type: u32,
    _padding: [u32; 3],
}

/// Standard draw indirect arguments
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawIndirectArgs {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

/// Indexed draw indirect arguments
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawIndexedIndirectArgs {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

/// Render mode for draw call generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RenderMode {
    Points = 0,
    Lines = 1,
    LineStrip = 2,
    Triangles = 3,
}

/// Result of draw call generation
pub struct IndirectDrawResult<'a> {
    pub commands_buffer: &'a wgpu::Buffer,
    pub count_buffer: &'a wgpu::Buffer,
    pub multi_draw_supported: bool,
    pub max_draw_calls: u32,
}

/// Batched indirect draw system for optimal GPU utilization
pub struct BatchedDrawSystem {
    indirect_system: IndirectDrawSystem,
    batch_manager: DrawBatchManager,
}

impl BatchedDrawSystem {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: IndirectDrawConfig,
    ) -> Result<Self> {
        let indirect_system = IndirectDrawSystem::new(device, queue, config.clone())?;
        let batch_manager = DrawBatchManager::new(config.batch_size);

        Ok(Self {
            indirect_system,
            batch_manager,
        })
    }

    /// Generate batched draw calls for multiple datasets
    pub fn generate_batched_draws(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        datasets: Vec<(&GpuBufferSet, &Viewport, RenderMode)>,
    ) -> Result<Vec<IndirectDrawResult>> {
        let mut results = Vec::new();

        // Group datasets into batches
        let batches = self.batch_manager.create_batches(&datasets);

        for batch in batches {
            // Process each batch
            for &(buffer_set, viewport, mode) in &batch {
                let result = self
                    .indirect_system
                    .generate_draw_calls(encoder, buffer_set, viewport, *mode)?;
                results.push(result);
            }
        }

        Ok(results)
    }
}

/// Manages batching of draw calls for optimal performance
struct DrawBatchManager {
    batch_size: u32,
    batch_stats: BatchStatistics,
}

impl DrawBatchManager {
    fn new(batch_size: u32) -> Self {
        Self {
            batch_size,
            batch_stats: BatchStatistics::default(),
        }
    }

    /// Create optimal batches from datasets
    fn create_batches<'a>(
        &mut self,
        datasets: &'a [(&GpuBufferSet, &Viewport, RenderMode)],
    ) -> Vec<Vec<&'a (&'a GpuBufferSet, &'a Viewport, RenderMode)>> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_vertices = 0u32;

        for dataset in datasets {
            let vertex_count = dataset.0.metadata.row_count;

            // Check if adding this dataset would exceed batch size
            if current_vertices + vertex_count > self.batch_size && !current_batch.is_empty() {
                // Finalize current batch
                batches.push(current_batch);
                current_batch = Vec::new();
                current_vertices = 0;

                self.batch_stats.batches_created += 1;
            }

            current_batch.push(dataset);
            current_vertices += vertex_count;
        }

        // Add final batch
        if !current_batch.is_empty() {
            batches.push(current_batch);
            self.batch_stats.batches_created += 1;
        }

        // Update statistics
        self.batch_stats.average_batch_size = datasets.len() as f32 / batches.len().max(1) as f32;

        batches
    }
}

/// Statistics for batch management
#[derive(Debug, Default, Clone)]
struct BatchStatistics {
    batches_created: u32,
    average_batch_size: f32,
    total_vertices_processed: u64,
}

/// Extension for conditional rendering
pub struct ConditionalRenderingExt {
    predicate_buffer: wgpu::Buffer,
    device: Arc<wgpu::Device>,
}

impl ConditionalRenderingExt {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        let predicate_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Conditional Rendering Predicate"),
            size: 4, // Single u32
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            predicate_buffer,
            device,
        }
    }

    /// Set rendering condition
    pub fn set_condition(&self, queue: &wgpu::Queue, should_render: bool) {
        let value = if should_render { 1u32 } else { 0u32 };
        queue.write_buffer(&self.predicate_buffer, 0, bytemuck::cast_slice(&[value]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_args_size() {
        assert_eq!(std::mem::size_of::<DrawIndirectArgs>(), 16);
        assert_eq!(std::mem::size_of::<DrawIndexedIndirectArgs>(), 20);
    }

    #[test]
    fn test_draw_gen_params_alignment() {
        assert_eq!(std::mem::size_of::<DrawGenParams>(), 32);
    }

    #[test]
    fn test_batch_manager() {
        let mut manager = DrawBatchManager::new(10000);

        // Mock datasets
        let datasets = vec![
            // Each tuple would normally contain real data
        ];

        let batches = manager.create_batches(&datasets);
        assert_eq!(batches.len(), 0); // No datasets provided
    }
}
