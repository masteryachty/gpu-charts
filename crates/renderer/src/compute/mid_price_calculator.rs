//! Mid price calculator using GPU compute shaders

use super::{ComputeInfrastructure, ComputeProcessor, ComputeResult};
use std::rc::Rc;
use wgpu::util::DeviceExt;

/// Calculates mid price from bid and ask data using GPU compute
pub struct MidPriceCalculator {
    infrastructure: ComputeInfrastructure,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,
}

/// Parameters for the compute shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeParams {
    element_count: u32,
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
}

impl MidPriceCalculator {
    /// Create a new mid price calculator
    pub fn new(device: Rc<wgpu::Device>, queue: Rc<wgpu::Queue>) -> Result<Self, String> {
        let infrastructure = ComputeInfrastructure::new(device.clone(), queue);

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mid Price Compute Bind Group Layout"),
            entries: &[
                // Bid data buffer
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
                // Ask data buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output mid data buffer
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
                // Parameters uniform
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

        // Load shader
        let shader_source = include_str!("mid_price_compute.wgsl");
        let pipeline = infrastructure.create_compute_pipeline(
            shader_source,
            "compute_mid_price",
            &bind_group_layout,
        )?;

        // Create params buffer
        let params = ComputeParams {
            element_count: 0,
            _padding1: 0,
            _padding2: 0,
            _padding3: 0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Compute Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            infrastructure,
            pipeline,
            bind_group_layout,
            params_buffer,
        })
    }

    /// Calculate mid price from bid and ask buffers
    pub fn calculate(
        &self,
        bid_buffer: &wgpu::Buffer,
        ask_buffer: &wgpu::Buffer,
        element_count: u32,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeResult, String> {
        // Update params
        let params = ComputeParams {
            element_count,
            _padding1: 0,
            _padding2: 0,
            _padding3: 0,
        };

        self.infrastructure.queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::cast_slice(&[params]),
        );

        // Create output buffer
        let output_buffer = self.infrastructure.create_compute_buffer(
            (element_count * 4) as u64, // f32 = 4 bytes
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
            "Mid Price Output Buffer",
        );

        // Create bind group
        let bind_group = self
            .infrastructure
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Mid Price Compute Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: bid_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: ask_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                ],
            });

        // Calculate workgroup count (256 threads per workgroup)
        let workgroup_count = ((element_count + 255) / 256, 1, 1);

        // Execute compute pass
        self.infrastructure
            .execute_compute(encoder, &self.pipeline, &bind_group, workgroup_count);

        Ok(ComputeResult {
            output_buffer,
            element_count,
        })
    }

    /// Calculate with automatic buffer creation from raw data
    pub fn calculate_from_data(
        &self,
        bid_data: &[f32],
        ask_data: &[f32],
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeResult, String> {
        if bid_data.len() != ask_data.len() {
            return Err("Bid and ask data must have the same length".to_string());
        }

        let element_count = bid_data.len() as u32;

        // Create input buffers
        let bid_buffer =
            self.infrastructure
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Bid Data Buffer"),
                    contents: bytemuck::cast_slice(bid_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let ask_buffer =
            self.infrastructure
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Ask Data Buffer"),
                    contents: bytemuck::cast_slice(ask_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        self.calculate(&bid_buffer, &ask_buffer, element_count, encoder)
    }
}

impl ComputeProcessor for MidPriceCalculator {
    fn compute(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeResult, String> {
        // This would be called with specific buffers
        // For now, return an error indicating it needs to be called with data
        Err("MidPriceCalculator requires bid and ask buffers".to_string())
    }

    fn name(&self) -> &str {
        "MidPriceCalculator"
    }
}
