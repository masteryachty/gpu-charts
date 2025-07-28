//! Line renderer for computed data (e.g., mid price, moving averages)

use crate::compute::{ComputeResult, MidPriceCalculator};
use data_manager::DataStore;
use std::rc::Rc;
use wgpu::util::DeviceExt;

/// Renders lines from computed data
pub struct ComputedLineRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,

    // Compute infrastructure
    mid_price_calculator: Option<MidPriceCalculator>,

    // Cached results
    cached_result: Option<ComputeResult>,
    cached_data_version: u64,

    // Configuration
    name: String,
    color: [f32; 3],
    line_style: LineStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

impl ComputedLineRenderer {
    pub fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        color_format: wgpu::TextureFormat,
        name: String,
        color: [f32; 3],
    ) -> Self {
        // Create shader module
        let shader = device.create_shader_module(wgpu::include_wgsl!("computed_line.wgsl"));

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Computed Line Bind Group Layout"),
            entries: &[
                // Time buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Computed value buffer
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
                // X range
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
                // Y range
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
                // Line params (color, style, etc.)
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Computed Line Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Computed Line Render Pipeline"),
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
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // Create mid price calculator
        let mid_price_calculator = MidPriceCalculator::new(device.clone(), queue.clone()).ok();

        Self {
            pipeline,
            bind_group_layout,
            device,
            queue,
            mid_price_calculator,
            cached_result: None,
            cached_data_version: 0,
            name,
            color,
            line_style: LineStyle::Solid,
        }
    }

    pub fn set_line_style(&mut self, style: LineStyle) {
        self.line_style = style;
    }

    /// Compute mid price from bid/ask data
    fn compute_mid_price(
        &mut self,
        data_store: &DataStore,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Option<ComputeResult> {
        // Find bid and ask metrics
        let mut bid_buffer = None;
        let mut ask_buffer = None;
        let mut element_count = 0u32;

        for data_group in &data_store.data_groups {
            for metric in &data_group.metrics {
                if metric.name == "best_bid" && !metric.y_buffers.is_empty() {
                    bid_buffer = Some(&metric.y_buffers[0]);
                    element_count = data_group.length;
                } else if metric.name == "best_ask" && !metric.y_buffers.is_empty() {
                    ask_buffer = Some(&metric.y_buffers[0]);
                }
            }
        }

        // If we have both bid and ask, compute mid price
        if let (Some(bid), Some(ask), Some(calculator)) =
            (bid_buffer, ask_buffer, &self.mid_price_calculator)
        {
            match calculator.calculate(bid, ask, element_count, encoder) {
                Ok(result) => {
                    log::info!(
                        "📊 [ComputedLineRenderer] Computed mid price for {} elements",
                        element_count
                    );
                    Some(result)
                }
                Err(e) => {
                    log::error!(
                        "❌ [ComputedLineRenderer] Failed to compute mid price: {}",
                        e
                    );
                    None
                }
            }
        } else {
            log::warn!("⚠️ [ComputedLineRenderer] Missing bid/ask data or calculator for mid price computation");
            None
        }
    }
}

impl crate::MultiRenderable for ComputedLineRenderer {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: &DataStore,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        // Check if we need to recompute
        let current_version = data_store.data_groups.len() as u64; // Simple version check
        if self.cached_result.is_none() || self.cached_data_version != current_version {
            // Compute mid price
            if let Some(result) = self.compute_mid_price(data_store, encoder) {
                self.cached_result = Some(result);
                self.cached_data_version = current_version;
            } else {
                return; // Can't render without data
            }
        }

        // Get computed result
        let computed_result = match &self.cached_result {
            Some(result) => result,
            None => return,
        };

        // Find time buffer
        let time_buffer = data_store
            .data_groups
            .first()
            .and_then(|group| group.x_buffers.first());

        if time_buffer.is_none() {
            return;
        }

        let time_buffer = time_buffer.unwrap();

        // Create uniforms
        use nalgebra_glm as glm;

        let x_range = glm::vec2(data_store.start_x as f32, data_store.end_x as f32);
        let x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("X Range Buffer"),
            contents: bytemuck::cast_slice(&[x_range.x, x_range.y]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let y_range = glm::vec2(
            data_store.min_y.unwrap_or(0.0),
            data_store.max_y.unwrap_or(100.0),
        );
        let y_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Y Range Buffer"),
            contents: bytemuck::cast_slice(&[y_range.x, y_range.y]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Line parameters
        #[repr(C)]
        #[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct LineParams {
            color: [f32; 3],
            style: u32, // 0=solid, 1=dashed, 2=dotted
        }

        let line_params = LineParams {
            color: self.color,
            style: match self.line_style {
                LineStyle::Solid => 0,
                LineStyle::Dashed => 1,
                LineStyle::Dotted => 2,
            },
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Params Buffer"),
            contents: bytemuck::cast_slice(&[line_params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Computed Line Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: time_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: computed_result.output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: x_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: y_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Render
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Computed Line Render Pass"),
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
        render_pass.draw(0..computed_result.element_count, 0..1);
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn priority(&self) -> u32 {
        100 // Render after main data but before UI elements
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        // No special resize handling needed
    }
}
