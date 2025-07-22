//! Candlestick chart renderer implementation

use crate::{GpuBufferSet, RenderContext, Viewport};
use gpu_charts_shared::{Result, VisualConfig};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Candlestick chart renderer for OHLC data
pub struct CandlestickRenderer {
    body_pipeline: wgpu::RenderPipeline,
    wick_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    visual_config: VisualConfig,
    viewport_size: (u32, u32),
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CandlestickUniforms {
    transform: [[f32; 4]; 4],
    bullish_color: [f32; 4],
    bearish_color: [f32; 4],
    wick_width: f32,
    viewport_width: f32,
    viewport_height: f32,
    _padding: f32,
}

impl CandlestickRenderer {
    pub fn new(device: &wgpu::Device, visual_config: &VisualConfig) -> Result<Self> {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Candlestick Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/candlestick.wgsl").into()),
        });

        // Create uniform buffer
        let uniforms = CandlestickUniforms {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            bullish_color: [0.0, 1.0, 0.0, 1.0], // Green
            bearish_color: [1.0, 0.0, 0.0, 1.0], // Red
            wick_width: 1.0,
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            _padding: 0.0,
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Candlestick Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Candlestick Bind Group Layout"),
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
            label: Some("Candlestick Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Candlestick Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create body pipeline (for candle bodies)
        let body_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Candlestick Body Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_body",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20, // time: f32, open: f32, high: f32, low: f32, close: f32
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
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
        });

        // Create wick pipeline (for candle wicks)
        let wick_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Candlestick Wick Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_wick",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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
        });

        Ok(Self {
            body_pipeline,
            wick_pipeline,
            uniform_buffer,
            uniform_bind_group,
            visual_config: visual_config.clone(),
            viewport_size: (1920, 1080),
        })
    }
}

impl super::ChartRenderer for CandlestickRenderer {
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

        let uniforms = CandlestickUniforms {
            transform: [
                [scale_x, 0.0, 0.0, 0.0],
                [0.0, -scale_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [translate_x, translate_y, 0.0, 1.0],
            ],
            bullish_color: [0.0, 0.8, 0.0, 1.0],
            bearish_color: [0.8, 0.0, 0.0, 1.0],
            wick_width: 1.0,
            viewport_width: self.viewport_size.0 as f32,
            viewport_height: self.viewport_size.1 as f32,
            _padding: 0.0,
        };

        context
            .queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Render bodies
        pass.set_pipeline(&self.body_pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        // TODO: Render actual OHLC data from buffers
        // This would involve creating vertex buffers from OHLC data

        // Render wicks
        pass.set_pipeline(&self.wick_pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        // TODO: Render wick geometry
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
        2 // One for bodies, one for wicks
    }
}
