//! Bar chart renderer implementation

use crate::{GpuBufferSet, RenderContext, Viewport};
use gpu_charts_shared::{Result, VisualConfig};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Bar chart renderer
pub struct BarChartRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    visual_config: VisualConfig,
    viewport_size: (u32, u32),
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BarUniforms {
    transform: [[f32; 4]; 4],
    bar_color: [f32; 4],
    bar_width: f32,
    bar_spacing: f32,
    viewport_width: f32,
    viewport_height: f32,
}

impl BarChartRenderer {
    pub fn new(device: &wgpu::Device, visual_config: &VisualConfig) -> Result<Self> {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Bar Chart Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/bar_chart.wgsl").into()),
        });

        // Create uniform buffer
        let uniforms = BarUniforms {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            bar_color: [0.4, 0.6, 0.9, 1.0], // Blue bars
            bar_width: 10.0,
            bar_spacing: 2.0,
            viewport_width: 1920.0,
            viewport_height: 1080.0,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bar Chart Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bar Chart Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bar Chart Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Bar Chart Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Bar Chart Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 12, // x: f32, value: f32, index: f32
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 8,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            visual_config: visual_config.clone(),
            viewport_size: (1920, 1080),
        })
    }
}

impl super::ChartRenderer for BarChartRenderer {
    fn render<'a>(
        &'a mut self,
        pass: &mut wgpu::RenderPass<'a>,
        buffer_sets: &[Arc<GpuBufferSet>],
        context: &RenderContext,
    ) {
        if buffer_sets.is_empty() {
            return;
        }

        // Update uniforms
        let scale_x = 2.0 / context.viewport.width;
        let scale_y = 2.0 / context.viewport.height;
        let translate_x = -1.0 - context.viewport.x * scale_x;
        let translate_y = -1.0 - context.viewport.y * scale_y;

        let uniforms = BarUniforms {
            transform: [
                [scale_x, 0.0, 0.0, 0.0],
                [0.0, -scale_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [translate_x, translate_y, 0.0, 1.0],
            ],
            bar_color: [0.4, 0.6, 0.9, 1.0],
            bar_width: 10.0,
            bar_spacing: 2.0,
            viewport_width: self.viewport_size.0 as f32,
            viewport_height: self.viewport_size.1 as f32,
        };

        context
            .queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Set pipeline and bind groups
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        // TODO: Render bar geometry from data buffers
        // This would use instanced rendering for efficient bar drawing
    }

    fn update_visual_config(&mut self, config: &VisualConfig) {
        self.visual_config = config.clone();
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        self.viewport_size = (width, height);
    }

    fn on_viewport_change(&mut self, _viewport: &Viewport) {
        // Viewport changes are handled during render
    }

    fn get_draw_call_count(&self) -> u32 {
        1 // One instanced draw call for all bars
    }
}
