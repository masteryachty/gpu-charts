use std::cell::RefCell;
use std::rc::Rc;

use wgpu::TextureFormat;
use wgpu_text::glyph_brush::ab_glyph::FontRef;

use crate::render_engine::RenderEngine;
use data_manager::{create_gpu_buffer_from_vec, DataStore};
use wgpu_text::{
    glyph_brush::{Section as TextSection, Text},
    BrushBuilder, TextBrush,
};

use super::plot::RenderListener;

pub struct XAxisRenderer {
    // color_format: TextureFormat,
    brush: TextBrush<FontRef<'static>>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_count: u32,
    last_min_x: f32,
    last_max_x: f32,
    last_width: i32,
}

const DURATION_SEC: i32 = 1;
const DURATION_MIN: i32 = DURATION_SEC * 60;
const DURATION_HOUR: i32 = DURATION_MIN * 60;
const DURATION_DAY: i32 = DURATION_HOUR * 24;

// 1, 5, 10, 50, 100, 500,
const LOGIC_TS_DURATIONS: [i32; 12] = [
    DURATION_SEC,
    5 * DURATION_SEC,
    10 * DURATION_SEC,
    15 * DURATION_SEC,
    DURATION_MIN,
    5 * DURATION_MIN,
    10 * DURATION_MIN,
    15 * DURATION_MIN,
    DURATION_HOUR,
    6 * DURATION_HOUR,
    DURATION_DAY,
    DURATION_DAY * 7,
];

impl RenderListener for XAxisRenderer {
    fn on_render(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: Rc<RefCell<DataStore>>,
    ) {
        let ds = data_store.borrow();
        // let data_group = ds.get_active_data_group();

        let min = ds.start_x as i32;
        let max = ds.end_x as i32;
        let range = max - min;
        let width = data_store.borrow().screen_size.width as i32;

        // Only recalculate and recreate buffers if the data range or width has changed
        let needs_recalculation = self.last_min_x != min as f32
            || self.last_max_x != max as f32
            || self.last_width != width;

        if needs_recalculation {
            log::info!("Recalculating X-axis with min: {min}, max: {max}");

            // Find appropriate time unit for axis labels
            let mut base_unit = 0;
            for i in LOGIC_TS_DURATIONS.iter() {
                if base_unit == 0 && (*i as f32 / range as f32) * width as f32 > 150.0 {
                    base_unit = *i;
                    break;
                }
            }

            if base_unit == 0 {
                base_unit = *LOGIC_TS_DURATIONS.last().unwrap();
            }
            log::info!("Using base_unit: {base_unit}");

            let interval = 1;
            let mut timestamps = Vec::new();
            let mut labels = Vec::new();
            let mut label_strings = Vec::new();

            // Pre-allocate with estimated capacity
            let estimated_count = (range / base_unit) + 1;
            timestamps.reserve(estimated_count as usize);
            label_strings.reserve(estimated_count as usize);

            // Collect timestamps and prepare labels
            for i in (0..=(range / base_unit)).rev() {
                let ts = (max / base_unit - i) * base_unit;

                if ts % (base_unit * interval) == 0 {
                    timestamps.push(ts);

                    // Format timestamp only when needed
                    if let Some(dt) = chrono::DateTime::from_timestamp(ts as i64, 0) {
                        let dt_str = dt.to_string();
                        let ts_string = format!("{}\n{}", &dt_str[0..10], &dt_str[11..]);
                        label_strings.push((ts_string, ts));
                    }
                }
            }

            // Create text sections
            labels.reserve(label_strings.len());
            for (ts_string, ts) in &label_strings {
                let test = ds.world_to_screen_with_margin(*ts as f32, 0.);

                let section = TextSection::default()
                    .add_text(Text::new(ts_string).with_color([1.0, 1.0, 1.0, 1.0]))
                    .with_screen_position((
                        (((test.0 + 1.) / 2.) * (width as f32)),
                        (data_store.borrow().screen_size.height - 50) as f32,
                    ));
                labels.push(section);
            }

            // Create vertex data for axis lines
            let mut vertices = Vec::with_capacity(timestamps.len() * 4);
            for timestamp in timestamps {
                vertices.push((timestamp - min) as f32);
                vertices.push(-1.);
                vertices.push((timestamp - min) as f32);
                vertices.push(1.);
            }

            // Create or update buffer
            self.vertex_count = (vertices.len() / 2) as u32;
            self.vertex_buffer = Some(create_gpu_buffer_from_vec(
                device,
                &vertices,
                "x_axis_vertices",
            ));

            // Update cached values
            self.last_min_x = min as f32;
            self.last_max_x = max as f32;
            self.last_width = width;

            // Update text brush
            self.brush.resize_view(
                data_store.borrow().screen_size.width as f32,
                data_store.borrow().screen_size.height as f32,
                queue,
            );
            self.brush.queue(device, queue, labels).unwrap();
        } else {
            // If only the window size changed, update the text brush size
            // if self.brush.resize_view(size.0 as f32, size.1 as f32, queue) {
            //     // Recompute labels if view size changed
            //     let mut labels = Vec::new();

            //     let base_unit = self.determine_base_unit(range, width);
            //     let interval = 1;

            //     for i in (0..=(range / base_unit)).rev() {
            //         let ts = (max / base_unit - i) * base_unit;

            //         if ts % (base_unit * interval) == 0 {
            //             if let Some(dt) = chrono::DateTime::from_timestamp(ts as i64, 0) {
            //                 let dt_str = dt.to_string();
            //                 let ts_string = format!("{}\n{}", &dt_str[0..10], &dt_str[11..]);

            //                 let w = (((ts - min) as f64 / range as f64) * width as f64) - 45.0;
            //                 let section = TextSection::default()
            //                     .add_text(Text::new(&ts_string))
            //                     .with_screen_position((w as f32, (size.1 - 50) as f32));
            //                 labels.push(section);
            //             }
            //         }
            //     }

            //     self.brush.queue(device, queue, labels).unwrap();
            // }
        }

        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("x axis"),
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

        // Draw vertical lines
        if let Some(buffer) = &self.vertex_buffer {
            let bind_group = ds.range_bind_group.as_ref().unwrap();

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, bind_group, &[]);
            render_pass.set_vertex_buffer(0, buffer.slice(..));
            render_pass.draw(0..self.vertex_count, 0..1);
        }

