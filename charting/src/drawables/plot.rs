use std::cell::RefCell;
use std::rc::Rc;

use wgpu::TextureFormat;

use crate::renderer::data_store::{DataStore, Vertex};
use crate::renderer::render_engine::RenderEngine;
use crate::renderer::culling::CullingSystem;
use crate::renderer::vertex_compression::{ChartVertexCompression, CompressedChartVertex};
use crate::renderer::gpu_vertex_gen::{ChartGpuVertexGen, ChartVertexGenConfig};
use crate::renderer::render_bundles::{ChartRenderBundles, ChartRenderBundleConfig, BundleKey};

pub struct PlotRenderer {
    pipeline: wgpu::RenderPipeline,
    compressed_pipeline: Option<wgpu::RenderPipeline>,
    gpu_gen_pipeline: Option<wgpu::RenderPipeline>,
    bind_group_layout: wgpu::BindGroupLayout,
    culling_system: Option<Rc<RefCell<CullingSystem>>>,
    vertex_compression: Option<Rc<RefCell<ChartVertexCompression>>>,
    gpu_vertex_gen: Option<Rc<RefCell<ChartGpuVertexGen>>>,
    render_bundles: Option<Rc<RefCell<ChartRenderBundles>>>,
    use_compression: bool,
    use_gpu_gen: bool,
    use_render_bundles: bool,
    // pub engine: Rc<RefCell<RenderEngine>>,
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
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: Rc<RefCell<DataStore>>,
    ) {
        //log::info!("Render plot2");
        // let device = &self.engine.borrow().device;

        let data_len = data_store.borrow().get_data_len();
        // log::info!("Data len: {}", data_len);
        if data_len > 0 {
            // Advance frame for render bundles
            if let Some(bundles) = &self.render_bundles {
                bundles.borrow_mut().advance_frame();
            }
            
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
                
                // Choose pipeline based on rendering mode
                if self.use_gpu_gen && self.gpu_gen_pipeline.is_some() {
                    render_pass.set_pipeline(self.gpu_gen_pipeline.as_ref().unwrap());
                } else if self.use_compression && self.compressed_pipeline.is_some() {
                    render_pass.set_pipeline(self.compressed_pipeline.as_ref().unwrap());
                } else {
                    render_pass.set_pipeline(&self.pipeline);
                }

                // Render all visible metrics from all active data groups
                for (data_series, metric) in ds.get_all_visible_metrics() {
                    // Note: Render bundles are disabled in the initial implementation due to lifetime issues
                    // with WebGPU's render bundle API. The buffers and bind groups need to outlive the
                    // bundle encoder, which conflicts with our dynamic data management.
                    // This feature will be revisited with a different architecture in the future.
                    
                    // Normal rendering path
                    // Create a bind group for this specific metric with its color
                    let bind_group = self.create_bind_group_for_metric(device, &ds, metric);
                    render_pass.set_bind_group(0, &bind_group, &[]);

                    // Calculate visible range using culling system
                    let (start_idx, end_idx) = if let Some(culling) = &self.culling_system {
                        let culling_ref = culling.borrow();
                        culling_ref.calculate_visible_range(&ds, ds.start_x as u64, ds.end_x as u64)
                    } else {
                        // Fallback to rendering all data
                        (0, data_series.length as usize)
                    };

                    log::info!("Culling: rendering indices {} to {} out of {} total points", 
                              start_idx, end_idx, data_series.length);

                    if self.use_gpu_gen {
                        // Use GPU-generated vertices
                        if let Some(_gpu_gen) = &self.gpu_vertex_gen {
                            // GPU vertex generation needs to happen in a separate compute pass
                            // For now, we'll use pre-generated vertices
                            log::info!("GPU vertex generation mode active");
                            // The actual generation would happen before the render pass
                        }
                    } else if self.use_compression {
                        // Use compressed vertex buffers
                        if let Some(compression) = &self.vertex_compression {
                            let comp_ref = compression.borrow();
                            let compressed_buffer = comp_ref.get_compressed_buffer(&ds, data_series, metric);
                            render_pass.set_vertex_buffer(0, compressed_buffer.slice(..));
                            
                            // Calculate compressed buffer indices
                            let vertex_count = (end_idx - start_idx) as u32;
                            if vertex_count > 0 {
                                log::info!("Drawing compressed vertices: {} vertices", vertex_count);
                                render_pass.draw(0..vertex_count, 0..1);
                            }
                        }
                    } else {
                        // Use original vertex buffers
                        for (i, x_buffer) in data_series.x_buffers.iter().enumerate() {
                            if let Some(y_buffer) = metric.y_buffers.get(i) {
                                render_pass.set_vertex_buffer(0, x_buffer.slice(..));
                                render_pass.set_vertex_buffer(1, y_buffer.slice(..));
                                
                                // Calculate buffer-specific indices
                                let buffer_size = (x_buffer.size() / 4) as u32;
                                let buffer_start_idx = start_idx.min(buffer_size as usize) as u32;
                                let buffer_end_idx = end_idx.min(buffer_size as usize) as u32;
                                
                                if buffer_start_idx < buffer_end_idx {
                                    log::info!("Drawing buffer {}: indices {} to {} (size: {}, metric: {})", 
                                              i, buffer_start_idx, buffer_end_idx, buffer_size, metric.name);
                                    render_pass.draw(buffer_start_idx..buffer_end_idx, 0..1);
                                }
                            }
                        }
                    }
                }
            }
        }
        //log render
    }
}

