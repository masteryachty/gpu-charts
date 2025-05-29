use super::render_engine::RenderEngine;
use arrow::compute::kernels::length;
use bytemuck::{Pod, Zeroable};
use js_sys::{ArrayBuffer, Float32Array, Int32Array, Uint32Array, Uint8Array};
use nalgebra_glm::{vec4, Mat4};
use std::ptr;
use std::{cell::RefCell, rc::Rc};
use wgpu::util::DeviceExt;
use wgpu::{Buffer, Device, TextureFormat};

pub struct DataSeries {
    pub x_buffers: Vec<wgpu::Buffer>,
    pub y_buffers: Vec<wgpu::Buffer>,
    pub x_raw: ArrayBuffer,
    pub y_raw: ArrayBuffer,
    pub min_x: u32,
    pub max_x: u32,

    // pub min_max_buffer: Option<wgpu::Buffer>,
    length: u32,
}

pub struct DataStore {
    pub min_x: u32,
    pub max_x: u32,
    pub min_y: Option<f32>,
    pub max_y: Option<f32>,
    pub data_groups: Vec<DataSeries>,
    pub active_data_group_index: usize,
    pub range_bind_group: Option<wgpu::BindGroup>,
}

pub struct Coord {
    pub x: f32,
    pub y: f32,
}

impl DataStore {
    pub fn new() -> DataStore {
        DataStore {
            min_x: 0,
            max_x: 0,
            min_y: None,
            max_y: None,
            data_groups: Vec::new(),
            active_data_group_index: 0,
            range_bind_group: None,
        }
    }

    // pub fn add_data(&mut self, x: f32, y: f32) {
    //     self.data.push(Coord { x, y });
    // }

    pub fn add_data_group(
        &mut self,
        mut x_series: (ArrayBuffer, Vec<Buffer>),
        mut y_series: (ArrayBuffer, Vec<Buffer>),
        set_as_active: bool,
        start: u32,
        end: u32,
    ) {
        let f: Uint32Array = Uint32Array::new(&x_series.0);
        // let y: Float32Array = Float32Array::new(&y_series.0);
        // // Copy contents to a Rust Vec<u32>
        // let mut rust_vec = vec![0u32; f.length() as usize];
        // let mut rust_y_vec = vec![0f32; y.length() as usize];
        // f.copy_to(&mut rust_vec);
        // y.copy_to(&mut rust_y_vec);

        // // Now you can print the values
        // log::info!("x buffer: {:?}", rust_vec);
        // log::info!("y buffer: {:?}", rust_y_vec);
        // let pairs: Vec<_> = rust_vec.iter().zip(rust_y_vec.iter()).collect();
        // log::info!("(x, y) pairs: {:?}", pairs);
        self.data_groups.push(DataSeries {
            x_buffers: x_series.1,
            y_buffers: y_series.1,
            x_raw: x_series.0,
            y_raw: y_series.0,
            min_x: start,
            max_x: end,
            length: f.length(),
        });
        if (set_as_active) {
            self.active_data_group_index = self.data_groups.len() - 1;
        }
    }

    pub fn get_active_data_group(&self) -> &DataSeries {
        &self.data_groups[self.active_data_group_index]
    }

    pub fn get_data_len(&self) -> u32 {
        self.get_active_data_group().length
    }

    pub fn set_x_range(&mut self, min_x: u32, max_x: u32) {
        self.min_x = min_x;
        self.max_x = max_x;
        self.min_y = None;
        self.max_y = None;
        self.range_bind_group = None;
    }

    // pub fn get_data(&self) -> Uint8Array {
    //     self.data.cop
    // }
    // self.make_vertex_buffer(device, d)

    fn make_vertex_buffers(&self, device: &Device, data: Vec<&[u8]>) -> Vec<wgpu::Buffer> {
        data.iter()
            .map(|d| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Data Buffer"),
                    usage: wgpu::BufferUsages::VERTEX, // You can change this based on your needs
                    contents: &d,
                })
            })
            .collect()
    }

    pub fn world_to_screen_with_margin(&self, x: f32, y: f32) -> (f32, f32) {
        let data_group = self.get_active_data_group();
        log::info!("in  Y: {}, {} ", self.min_y.unwrap(), self.max_y.unwrap());

        // let projection = world_to_screen_conversion_with_margin2(
        //     data_group.min_x as f32,
        //     data_group.max_x as f32,
        //     self.min_y.unwrap(),
        //     self.max_y.unwrap(),
        //     -1.,
        //     1.,
        // );

        // let x_margin = ((data_group.max_x - data_group.min_x) as f32) * 0.0;
        // let y_margin = (self.max_y.unwrap() - self.min_y.unwrap()) * 0.0;

        let projection = glm::ortho_rh_zo(
            data_group.min_x as f32,
            data_group.max_x as f32,
            self.max_y.unwrap(),
            self.min_y.unwrap(),
            -1.0,
            1.0,
        );

        let pos = glm::vec4(x, y, 0.1, 1.);

        let result = projection * pos;
        (result.xy().x, result.xy().y)
    }

    pub fn update_buffers(&mut self, device: &Device, buffer_y: wgpu::Buffer) {
        let x_min_max = glm::vec2(self.min_x, self.max_x);
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
        self.min_y = Some(min_y);
        self.max_y = Some(max_y);
    }
}

// #[derive(Copy, Clone, Pod, Zeroable)]
// #[repr(C, packed)]
pub struct Vertex {
    // pub position: [f32; 2],
}

impl Vertex {
    pub fn get_x_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<[f32; 1]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0, // This corresponds to @location(0) in the shader
                format: wgpu::VertexFormat::Float32, // This matches vec2<f32> in your shader
            }],
        }
    }

    pub fn get_y_layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32];

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
