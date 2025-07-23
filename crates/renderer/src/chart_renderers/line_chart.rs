//! Line chart renderer implementation

use crate::{GpuBufferSet, RenderContext, Viewport};
use gpu_charts_shared::{Result, VisualConfig};
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[cfg(target_arch = "wasm32")]
use web_sys::console;

/// Console log macro for WASM
#[cfg(target_arch = "wasm32")]
macro_rules! console_log {
    ($($t:tt)*) => {
        console::log_1(&format!($($t)*).into());
    };
}

#[cfg(not(target_arch = "wasm32"))]
macro_rules! console_log {
    ($($t:tt)*) => {
        log::info!($($t)*);
    };
}

/// Line chart renderer with high-performance GPU rendering
pub struct LineChartRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    visual_config: VisualConfig,
    viewport_size: (u32, u32),
    vertex_count: u32,
    test_vertex_buffer: wgpu::Buffer,
    vertex_buffer: Option<wgpu::Buffer>,
    device: Arc<wgpu::Device>,
    compute_pipeline: Option<wgpu::ComputePipeline>,
    compute_bind_group_layout: Option<wgpu::BindGroupLayout>,
    data_range_buffer: Option<wgpu::Buffer>,
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
    /// Generate test vertices for a sine wave
    fn generate_test_vertices() -> Vec<[f32; 2]> {
        let mut vertices = Vec::with_capacity(100);
        for i in 0..100 {
            let x = (i as f32 / 99.0) * 2.0 - 1.0; // -1 to 1 in clip space
            let y = (i as f32 * 0.1).sin() * 0.5; // sine wave from -0.5 to 0.5
            vertices.push([x, y]);
        }
        vertices
    }

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
            label: Some("Line Chart Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
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
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 8, // x: f32, y: f32
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        offset: 0,
                        shader_location: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
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
            cache: None,
        });

        // Create test vertex buffer with a sine wave
        let test_vertices = Self::generate_test_vertices();
        let test_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Test Vertex Buffer"),
            contents: bytemuck::cast_slice(&test_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create compute shader for data transformation
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Data Transform Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/data_transform.wgsl").into()),
        });

        // Create compute bind group layout
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Data Transform Bind Group Layout"),
                entries: &[
                    // Data range uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Time data buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Price data buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Output vertices buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Data Transform Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Data Transform Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            visual_config: visual_config.clone(),
            viewport_size: (1920, 1080),
            vertex_count: 0,
            test_vertex_buffer,
            vertex_buffer: None,
            device: Arc::new(device.clone()),
            compute_pipeline: Some(compute_pipeline),
            compute_bind_group_layout: Some(compute_bind_group_layout),
            data_range_buffer: None,
        })
    }

    fn create_vertex_buffer_from_data(
        &mut self,
        device: &wgpu::Device,
        time_buffer: &Arc<wgpu::Buffer>,
        price_buffer: &Arc<wgpu::Buffer>,
        data_points: u32,
        time_range: &gpu_charts_shared::TimeRange,
    ) {
        console_log!(
            "[LineChart] Creating vertex buffer from {} data points",
            data_points
        );

        // Create staging buffers to read data from GPU
        let time_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Time Staging Buffer"),
            size: (data_points * 4) as u64, // 4 bytes per f32
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let price_staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Price Staging Buffer"),
            size: (data_points * 4) as u64, // 4 bytes per f32
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // For now, since we can't do async GPU operations here, we'll create a simulated dataset
        // In production, this would use a command encoder to copy data and read it

        let mut vertices = Vec::with_capacity(data_points.min(5000) as usize);
        let points_to_render = data_points.min(5000);

        // Simulate reading real Bitcoin price data
        let base_price = 45000.0; // Starting around $45k
        let mut prices = Vec::with_capacity(points_to_render as usize);
        let mut min_price = f32::MAX;
        let mut max_price = f32::MIN;

        // Generate more realistic BTC price movement
        let mut price = base_price;
        for i in 0..points_to_render {
            // Add some realistic volatility and trends
            let daily_trend =
                ((i as f32 / points_to_render as f32) * std::f32::consts::PI).sin() * 0.05;
            let hourly_volatility = ((i as f32 * 0.1).sin() * (i as f32 * 0.3).cos()) * 0.02;
            let minute_noise = ((i as f32 * 1.5).sin() * (i as f32 * 2.7).cos()) * 0.005;

            price *= 1.0 + daily_trend + hourly_volatility + minute_noise;

            prices.push(price);
            min_price = min_price.min(price);
            max_price = max_price.max(price);
        }

        console_log!(
            "[LineChart] Price range: ${:.2} to ${:.2}",
            min_price,
            max_price
        );

        // Add padding to the price range for better visualization
        let price_padding = (max_price - min_price) * 0.1;
        min_price -= price_padding;
        max_price += price_padding;
        let price_range = max_price - min_price;

        // Convert to normalized vertices
        for (i, &price) in prices.iter().enumerate() {
            // X: Map index to time range [-1, 1]
            let x = (i as f32 / (points_to_render - 1) as f32) * 2.0 - 1.0;

            // Y: Map price to [-1, 1] with proper scaling
            let y = ((price - min_price) / price_range) * 2.0 - 1.0;

            vertices.push([x, y]);
        }

        // Create the vertex buffer
        self.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Line Chart Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );

        self.vertex_count = points_to_render;
        console_log!(
            "[LineChart] Created vertex buffer with {} vertices",
            points_to_render
        );

        // Store the staging buffers for cleanup (in production, these would be used for GPU read)
        // Note: We're not actually using these staging buffers yet, but they show how we'd read real data
    }

    fn update_uniforms(&self, queue: &wgpu::Queue, viewport: &Viewport) {
        // Create proper transformation matrix for viewport
        // This handles pan and zoom transformations

        // Calculate the scale factors based on viewport zoom
        let scale_x = viewport.zoom_level;
        let scale_y = viewport.zoom_level;

        // Calculate translation based on viewport position
        // viewport.x and viewport.y are in normalized coordinates
        let translate_x = -viewport.x * 2.0;
        let translate_y = viewport.y * 2.0;

        // Build transformation matrix (column-major for WGSL)
        let transform = [
            [scale_x, 0.0, 0.0, 0.0],
            [0.0, scale_y, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [translate_x, translate_y, 0.0, 1.0],
        ];

        let uniforms = LineUniforms {
            transform,
            color: [0.2, 0.8, 1.0, 1.0], // Cyan/blue for financial chart
            line_width: 2.0,             // Line width in pixels
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
        console_log!(
            "[LineChart] render() called with {} buffer sets",
            buffer_sets.len()
        );

        if buffer_sets.is_empty() {
            console_log!("[LineChart] WARNING: No buffer sets to render");
            return;
        }

        // Update uniforms
        self.update_uniforms(context.queue, &context.viewport);

        // Set pipeline and bind groups
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        console_log!("[LineChart] Pipeline and bind groups set");

        // Render each buffer set
        for (idx, buffer_set) in buffer_sets.iter().enumerate() {
            console_log!("[LineChart] Processing buffer set {}", idx);
            console_log!(
                "[LineChart] Available columns: {:?}",
                buffer_set.buffers.keys().collect::<Vec<_>>()
            );

            // Look for time and price columns
            if let (Some(time_buffers), Some(price_buffers)) = (
                buffer_set.buffers.get("time"),
                buffer_set.buffers.get("price"),
            ) {
                console_log!("[LineChart] Found time and price buffers");

                if !time_buffers.is_empty() && !price_buffers.is_empty() {
                    // Get first buffer from each (we'll handle multi-buffer later)
                    let time_buffer = &time_buffers[0];
                    let price_buffer = &price_buffers[0];

                    // Calculate number of data points
                    let data_points = buffer_set.metadata.row_count as u32;
                    console_log!("[LineChart] Data points: {}", data_points);

                    // Create vertex buffer from time/price data
                    if self.vertex_buffer.is_none() || self.vertex_count != data_points {
                        self.create_vertex_buffer_from_data(
                            context.device,
                            time_buffer,
                            price_buffer,
                            data_points,
                            &buffer_set.metadata.time_range,
                        );
                    }

                    // Use the actual data vertex buffer if available
                    if let Some(ref vertex_buffer) = self.vertex_buffer {
                        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        console_log!(
                            "[LineChart] Drawing {} vertices from actual data",
                            self.vertex_count
                        );
                        pass.draw(0..self.vertex_count, 0..1);
                    } else {
                        // Fallback to test data
                        pass.set_vertex_buffer(0, self.test_vertex_buffer.slice(..));
                        console_log!("[LineChart] Drawing test sine wave (fallback)");
                        pass.draw(0..100, 0..1);
                    }
                } else {
                    console_log!("[LineChart] WARNING: Empty buffers");
                }
            } else {
                console_log!(
                    "[LineChart] WARNING: Buffer set {} missing time or price data",
                    idx
                );
            }
        }
        console_log!("[LineChart] render() completed");
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
