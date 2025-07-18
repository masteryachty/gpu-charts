use js_sys::{ArrayBuffer, Uint32Array};
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device};

pub struct ScreenDimensions {
    pub width: u32,
    pub height: u32,
}

pub struct MetricSeries {
    pub y_buffers: Vec<wgpu::Buffer>,
    pub y_raw: ArrayBuffer, // Raw data for CPU access
    pub color: [f32; 3],
    pub visible: bool,
    pub name: String, // e.g., "best_bid", "best_ask"
}

pub struct DataSeries {
    pub x_buffers: Vec<wgpu::Buffer>, // Shared time axis
    pub x_raw: ArrayBuffer,
    pub metrics: Vec<MetricSeries>, // Multiple Y-series sharing same X
    pub length: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartType {
    Line,
    Candlestick,
}

pub struct DataStore {
    pub start_x: u32,
    pub end_x: u32,
    pub min_y: Option<f32>,
    pub max_y: Option<f32>,
    pub data_groups: Vec<DataSeries>,
    pub active_data_group_indices: Vec<usize>, // Multiple active series
    pub range_bind_group: Option<wgpu::BindGroup>,
    pub screen_size: ScreenDimensions,
    pub topic: Option<String>,
    pub chart_type: ChartType,
    pub candle_timeframe: u32, // in seconds
    dirty: bool, // Track if data has changed and needs re-rendering
}

// pub struct Coord {
//     pub x: f32,
//     pub y: f32,
// }

impl DataStore {
    pub fn new(width: u32, height: u32) -> DataStore {
        DataStore {
            start_x: 0,
            end_x: 0,
            min_y: None,
            max_y: None,
            data_groups: Vec::new(),
            active_data_group_indices: Vec::new(),
            range_bind_group: None,
            screen_size: ScreenDimensions { width, height },
            topic: None,
            chart_type: ChartType::Candlestick,
            candle_timeframe: 60, // Default 1 minute
            dirty: true, // Start dirty to ensure initial render
        }
    }

    /// Check if the data store needs re-rendering
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the data store as clean (rendered)
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Mark the data store as dirty (needs re-rendering)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    // pub fn add_data(&mut self, x: f32, y: f32) {
    //     self.data.push(Coord { x, y });
    // }

    pub fn add_data_group(&mut self, x_series: (ArrayBuffer, Vec<Buffer>), set_as_active: bool) {
        let f: Uint32Array = Uint32Array::new(&x_series.0);

        self.data_groups.push(DataSeries {
            x_buffers: x_series.1,
            x_raw: x_series.0,
            metrics: Vec::new(),
            length: f.length(),
        });

        if set_as_active {
            let new_index = self.data_groups.len() - 1;
            if !self.active_data_group_indices.contains(&new_index) {
                self.active_data_group_indices.push(new_index);
            }
        }
        
        self.mark_dirty();
    }

    pub fn add_metric_to_group(
        &mut self,
        group_index: usize,
        y_series: (ArrayBuffer, Vec<Buffer>),
        color: [f32; 3],
        name: String,
    ) {
        if let Some(data_group) = self.data_groups.get_mut(group_index) {
            data_group.metrics.push(MetricSeries {
                y_buffers: y_series.1,
                y_raw: y_series.0,
                color,
                visible: true,
                name,
            });
        }
        
        self.mark_dirty();
    }

    pub fn get_active_data_groups(&self) -> Vec<&DataSeries> {
        self.active_data_group_indices
            .iter()
            .filter_map(|&index| self.data_groups.get(index))
            .collect()
    }

    pub fn get_active_data_group(&self) -> Option<&DataSeries> {
        self.get_active_data_groups().first().copied()
    }

    pub fn get_data_len(&self) -> u32 {
        self.get_active_data_groups()
            .iter()
            .map(|group| group.length)
            .max()
            .unwrap_or(0)
    }

    pub fn get_all_visible_metrics(&self) -> Vec<(&DataSeries, &MetricSeries)> {
        self.get_active_data_groups()
            .into_iter()
            .flat_map(|data_series| {
                data_series
                    .metrics
                    .iter()
                    .filter(|metric| metric.visible)
                    .map(move |metric| (data_series, metric))
            })
            .collect()
    }

    pub fn set_x_range(&mut self, min_x: u32, max_x: u32) {
        if self.start_x != min_x || self.end_x != max_x {
            self.start_x = min_x;
            self.end_x = max_x;
            self.min_y = None;
            self.max_y = None;
            self.range_bind_group = None;
            self.mark_dirty();
        }
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        if self.screen_size.width != width || self.screen_size.height != height {
            self.screen_size = ScreenDimensions { width, height };
            self.mark_dirty();
        }
    }

    // pub fn get_data(&self) -> Uint8Array {
    //     self.data.cop
    // }
    // self.make_vertex_buffer(device, d)

