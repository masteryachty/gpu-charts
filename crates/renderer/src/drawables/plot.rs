use std::rc::Rc;

use wgpu::TextureFormat;

use data_manager::{DataStore, Vertex};

pub struct PlotRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    device: Rc<wgpu::Device>,
    /// Optional filter for specific data columns (data_type, column_name)
    data_filter: Option<Vec<(String, String)>>,
}

impl PlotRenderer {
    /// Set the data filter to restrict which data columns this renderer will display
    pub fn set_data_filter(&mut self, filter: Option<Vec<(String, String)>>) {
        self.data_filter = filter;
    }

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: &DataStore,
    ) {
        // Skip rendering if no min/max buffer is available
        if data_store.min_max_buffer.is_none() {
            return;
        }

        let data_len = data_store.get_data_len();
        if data_len > 0 {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Plot Renderer"),
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

            {
                render_pass.set_pipeline(&self.pipeline);

                // Get visible metrics and apply filter if set
                let visible_metrics = data_store.get_all_visible_metrics();

                for (data_series, metric) in visible_metrics {
                    // Apply data filter if set
                    if let Some(ref filter) = self.data_filter {
                        let mut should_render = false;

                        // Check if this metric matches any of our filter criteria
                        for (_data_type, column_name) in filter {
                            if metric.name == *column_name {
                                should_render = true;
                                break;
                            }
                        }

                        if !should_render {
                            continue;
                        }
                    }
                    
                    // Check if we have buffers to render
                    if metric.y_buffers.is_empty() || data_series.x_buffers.is_empty() {
                        continue;
                    }
                    
                    // Create a bind group for this specific metric with its color
                    let bind_group = self.create_bind_group_for_metric(data_store, metric);
                    render_pass.set_bind_group(0, &bind_group, &[]);

                    for (i, x_buffer) in data_series.x_buffers.iter().enumerate() {
                        if let Some(y_buffer) = metric.y_buffers.get(i) {
                            render_pass.set_vertex_buffer(0, x_buffer.slice(..));
                            render_pass.set_vertex_buffer(1, y_buffer.slice(..));
                            
                            // Use the smaller of the two buffer sizes to avoid overrun
                            let x_count = (x_buffer.size() / 4) as u32;
                            let y_count = (y_buffer.size() / 4) as u32;
                            let vertex_count = x_count.min(y_count);
                            
                            render_pass.draw(0..vertex_count, 0..1);
                        }
                    }
                }
            }
        }
    }
}

impl PlotRenderer {
    fn create_bind_group_for_metric(
        &self,
        data_store: &DataStore,
        metric: &data_manager::MetricSeries,
    ) -> wgpu::BindGroup {
        use wgpu::util::DeviceExt;

        // Create buffers for x_min_max, y_min_max, and color
        let x_min_max = [data_store.start_x, data_store.end_x];
        let x_min_max_bytes = bytemuck::cast_slice(&x_min_max);
        let x_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("x_min_max buffer"),
                contents: x_min_max_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Use GPU-computed min/max buffer (we checked it exists in render())
        let y_buffer = data_store.min_max_buffer.as_ref().unwrap().clone();

        let color_bytes = bytemuck::cast_slice(&metric.color);
        let color_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("color buffer"),
                contents: color_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("bind_group_{}", metric.name)),
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
                    resource: color_buffer.as_entire_binding(),
                },
            ],
        })
    }

    pub fn new(
        device: Rc<wgpu::Device>,
        _queue: Rc<wgpu::Queue>,
        color_format: TextureFormat,
    ) -> PlotRenderer {
        let shader = device.create_shader_module(wgpu::include_wgsl!("plot.wgsl"));
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Triangle Render Pipeling"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Vertex::get_x_layout(), Vertex::get_y_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_format.into())],
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
            // primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            bind_group_layout,
            device,
            data_filter: None,
        }
    }
}

// let vertices: [Vertex; 4] = [
//     Vertex {
//         position: Vec3::new(100., 100., 0.0),
//         color: Vec3::new(1.0, 0.0, 0.0),
//     },
//     Vertex {
//         position: Vec3::new(700., 500., 0.0),
//         color: Vec3::new(0.0, 1.0, 0.0),
//     },
//     Vertex {
//         position: Vec3::new(700., 100., 0.0),
//         color: Vec3::new(0.0, 0.0, 1.0),
//     },
//     Vertex {
//         position: Vec3::new(100., 500., 0.0),
//         color: Vec3::new(1.0, 0.0, 0.0),
//     },
// ];

// let x = projection
//     * glm::vec4(
//         vertices[0].position.x,
//         vertices[0].position.y,
//         vertices[0].position.z,
//         1.,
//     );
// let bytes: &[u8] = unsafe { any_as_u8_slice(&vertices) };