impl PlotRenderer {
    fn create_bind_group_for_metric(
        &self,
        device: &wgpu::Device,
        data_store: &DataStore,
        metric: &crate::renderer::data_store::MetricSeries,
    ) -> wgpu::BindGroup {
        use wgpu::util::DeviceExt;

        // Create buffers for x_min_max, y_min_max, and color
        let x_min_max = glm::vec2(data_store.start_x, data_store.end_x);
        let x_min_max_bytes: &[u8] = unsafe { any_as_u8_slice(&x_min_max) };
        let x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("x_min_max buffer"),
            contents: x_min_max_bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let y_min_max = glm::vec2(
            data_store.min_y.unwrap_or(0.0),
            data_store.max_y.unwrap_or(1.0),
        );
        let y_min_max_bytes: &[u8] = unsafe { any_as_u8_slice(&y_min_max) };
        let y_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("y_min_max buffer"),
            contents: y_min_max_bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let color_bytes: &[u8] = unsafe { any_as_u8_slice(&metric.color) };
        let color_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("color buffer"),
            contents: color_bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Add compression parameters if using compression
        if self.use_compression {
            // For compressed pipeline, we need an additional binding for compression params
            let compression_params = if let Some(compression) = &self.vertex_compression {
                let comp_ref = compression.borrow();
                comp_ref.get_compression_params(data_store)
            } else {
                // Fallback params
                [data_store.start_x as f32, data_store.end_x as f32, 
                 data_store.min_y.unwrap_or(0.0), data_store.max_y.unwrap_or(1.0)]
            };
            
            let compression_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("compression_params buffer"),
                contents: unsafe { any_as_u8_slice(&compression_params) },
                usage: wgpu::BufferUsages::UNIFORM,
            });

            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("bind_group_compressed_{}", metric.name)),
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
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: compression_buffer.as_entire_binding(),
                    },
                ],
            })
        } else {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
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
    }

    pub fn new(
        engine: Rc<RefCell<RenderEngine>>,
        color_format: TextureFormat,
        _: Rc<RefCell<DataStore>>,
        culling_system: Option<Rc<RefCell<CullingSystem>>>,
    ) -> PlotRenderer {
        let device = &engine.borrow().device;
        // let queue = &engine.borrow().queue;
        let shader = device.create_shader_module(wgpu::include_wgsl!("plot.wgsl"));
        // Create bind group layout with optional compression params
        let mut bind_group_entries = vec![
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
        ];
        
        // Add compression params binding if needed
        if std::env::var("ENABLE_VERTEX_COMPRESSION").unwrap_or_default() == "1" {
            bind_group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }
        
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &bind_group_entries,
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
        
        // Create compressed pipeline if vertex compression is enabled
        let compressed_pipeline = if std::env::var("ENABLE_VERTEX_COMPRESSION").unwrap_or_default() == "1" {
            let compressed_shader = device.create_shader_module(wgpu::include_wgsl!("plot_compressed.wgsl"));
            
            let compressed_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Compressed Plot Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &compressed_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[CompressedChartVertex::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &compressed_shader,
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
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });
            
            Some(compressed_pipeline)
        } else {
            None
        };
        
        // Create GPU vertex generation pipeline if enabled
        let gpu_gen_pipeline = if std::env::var("ENABLE_GPU_VERTEX_GEN").unwrap_or_default() == "1" {
            let gpu_gen_shader = device.create_shader_module(wgpu::include_wgsl!("plot_gpu_gen.wgsl"));
            
            let gpu_gen_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("GPU Generated Plot Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &gpu_gen_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[ChartGpuVertexGen::get_vertex_layout()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &gpu_gen_shader,
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
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });
            
            Some(gpu_gen_pipeline)
        } else {
            None
        };
        
        Self {
            pipeline,
            compressed_pipeline,
            gpu_gen_pipeline,
            bind_group_layout,
            culling_system,
            vertex_compression: None,
            gpu_vertex_gen: None,
            render_bundles: None,
            use_compression: false,
            use_gpu_gen: false,
            use_render_bundles: false,
            // engine: engine.clone(),
        }
    }
    
    pub fn set_vertex_compression(&mut self, compression: Option<Rc<RefCell<ChartVertexCompression>>>) {
        self.vertex_compression = compression;
        self.use_compression = self.vertex_compression.is_some() && self.compressed_pipeline.is_some();
    }
    
    pub fn set_gpu_vertex_gen(&mut self, gpu_gen: Option<Rc<RefCell<ChartGpuVertexGen>>>) {
        self.gpu_vertex_gen = gpu_gen;
        self.use_gpu_gen = self.gpu_vertex_gen.is_some() && self.gpu_gen_pipeline.is_some();
    }
    
    pub fn set_render_bundles(&mut self, render_bundles: Option<Rc<RefCell<ChartRenderBundles>>>) {
        self.render_bundles = render_bundles;
        self.use_render_bundles = self.render_bundles.is_some();
    }
}

// From: https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
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
// //log::info!("draw {},{}", x.x, x.y);
// let bytes: &[u8] = unsafe { any_as_u8_slice(&vertices) };
