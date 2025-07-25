//! Example volume bar renderer that can be used with MultiRenderer
//! 
//! This demonstrates how to create custom renderers that integrate with the MultiRenderer system.

use std::rc::Rc;
use wgpu::{CommandEncoder, Device, Queue, TextureFormat, TextureView};
use wgpu::util::DeviceExt;

use data_manager::DataStore;
use crate::multi_renderer::MultiRenderable;

/// Renders volume bars for financial charts
/// 
/// This is an example renderer that shows how to create custom visualizations
/// that can be combined with other renderers using MultiRenderer.
pub struct VolumeBarRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    device: Rc<wgpu::Device>,
    format: TextureFormat,
}

impl VolumeBarRenderer {
    pub fn new(
        device: Rc<wgpu::Device>,
        _queue: Rc<wgpu::Queue>,
        format: TextureFormat,
    ) -> Self {
        // Create a simple shader for rendering volume bars
        let shader_source = r#"
struct Uniforms {
    x_range: vec2<f32>,
    y_range: vec2<f32>,
    bar_width: f32,
    base_y: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> volumes: array<vec2<f32>>; // x: timestamp, y: volume

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_idx: u32) -> VertexOutput {
    let bar_idx = vertex_idx / 6u;
    let vertex_in_bar = vertex_idx % 6u;
    
    let volume_data = volumes[bar_idx];
    let timestamp = volume_data.x;
    let volume = volume_data.y;
    
    // Normalize coordinates to clip space
    let x_norm = (timestamp - uniforms.x_range.x) / (uniforms.x_range.y - uniforms.x_range.x);
    let y_norm = (volume - uniforms.y_range.x) / (uniforms.y_range.y - uniforms.y_range.x);
    
    let x_pos = x_norm * 2.0 - 1.0;
    let y_top = uniforms.base_y + y_norm * 0.3; // Volume bars take up 30% of chart height
    let y_bottom = uniforms.base_y;
    
    let half_width = uniforms.bar_width * 0.5;
    
    var out: VertexOutput;
    
    // Create two triangles for a bar
    switch vertex_in_bar {
        case 0u: { out.position = vec4<f32>(x_pos - half_width, y_bottom, 0.0, 1.0); }
        case 1u: { out.position = vec4<f32>(x_pos + half_width, y_bottom, 0.0, 1.0); }
        case 2u: { out.position = vec4<f32>(x_pos - half_width, y_top, 0.0, 1.0); }
        case 3u: { out.position = vec4<f32>(x_pos - half_width, y_top, 0.0, 1.0); }
        case 4u: { out.position = vec4<f32>(x_pos + half_width, y_bottom, 0.0, 1.0); }
        case 5u: { out.position = vec4<f32>(x_pos + half_width, y_top, 0.0, 1.0); }
        default: { out.position = vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    }
    
    // Color based on volume intensity
    let intensity = y_norm;
    out.color = vec4<f32>(0.2, 0.6, 0.8, 0.7) * intensity + vec4<f32>(0.1, 0.2, 0.3, 0.5);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Volume Bar Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Volume Bar Bind Group Layout"),
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
            label: Some("Volume Bar Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Volume Bar Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            device,
            format,
        }
    }

    fn render_internal(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
    ) {
        // This is a simplified example - in a real implementation, you'd:
        // 1. Extract volume data from the DataStore
        // 2. Create GPU buffers for the volume data
        // 3. Render the volume bars
        
        let data_len = data_store.get_data_len();
        if data_len == 0 {
            return;
        }

        // Create mock volume data for demonstration
        let mock_volumes: Vec<[f32; 2]> = vec![
            [data_store.start_x as f32, 100.0],
            [((data_store.start_x + data_store.end_x) / 2) as f32, 250.0],
            [data_store.end_x as f32, 150.0],
        ];

        let volume_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Volume Data Buffer"),
            contents: bytemuck::cast_slice(&mock_volumes),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let uniforms = [
            data_store.start_x as f32,
            data_store.end_x as f32,
            data_store.min_y.unwrap_or(0.0),
            data_store.max_y.unwrap_or(300.0),
            0.02, // bar width in clip space
            -0.8, // base y position in clip space
        ];

        let uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Volume Bar Uniforms"),
            contents: bytemuck::cast_slice(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Volume Bar Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: volume_buffer.as_entire_binding(),
                },
            ],
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Volume Bar Render Pass"),
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
        render_pass.draw(0..(mock_volumes.len() as u32 * 6), 0..1);
    }
}

impl MultiRenderable for VolumeBarRenderer {
    fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        data_store: &DataStore,
        _device: &Device,
        _queue: &Queue,
    ) {
        self.render_internal(encoder, view, data_store);
    }

    fn name(&self) -> &str {
        "VolumeBarRenderer"
    }

    fn priority(&self) -> u32 {
        25 // Render volume bars in the background, before candles/lines
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        // Volume bars don't need special resize handling in this example
    }
}

/// Example of creating a custom renderer using the adapter pattern
pub fn create_custom_volume_renderer(
    device: Rc<Device>,
    queue: Rc<Queue>,
    format: TextureFormat,
) -> Box<dyn MultiRenderable> {
    let renderer = VolumeBarRenderer::new(device, queue, format);
    Box::new(renderer)
}