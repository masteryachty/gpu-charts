use std::cell::RefCell;
use std::rc::Rc;

use wgpu::{ TextureFormat};
use wgpu_text::glyph_brush::ab_glyph::FontRef;

use crate::renderer::data_retriever::create_gpu_buffer_from_vec_f32;
use crate::renderer::data_store::{DataStore};
use crate::renderer::render_engine::RenderEngine;
use wgpu_text::{
    glyph_brush::{Section as TextSection, Text},
    BrushBuilder, TextBrush,
};

use super::plot::RenderListener;

pub struct YAxisRenderer {
    // color_format: TextureFormat,
    brush: TextBrush<FontRef<'static>>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_count: u32,
    last_min_y: f32,
    last_max_y: f32,
    last_height: i32,
}

impl RenderListener for YAxisRenderer {
    fn on_render(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: Rc<RefCell<DataStore>>,
        size: (i32, i32),
    ) {
        let ds = data_store.borrow();
        let min = ds.min_y.unwrap();
        let max = ds.max_y.unwrap();
        // let range = max - min;
        let height = size.1;

        // Only recalculate and recreate buffers if the data range or width has changed
        let needs_recalculation = self.last_min_y != min as f32
            || self.last_max_y != max as f32
            || self.last_height != height;

        if needs_recalculation {
            log::info!(
                "Recalculating Y-axis with min: {}, max: {}, height: {}",
                min,
                max,
                height
            );

            let (interval, start, end) = calculate_y_axis_interval(min, max);
            let mut y_values = Vec::new();
            let mut labels = Vec::new();
            let mut label_strings = Vec::new();

            // Pre-allocate with estimated capacity
            let estimated_count = ((end - start) / interval) + 1.;
            y_values.reserve(estimated_count as usize);
            label_strings.reserve(estimated_count as usize);
            let mut y = start;
            // Collect yValues and prepare labels
            while y <= end {
                y_values.push(y);
                label_strings.push((y.to_string(), y));
                y += interval;
            }

            // Create text sections
            labels.reserve(label_strings.len());
            for (y_string, y) in &label_strings {
                // let h = (((y - min) as f64 / range as f64) * ((height as f64) * 0.8));
                let test = ds.world_to_screen_with_margin(0., *y);

                // let test = ds.world_to_screen_with_margin(0., 1.);
                log::info!("calculating Y: {}, {} ", y, test.1);

                let section = TextSection::default()
                    .add_text(Text::new(y_string))
                    .with_screen_position((
                        (5) as f32,
                        (((test.1 + 1.) / 2.) * (height as f32) - 15.),
                    ));
                labels.push(section);
            }

            // Create vertex data for axis lines
            let mut vertices = Vec::with_capacity(y_values.len() * 4);
            for y in y_values {
                vertices.push(-1.);
                vertices.push(y);
                vertices.push(1.);
                vertices.push(y);
            }

            // Create or update buffer
            self.vertex_count = (vertices.len() / 2) as u32;
            self.vertex_buffer = Some(create_gpu_buffer_from_vec_f32(
                device,
                &vertices,
                "y_axis_vertices",
            ));

            // Update cached values
            self.last_min_y = min as f32;
            self.last_max_y = max as f32;
            self.last_height = height;

            // Update text brush
            self.brush.resize_view(size.0 as f32, height as f32, queue);
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
            label: Some("y axis"),
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

impl YAxisRenderer {
    pub fn new(
        engine: Rc<RefCell<RenderEngine>>,
        color_format: TextureFormat,
        width: u32,
        height: u32,
        _data_store: Rc<RefCell<DataStore>>,
    ) -> Self {
        let device = &engine.borrow().device;

        // Create text brush
        let brush = BrushBuilder::using_font_bytes(include_bytes!("Roboto.ttf"))
            .unwrap()
            .build(device, width, height, color_format);

        // Create shader and pipeline
        let shader = device.create_shader_module(wgpu::include_wgsl!("y_axis.wgsl"));

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
            label: Some("Y-Axis Render Pipeline"),
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
            last_min_y: 0.0,
            last_max_y: 0.0,
            last_height: 0,
        }
    }
}

// Helper function to convert a struct to a u8 slice
// unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
//     ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
// }

/// Calculates Y-axis interval given a min and max value.
pub fn calculate_y_axis_interval(min: f32, max: f32) -> (f32, f32, f32) {
    let range = max - min;
    if range == 0.0 {
        return (1.0, min.floor(), min.ceil() + 1.0); // Default case for flat range
    }

    let target_intervals = 5.0;
    let raw_interval = range / target_intervals;

    let exponent = raw_interval.log10().floor();
    let base = 10f32.powf(exponent);
    let fraction = raw_interval / (base);

    let nice_fraction: f32 = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };

    let interval = nice_fraction * base;

    // Snap the axis start and end to the interval
    let start = (min / interval).floor() * interval;
    let end = (max / interval).ceil() * interval;

    (interval, start, end)
}

#[cfg(test)]
mod tests {
    use super::calculate_y_axis_interval;

    #[test]
    fn test_basic_range() {
        let (interval, start, end) = calculate_y_axis_interval(2.9, 7.1);
        assert_eq!(interval, 1.0);
        assert_eq!(start, 2.0);
        assert_eq!(end, 8.0);
    }

    #[test]
    fn test_small_range() {
        let (interval, start, end) = calculate_y_axis_interval(0.1, 0.9);
        assert_eq!(interval, 0.2);
        assert_eq!(start, 0.0);
        assert_eq!(end, 1.0);
    }

    #[test]
    fn test_large_range() {
        let (interval, start, end) = calculate_y_axis_interval(20.0, 100.0);
        assert_eq!(interval, 20.0);
        assert_eq!(start, 0.0);
        assert_eq!(end, 120.0);
    }

    #[test]
    fn test_tiny_range() {
        let (interval, start, end) = calculate_y_axis_interval(0.01, 0.015);
        assert_eq!(interval, 0.001);
        assert_eq!(start, 0.01);
        assert_eq!(end, 0.016);
    }

    #[test]
    fn test_zero_range() {
        let (interval, start, end) = calculate_y_axis_interval(5.0, 5.0);
        assert_eq!(interval, 1.0);
        assert_eq!(start, 5.0);
        assert_eq!(end, 6.0);
    }

    #[test]
    fn test_negative_to_positive_range() {
        let (interval, start, end) = calculate_y_axis_interval(-1.5, 2.5);
        assert_eq!(interval, 1.0);
        assert_eq!(start, -2.0);
        assert_eq!(end, 3.0);
    }
}
