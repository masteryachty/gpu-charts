//! Tooltip renderer for displaying data values on hover/scrub

use std::rc::Rc;
use wgpu::util::DeviceExt;

use data_manager::DataStore;
use shared_types::{TooltipConfig, TooltipLabelGpu, TooltipState};

use crate::multi_renderer::MultiRenderable;

pub struct TooltipRenderer {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    pipeline_line: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    uniform_buffer: wgpu::Buffer,
    label_buffer: Option<wgpu::Buffer>,
    _config: TooltipConfig,
    screen_width: f32,
    screen_height: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TooltipUniforms {
    view_matrix: [[f32; 4]; 4],
    screen_size: [f32; 2],
    line_x: f32,
    is_active: f32,
}

impl TooltipRenderer {
    pub fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Tooltip Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/tooltip.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Tooltip Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Tooltip Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create pipeline for vertical line
        let pipeline_line = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Tooltip Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_line"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Tooltip Uniform Buffer"),
            size: std::mem::size_of::<TooltipUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            pipeline_line,
            bind_group_layout,
            bind_group: None,
            uniform_buffer,
            label_buffer: None,
            _config: TooltipConfig::default(),
            screen_width: width as f32,
            screen_height: height as f32,
        }
    }

    pub fn update_tooltip_state(&mut self, state: &TooltipState) {
        if !state.active {
            return;
        }

        // Update uniforms
        let uniforms = TooltipUniforms {
            view_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            screen_size: [self.screen_width, self.screen_height],
            line_x: state.x_position,
            is_active: if state.active { 1.0 } else { 0.0 },
        };

        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        // Update label buffer if we have labels
        if !state.labels.is_empty() {
            let label_data: Vec<TooltipLabelGpu> = state.labels
                .iter()
                .filter(|l| l.visible)
                .map(|label| TooltipLabelGpu {
                    position: [state.x_position + 5.0, label.screen_y],
                    value: label.value,
                    color: label.color,
                    _padding: 0.0,
                })
                .collect();

            if !label_data.is_empty() {
                // Create or update label buffer
                let buffer_size = (label_data.len() * std::mem::size_of::<TooltipLabelGpu>()) as u64;
                
                if self.label_buffer.is_none() || 
                   self.label_buffer.as_ref().unwrap().size() < buffer_size {
                    self.label_buffer = Some(
                        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Tooltip Label Buffer"),
                            contents: bytemuck::cast_slice(&label_data),
                            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        })
                    );
                    
                    // Recreate bind group with new buffer
                    self.create_bind_group();
                } else {
                    self.queue.write_buffer(
                        self.label_buffer.as_ref().unwrap(),
                        0,
                        bytemuck::cast_slice(&label_data),
                    );
                }
            }
        }
    }

    fn create_bind_group(&mut self) {
        if let Some(label_buffer) = &self.label_buffer {
            self.bind_group = Some(
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Tooltip Bind Group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: label_buffer.as_entire_binding(),
                        },
                    ],
                })
            );
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.screen_width = width as f32;
        self.screen_height = height as f32;
    }
}

impl MultiRenderable for TooltipRenderer {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: &DataStore,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        // Check if we have tooltip state in the data store
        if let Some(tooltip_state) = data_store.get_tooltip_state() {
            if !tooltip_state.active {
                return;
            }

            self.update_tooltip_state(tooltip_state);

            if self.bind_group.is_none() {
                return; // No data to render
            }

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tooltip Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // Don't clear, we're overlaying
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);

            // Draw vertical line
            render_pass.set_pipeline(&self.pipeline_line);
            render_pass.draw(0..2, 0..1);

            // Skip drawing labels - we only want the vertical line
            // The React tooltip component handles displaying the data
        }
    }

    fn name(&self) -> &str {
        "TooltipRenderer"
    }

    fn priority(&self) -> u32 {
        200 // Render on top of everything
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.resize(width, height);
    }
}