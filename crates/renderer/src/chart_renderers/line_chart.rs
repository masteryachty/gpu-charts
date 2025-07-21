//! Line chart renderer implementation

use crate::{GpuBufferSet, RenderContext, Viewport};
use gpu_charts_shared::{Result, VisualConfig};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Line chart renderer with high-performance GPU rendering
pub struct LineChartRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    visual_config: VisualConfig,
    viewport_size: (u32, u32),
    vertex_count: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LineUniforms {
    transform: [[f32; 4]; 4],
    color: [f32; 4],
    line_width: f32,
    viewport_width: f32,
    viewport_height: f32,
    _padding: f32,
}

impl LineChartRenderer {
    pub fn new(device: &wgpu::Device, visual_config: &VisualConfig) -> Result<Self> {
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Chart Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/line_chart.wgsl").into()),
        });
        
        // Create uniform buffer
        let uniforms = LineUniforms {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            color: [1.0, 1.0, 1.0, 1.0],
            line_width: 2.0,
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            _padding: 0.0,
        };
        
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Chart Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Line Chart Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Line Chart Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        
        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Chart Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Chart Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 8, // x: f32, y: f32
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
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
                topology: wgpu::PrimitiveTopology::LineStrip,
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
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            visual_config: visual_config.clone(),
            viewport_size: (1920, 1080),
            vertex_count: 0,
        })
    }
    
    fn update_uniforms(&self, queue: &wgpu::Queue, viewport: &Viewport) {
        // Calculate transform matrix based on viewport
        let scale_x = 2.0 / viewport.width;
        let scale_y = 2.0 / viewport.height;
        let translate_x = -1.0 - viewport.x * scale_x;
        let translate_y = -1.0 - viewport.y * scale_y;
        
        let uniforms = LineUniforms {
            transform: [
                [scale_x, 0.0, 0.0, 0.0],
                [0.0, -scale_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [translate_x, translate_y, 0.0, 1.0],
            ],
            color: self.visual_config.text_color, // Use text color for lines
            line_width: 2.0,
            viewport_width: self.viewport_size.0 as f32,
            viewport_height: self.viewport_size.1 as f32,
            _padding: 0.0,
        };
        
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }
}

impl super::ChartRenderer for LineChartRenderer {
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
        self.update_uniforms(context.queue, &context.viewport);
        
        // Set pipeline and bind groups
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        
        // Render each buffer set
        for buffer_set in buffer_sets {
            // Look for time and price columns
            if let (Some(time_buffers), Some(price_buffers)) = (
                buffer_set.buffers.get("time"),
                buffer_set.buffers.get("price"),
            ) {
                // Render each buffer chunk
                for (_time_buffer, _price_buffer) in time_buffers.iter().zip(price_buffers.iter()) {
                    // In a real implementation, we'd create vertex buffers from time/price data
                    // For now, we'll use the buffers directly if they're in the right format
                    // This would involve GPU compute to transform the data
                    
                    // Placeholder: assume buffers are already in vertex format
                    // pass.set_vertex_buffer(0, buffer.slice(..));
                    // pass.draw(0..vertex_count, 0..1);
                }
            }
        }
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
        1 // One draw call per data buffer
    }
}