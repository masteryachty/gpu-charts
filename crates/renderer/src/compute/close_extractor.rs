//! Extracts close prices from OHLC candle data for use in indicators

use super::{ComputeInfrastructure, ComputeResult};
use std::rc::Rc;
use wgpu::util::DeviceExt;

/// Extracts close prices from OHLC candle buffers
pub struct CloseExtractor {
    infrastructure: Rc<ComputeInfrastructure>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ExtractParams {
    candle_count: u32,
}

impl CloseExtractor {
    /// Create a new close price extractor
    pub fn new(infrastructure: Rc<ComputeInfrastructure>) -> Result<Self, String> {
        let device = &infrastructure.device;

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Close Extractor Bind Group Layout"),
            entries: &[
                // OHLC candles input
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
                // Close prices output
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
                // Parameters
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
        let shader_source = include_str!("extract_close.wgsl");
        
        // Create pipeline
        let pipeline = infrastructure.create_compute_pipeline(
            shader_source,
            "extract_close",
            &bind_group_layout,
        )?;

        Ok(Self {
            infrastructure,
            pipeline,
            bind_group_layout,
        })
    }

    /// Extract close prices from OHLC candle buffer
    pub fn extract(
        &self,
        candles_buffer: &wgpu::Buffer,
        candle_count: u32,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeResult, String> {
        // Extract close prices from candles

        // Create output buffer for close prices
        let output_buffer = self.infrastructure.create_compute_buffer(
            (candle_count * 4) as u64, // f32 = 4 bytes
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
            "Close Prices Buffer",
        );

        // Create params buffer
        let params = ExtractParams { candle_count };
        let params_buffer = self.infrastructure.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Close Extractor Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create bind group
        let bind_group = self.infrastructure.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Close Extractor Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: candles_buffer.as_entire_binding(),
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

        // Calculate workgroup count (256 threads per workgroup)
        let workgroup_count = ((candle_count + 255) / 256, 1, 1);

        // Execute compute pass
        self.infrastructure.execute_compute(encoder, &self.pipeline, &bind_group, workgroup_count);

        Ok(ComputeResult {
            output_buffer,
            element_count: candle_count,
        })
    }
}