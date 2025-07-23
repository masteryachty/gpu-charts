//! Grid and axes overlay renderer

use crate::{RenderContext, Viewport};
use gpu_charts_shared::{Result, VisualConfig};
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

/// Grid overlay renderer
pub struct GridOverlay {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    visual_config: VisualConfig,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GridUniforms {
    transform: [[f32; 4]; 4],
    grid_color: [f32; 4],
    axes_color: [f32; 4],
    viewport_size: [f32; 2],
    grid_spacing: [f32; 2],
}

impl GridOverlay {
    pub fn new(device: &wgpu::Device, visual_config: &VisualConfig) -> Result<Self> {
        console_log!("[GridOverlay] Creating grid overlay renderer");
        
        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Grid Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../chart_renderers/shaders/grid.wgsl").into()),
        });

        // Create uniform buffer
        let uniforms = GridUniforms {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            grid_color: visual_config.grid_color,
            axes_color: [0.8, 0.8, 0.8, 1.0], // Light gray for axes
            viewport_size: [1920.0, 1080.0],
            grid_spacing: [10.0, 10.0], // 10x10 grid
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grid Bind Group Layout"),
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
            label: Some("Grid Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // No vertex buffer, we generate vertices in shader
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
                topology: wgpu::PrimitiveTopology::TriangleStrip,
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

        Ok(Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            visual_config: visual_config.clone(),
        })
    }

    fn update_uniforms(&self, queue: &wgpu::Queue, viewport: &Viewport) {
        let uniforms = GridUniforms {
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            grid_color: self.visual_config.grid_color,
            axes_color: [0.8, 0.8, 0.8, 1.0],
            viewport_size: [viewport.width, viewport.height],
            grid_spacing: [10.0, 10.0],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }
}

impl super::OverlayRenderer for GridOverlay {
    fn render<'a>(&'a mut self, pass: &mut wgpu::RenderPass<'a>, context: &RenderContext) {
        if !context.visual_config.show_grid {
            return;
        }

        console_log!("[GridOverlay] Rendering grid");
        
        // Update uniforms
        self.update_uniforms(context.queue, &context.viewport);

        // Set pipeline and bind groups
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        // Draw full-screen quad (4 vertices for triangle strip)
        pass.draw(0..4, 0..1);
    }

    fn update_visual_config(&mut self, config: &VisualConfig) {
        self.visual_config = config.clone();
    }

    fn on_resize(&mut self, _width: u32, _height: u32) {
        // Grid adapts automatically via uniforms
    }

    fn get_draw_call_count(&self) -> u32 {
        1
    }
}