    // fn make_vertex_buffers(&self, device: &Device, data: Vec<&[u8]>) -> Vec<wgpu::Buffer> {
    //     data.iter()
    //         .map(|d| {
    //             device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    //                 label: Some("Data Buffer"),
    //                 usage: wgpu::BufferUsages::VERTEX, // You can change this based on your needs
    //                 contents: &d,
    //             })
    //         })
    //         .collect()
    // }

    pub fn world_to_screen_with_margin(&self, x: f32, y: f32) -> (f32, f32) {
        // let data_group = self.get_active_data_group();

        let projection = glm::ortho_rh_zo(
            self.start_x as f32,
            self.end_x as f32,
            self.max_y.unwrap() + ((self.max_y.unwrap() - self.min_y.unwrap()) * 0.1),
            self.min_y.unwrap() - ((self.max_y.unwrap() - self.min_y.unwrap()) * 0.1),
            -1.0,
            1.0,
        );

        let pos = glm::vec4(x, y, 0.1, 1.);

        let result = projection * pos;
        (result.xy().x, result.xy().y)
    }

    pub fn screen_to_world_with_margin(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        log::info!(
            "conv: {:?} {:?} {:?} {:?}",
            screen_x,
            screen_y,
            self.screen_size.width,
            self.screen_size.height
        );

        let min_x = self.start_x as f32;
        let max_x = self.end_x as f32;
        let max_y = self.max_y.unwrap();
        let min_y = self.min_y.unwrap();

        let y_margin = (max_y - min_y) * 0.1;

        let top = max_y + y_margin;
        let bottom = min_y - y_margin;

        // Step 1: Create the projection matrix
        let projection = glm::ortho_rh_zo(min_x, max_x, top, bottom, -1.0, 1.0);

        // Step 2: Invert the matrix
        let inv_projection = projection
            .try_inverse()
            .expect("Projection matrix should be invertible");

        // Step 3: Convert from screen pixels to NDC (-1 to 1)
        let ndc_x = (2.0 * screen_x / (self.screen_size.width as f32)) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y / (self.screen_size.height as f32)); // Y-flipped

        let screen_pos = glm::vec4(ndc_x, ndc_y, 0.1, 1.0);

        // Step 4: Apply inverse projection
        let world_pos = inv_projection * screen_pos;

        (world_pos.x, world_pos.y)
    }

    pub fn update_buffers(&mut self, device: &Device, buffer_y: wgpu::Buffer) {
        let x_min_max = glm::vec2(self.start_x, self.end_x);
        let x_min_max_bytes: &[u8] = unsafe { any_as_u8_slice(&x_min_max) };

        let view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("x_min_max buffer"),
            contents: x_min_max_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // let projection = glm::ortho(self.min_x, self.max_x, self.min_y, self.max_y, -1., 1.);
        // let projection_bytes: &[u8] = unsafe { any_as_u8_slice(&projection) };
        // let projection_buffer_descriptor = wgpu::util::BufferInitDescriptor {
        //     label: Some("projection buffer"),
        //     contents: projection_bytes,
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // };
        // let projection_buffer = device.create_buffer_init(&projection_buffer_descriptor);

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
        // Borrow data_store immutably to get the data length
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: view_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer_y.as_entire_binding(),
                },
            ],
        });
        self.range_bind_group = Some(bind_group);
    }

    pub fn update_min_max_y(&mut self, min_y: f32, max_y: f32) {
        let changed = self.min_y != Some(min_y) || self.max_y != Some(max_y);
        if changed {
            self.min_y = Some(min_y);
            self.max_y = Some(max_y);
            self.mark_dirty();
        }
    }

    pub fn set_chart_type(&mut self, chart_type: ChartType) {
        if self.chart_type != chart_type {
            self.chart_type = chart_type;
            self.mark_dirty();
        }
    }

    pub fn set_candle_timeframe(&mut self, timeframe_seconds: u32) {
        if self.candle_timeframe != timeframe_seconds {
            self.candle_timeframe = timeframe_seconds;
            self.mark_dirty();
        }
    }
}

// #[derive(Copy, Clone, Pod, Zeroable)]
// #[repr(C, packed)]
pub struct Vertex {
    // pub position: [f32; 2],
}

impl Vertex {
    pub fn get_x_layout() -> wgpu::VertexBufferLayout<'static> {
        // const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[u32; 1]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0, // This corresponds to @location(0) in the shader
                format: wgpu::VertexFormat::Uint32, // This matches vec2<f32> in your shader
            }],
        }
    }

    pub fn get_y_layout() -> wgpu::VertexBufferLayout<'static> {
        // const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 1]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1, // This corresponds to @location(0) in the shader
                format: wgpu::VertexFormat::Float32, // This matches vec2<f32> in your shader
            }],
        }
    }
}

// From: https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}