        // Draw text labels
        self.brush.draw(&mut render_pass);
    }
}

impl XAxisRenderer {
    pub fn new(
        engine: Rc<RefCell<RenderEngine>>,
        color_format: TextureFormat,
        data_store: Rc<RefCell<DataStore>>,
    ) -> Self {
        let device = &engine.borrow().device;

        // Create text brush
        let brush = BrushBuilder::using_font_bytes(include_bytes!("Roboto.ttf"))
            .unwrap()
            .build(
                device,
                data_store.borrow().screen_size.width,
                data_store.borrow().screen_size.height,
                color_format,
            );

        // Create shader and pipeline
        let shader = device.create_shader_module(wgpu::include_wgsl!("x_axis.wgsl"));

        const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];
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

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("X-Axis Render Pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ATTRIBUTES,
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(color_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
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
            // color_format,
            brush,
            pipeline,
            vertex_buffer: None,
            vertex_count: 0,
            last_min_x: 0.0,
            last_max_x: 0.0,
            last_width: 0,
        }
    }

    // Helper method to determine the appropriate base unit
    // fn determine_base_unit(&self, range: i32, width: i32) -> i32 {
    //     for i in LOGIC_TS_DURATIONS.iter() {
    //         if (*i as f32 / range as f32) * width as f32 > 150.0 {
    //             return *i;
    //         }
    //     }
    //     *LOGIC_TS_DURATIONS.last().unwrap()
    // }
}

// Helper function to convert a struct to a u8 slice
// unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
//     ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
// }
