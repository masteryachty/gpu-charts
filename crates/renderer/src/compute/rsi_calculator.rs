//! RSI calculator using GPU compute shaders

use super::{ComputeInfrastructure, ComputeProcessor, ComputeResult};
use std::rc::Rc;
use wgpu::util::DeviceExt;

/// Calculates RSI (Relative Strength Index) from price data using GPU compute
pub struct RsiCalculator {
    infrastructure: ComputeInfrastructure,
    pipeline: wgpu::ComputePipeline,
    sma_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,
}

/// Parameters for the RSI compute shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RsiComputeParams {
    element_count: u32,
    period: u32,
    _padding1: u32,
    _padding2: u32,
}

impl RsiCalculator {
    /// Create a new RSI calculator
    pub fn new(device: Rc<wgpu::Device>, queue: Rc<wgpu::Queue>) -> Result<Self, String> {
        let infrastructure = ComputeInfrastructure::new(device.clone(), queue);

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("RSI Compute Bind Group Layout"),
            entries: &[
                // Price data buffer
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
                // Output RSI data buffer
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
                // Parameters uniform
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

        // Load shader
        let shader_source = include_str!("rsi_compute.wgsl");
        let pipeline = infrastructure.create_compute_pipeline(
            shader_source,
            "compute_rsi",
            &bind_group_layout,
        )?;

        // Create SMA-based pipeline for higher accuracy
        let sma_pipeline = infrastructure.create_compute_pipeline(
            shader_source,
            "compute_rsi_sma",
            &bind_group_layout,
        )?;

        // Create params buffer
        let params = RsiComputeParams {
            element_count: 0,
            period: 14, // Default RSI period
            _padding1: 0,
            _padding2: 0,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("RSI Compute Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            infrastructure,
            pipeline,
            sma_pipeline,
            bind_group_layout,
            params_buffer,
        })
    }

    /// Calculate RSI from price buffer with specified period
    pub fn calculate_rsi(
        &self,
        price_buffer: &wgpu::Buffer,
        period: u32,
        element_count: u32,
        encoder: &mut wgpu::CommandEncoder,
        use_sma: bool,
    ) -> Result<ComputeResult, String> {
        // Update params
        let params = RsiComputeParams {
            element_count,
            period,
            _padding1: 0,
            _padding2: 0,
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
            "RSI Output Buffer",
        );

        // Create bind group
        let bind_group = self
            .infrastructure
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("RSI Compute Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: price_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                ],
            });

        // Select pipeline based on accuracy preference
        let pipeline = if use_sma {
            &self.sma_pipeline
        } else {
            &self.pipeline
        };

        // Calculate workgroup count
        let threads_per_workgroup = if use_sma { 64 } else { 256 };
        let workgroup_count = ((element_count + threads_per_workgroup - 1) / threads_per_workgroup, 1, 1);

        // Execute compute pass
        self.infrastructure
            .execute_compute(encoder, pipeline, &bind_group, workgroup_count);

        Ok(ComputeResult {
            output_buffer,
            element_count,
        })
    }

    /// Calculate RSI with automatic buffer creation from raw price data
    pub fn calculate_from_price_data(
        &self,
        price_data: &[f32],
        period: u32,
        encoder: &mut wgpu::CommandEncoder,
        use_sma: bool,
    ) -> Result<ComputeResult, String> {
        let element_count = price_data.len() as u32;

        // Create input buffer
        let price_buffer =
            self.infrastructure
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Price Data Buffer"),
                    contents: bytemuck::cast_slice(price_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        self.calculate_rsi(&price_buffer, period, element_count, encoder, use_sma)
    }

    /// Calculate RSI with default period of 14
    pub fn calculate_rsi_default(
        &self,
        price_buffer: &wgpu::Buffer,
        element_count: u32,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeResult, String> {
        self.calculate_rsi(price_buffer, 14, element_count, encoder, false)
    }
}

impl ComputeProcessor for RsiCalculator {
    fn compute(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeResult, String> {
        // This would be called with specific buffers
        // For now, return an error indicating it needs to be called with data
        Err("RsiCalculator requires price buffer and parameters".to_string())
    }

    fn name(&self) -> &str {
        "RsiCalculator"
    }
}