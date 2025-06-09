use std::cell::RefCell;
use std::rc::Rc;

use wgpu::TextureFormat;

use crate::renderer::data_store::{DataStore, Vertex};
use crate::renderer::render_engine::RenderEngine;

pub struct PlotRenderer {
    pipeline: wgpu::RenderPipeline,
    // pub engine: Rc<RefCell<RenderEngine>>,
    // bind_group_layout: wgpu::BindGroupLayout,
}

pub trait RenderListener {
    fn on_render(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: Rc<RefCell<DataStore>>,
    );
}

impl RenderListener for PlotRenderer {
    fn on_render(
        &mut self,
        _: &wgpu::Queue,
        _: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: Rc<RefCell<DataStore>>,
    ) {
        //log::info!("Render plot2");
        // let device = &self.engine.borrow().device;

        let data_len = data_store.borrow().get_data_len();
        // log::info!("Data len: {}", data_len);
        if data_len > 0 {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Triangle Drawer"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            {
                let ds = data_store.borrow();
                let bind_group = ds.range_bind_group.as_ref().unwrap();
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);

                for (i, buffer) in ds.data_groups[ds.active_data_group_index]
                    .x_buffers
                    .iter()
                    .enumerate()
                {
                    render_pass.set_vertex_buffer(0, buffer.slice(..));
                    render_pass.set_vertex_buffer(
                        1,
                        ds.data_groups[ds.active_data_group_index].y_buffers[i].slice(..),
                    );
                    log::info!("size: {:?}", buffer.size());
                    render_pass.draw(0..(buffer.size() / 4) as u32, 0..1);
                }
            }
        }
        //log render
    }
}

impl PlotRenderer {
    pub fn new(
        engine: Rc<RefCell<RenderEngine>>,
        color_format: TextureFormat,
        _: Rc<RefCell<DataStore>>,
    ) -> PlotRenderer {
        let device = &engine.borrow().device;
        // let queue = &engine.borrow().queue;
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
                cull_mode: Some(wgpu::Face::Back),
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
            // engine: engine.clone(),
            // bind_group_layout,
        }
    }
}
// From: https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
// unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
//     ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
// }

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
// //log::info!("draw {},{}", x.x, x.y);
// let bytes: &[u8] = unsafe { any_as_u8_slice(&vertices) };
