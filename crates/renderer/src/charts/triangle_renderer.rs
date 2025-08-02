//! Triangle renderer for displaying trade markers
//! Renders fixed-size triangles at trade positions with color based on trade side

use crate::MultiRenderable;
use data_manager::DataStore;
use std::rc::Rc;
use wgpu::{util::DeviceExt, TextureFormat};

/// Triangle renderer for trade markers
pub struct TriangleRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    /// Fixed triangle size in pixels
    triangle_size: f32,
    /// Data group name (e.g., "trades")
    data_group_name: String,
}

impl TriangleRenderer {
    /// Create a new triangle renderer
    pub fn new(
        device: Rc<wgpu::Device>,
        _queue: Rc<wgpu::Queue>,
        color_format: TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("triangle.wgsl"));

        // Bind group layout for uniforms and data buffers
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("triangle_bind_group_layout"),
            entries: &[
                // x_min_max (u32 timestamps)
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
                // y_min_max (f32 prices) - from GPU compute shader
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
                // screen_size (for pixel-perfect triangles)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // triangle_size
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // time_buffer (storage buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // price_buffer (storage buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // side_buffer (storage buffer)
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
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
            label: Some("triangle_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("triangle_render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[], // No vertex buffers, we read from storage buffers
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            triangle_size: 8.0,
            data_group_name: "trades".to_string(),
        }
    }

    /// Set the triangle size in pixels
    pub fn set_triangle_size(&mut self, size: f32) {
        self.triangle_size = size;
    }

    /// Set the data group name to read from
    pub fn set_data_group(&mut self, group_name: String) {
        self.data_group_name = group_name;
    }

    fn create_bind_group(
        &self,
        data_store: &DataStore,
        device: &wgpu::Device,
    ) -> Option<(wgpu::BindGroup, u32)> {
        use nalgebra_glm as glm;

        // Find the data group with our name - look for a group that has both "price" and "side" metrics
        let (_group_index, data_group) =
            data_store
                .data_groups
                .iter()
                .enumerate()
                .find(|(_idx, group)| {
                    // Log metrics in this group
                    for _metric in &group.metrics {}

                    // For trades, we need a group that has both "price" and "side" metrics
                    let has_price = group.metrics.iter().any(|m| m.name == "price");
                    let has_side = group.metrics.iter().any(|m| m.name == "side");
                    // Check if this is a trades group
                    has_price && has_side
                })?;

        // The data group itself contains the time buffers
        let price_metric = data_group.metrics.iter().find(|m| m.name == "price")?;
        let side_metric = data_group.metrics.iter().find(|m| m.name == "side")?;

        // Get the time buffer from the data group itself (not from a metric)
        let time_buffer = match data_group.x_buffers.first() {
            Some(buf) => buf,
            None => {
                return None;
            }
        };

        let price_buffer = match price_metric.y_buffers.first() {
            Some(buf) => buf,
            None => {
                return None;
            }
        };

        let side_buffer = match side_metric.y_buffers.first() {
            Some(buf) => buf,
            None => {
                return None;
            }
        };

        // Calculate instance count from buffer size
        let instance_count = (time_buffer.size() / 4) as u32;

        // Debug: Log buffer info

        // X range (timestamps) - keep as u32 for precision
        let x_min = data_store.start_x;
        let x_max = data_store.end_x;
        let x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle_x_min_max"),
            contents: bytemuck::cast_slice(&[x_min, x_max]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Y range - use the GPU-computed min/max buffer
        let y_buffer = match &data_store.min_max_buffer {
            Some(buffer) => buffer.clone(),
            None => return None, // Skip rendering if no min/max buffer available
        };

        // Screen size
        let screen_size = glm::vec2(
            data_store.screen_size.width as f32,
            data_store.screen_size.height as f32,
        );
        let screen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle_screen_size"),
            contents: bytemuck::cast_slice(&[screen_size.x, screen_size.y]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Triangle size
        let size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("triangle_size"),
            contents: bytemuck::cast_slice(&[self.triangle_size]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("triangle_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: x_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: y_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: screen_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: size_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: time_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: price_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: side_buffer.as_entire_binding(),
                },
            ],
        });

        Some((bind_group, instance_count))
    }
}

impl MultiRenderable for TriangleRenderer {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: &DataStore,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        // Try to create bind group - this will fail if no trade data is available
        let (bind_group, instance_count) = match self.create_bind_group(data_store, device) {
            Some(result) => result,
            None => {
                // No trade data available, skip rendering
                return;
            }
        };

        // Skip if no instances to render
        if instance_count == 0 {
            return;
        }

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Triangle Renderer"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);

        // 3 vertices per triangle, render for each instance
        render_pass.draw(0..3, 0..instance_count);
    }

    fn name(&self) -> &str {
        "TriangleRenderer"
    }

    fn priority(&self) -> u32 {
        150 // Render on top of other elements
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        // No special resize handling needed
    }
}
