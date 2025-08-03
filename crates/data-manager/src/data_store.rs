use config_system::{ChartPreset, ComputeOp};
use js_sys::{ArrayBuffer, Uint32Array};
use nalgebra_glm as glm;
use std::rc::Rc;
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
    pub name: String, // e.g., "best_bid", "best_ask", "mid_price"

    // Computed metric fields
    pub is_computed: bool,
    pub compute_type: Option<ComputeOp>,
    pub dependencies: Vec<MetricRef>,
    pub is_computed_ready: bool,
    pub compute_version: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MetricRef {
    pub group_index: usize,
    pub metric_index: usize,
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

#[derive(Debug, Clone, PartialEq)]
pub struct AppStoreState {
    pub start_time: u64,
    pub end_time: u64,
}

pub struct DataStore {
    pub preset: Option<ChartPreset>,
    pub symbol: Option<String>,
    pub start_x: u32,
    pub end_x: u32,
    pub data_groups: Vec<DataSeries>,
    pub active_data_group_indices: Vec<usize>, // Multiple active series
    pub range_bind_group: Option<wgpu::BindGroup>,
    pub screen_size: ScreenDimensions,
    dirty: bool, // Track if data has changed and needs re-rendering
    pub min_max_buffer: Option<Rc<wgpu::Buffer>>, // GPU-calculated min/max buffer
    pub gpu_min_y: Option<f32>, // GPU-calculated min Y value
    pub gpu_max_y: Option<f32>, // GPU-calculated max Y value
}

// pub struct Coord {
//     pub x: f32,
//     pub y: f32,
// }

impl DataStore {
    pub fn new(width: u32, height: u32, start_x: u32, end_x: u32) -> DataStore {
        DataStore {
            preset: None,
            symbol: None,
            start_x,
            end_x,
            data_groups: Vec::new(),
            active_data_group_indices: Vec::new(),
            range_bind_group: None,
            screen_size: ScreenDimensions { width, height },
            dirty: true,          // Start dirty to ensure initial render
            min_max_buffer: None, // GPU buffer will be created during rendering
            gpu_min_y: None,
            gpu_max_y: None,
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

        // Clear GPU min/max calculations when new data is added
        self.clear_gpu_bounds();
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
            data_group
                .metrics
                .push(MetricSeries::new(y_series.1, y_series.0, color, name));
        }

        self.mark_dirty();
    }

    /// Add a computed metric to a data group
    pub fn add_computed_metric_to_group(
        &mut self,
        group_index: usize,
        name: String,
        color: [f32; 3],
        compute_type: ComputeOp,
        dependencies: Vec<MetricRef>,
    ) {
        if let Some(data_group) = self.data_groups.get_mut(group_index) {
            data_group.metrics.push(MetricSeries::new_computed(
                name,
                color,
                compute_type,
                dependencies,
            ));
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

    /// Clear GPU bounds calculations (called when new data is loaded)
    pub fn clear_gpu_bounds(&mut self) {
        self.min_max_buffer = None;
        self.gpu_min_y = None;
        self.gpu_max_y = None;
        self.range_bind_group = None;
        self.mark_dirty();
    }

    pub fn set_x_range(&mut self, min_x: u32, max_x: u32) {
        if self.start_x != min_x || self.end_x != max_x {
            self.start_x = min_x;
            self.end_x = max_x;
            self.clear_gpu_bounds();
        }
    }

    /// Check if Y bounds are available
    pub fn has_y_bounds(&self) -> bool {
        self.gpu_min_y.is_some() && self.gpu_max_y.is_some()
    }

    /// Get Y bounds if available
    pub fn get_y_bounds(&self) -> Option<(f32, f32)> {
        match (self.gpu_min_y, self.gpu_max_y) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        if self.screen_size.width != width || self.screen_size.height != height {
            self.screen_size = ScreenDimensions { width, height };
            self.mark_dirty();
        }
    }

    pub fn world_to_screen_with_margin(&self, x: f32, y: f32) -> (f32, f32) {
        // Use default values if min/max Y are not set yet
        let min_y = self.gpu_min_y.unwrap_or(0.0);
        let max_y = self.gpu_max_y.unwrap_or(100.0);
        let y_range = max_y - min_y;

        let projection = glm::ortho_rh_zo(
            self.start_x as f32,
            self.end_x as f32,
            max_y + (y_range * 0.1),
            min_y - (y_range * 0.1),
            -1.0,
            1.0,
        );

        let pos = glm::vec4(x, y, 0.1, 1.);

        let result = projection * pos;
        (result.xy().x, result.xy().y)
    }

    /// Convert Y world coordinate to screen position for Y-axis labels
    pub fn y_to_screen_position(&self, y: f32) -> f32 {
        // Use default values if min/max Y are not set yet
        let min_y = self.gpu_min_y.unwrap_or(0.0);
        let max_y = self.gpu_max_y.unwrap_or(100.0);
        let y_range = max_y - min_y;

        // Add 10% margin to Y range
        let y_min_with_margin = min_y - (y_range * 0.1);
        let y_max_with_margin = max_y + (y_range * 0.1);

        // Normalize Y to [0, 1] range with margin
        let normalized_y = (y - y_min_with_margin) / (y_max_with_margin - y_min_with_margin);

        // Convert to screen coordinates (flip Y axis)
        (1.0 - normalized_y) * self.screen_size.height as f32
    }

    pub fn screen_to_world_with_margin(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        let min_x = self.start_x as f32;
        let max_x = self.end_x as f32;
        let min_y = self.gpu_min_y.unwrap_or(0.0);
        let max_y = self.gpu_max_y.unwrap_or(100.0);

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

        // let projection = glm::ortho(self.min_x, self.max_x, self.gpu_min_y, self.gpu_max_y, -1., 1.);
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

    /// Update the shared range bind group with GPU-calculated min/max buffer
    pub fn update_shared_bind_group_with_gpu_buffer(&mut self, device: &wgpu::Device) {
        use wgpu::util::DeviceExt;

        // Create x range buffer
        let x_min_max = glm::vec2(self.start_x as f32, self.end_x as f32);
        let x_min_max_bytes: &[u8] = unsafe { any_as_u8_slice(&x_min_max) };
        let x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shared_x_range_buffer"),
            contents: x_min_max_bytes,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Use the GPU-calculated min/max buffer if available
        let y_buffer = if let Some(gpu_min_max_buffer) = &self.min_max_buffer {
            // The GPU buffer already contains the min/max values
            // Just use the first 8 bytes (2 floats) for the overall min/max
            gpu_min_max_buffer.clone()
        } else {
            // Fallback to default values if GPU buffer not available
            let y_min_max = glm::vec2(0.0, 100.0);
            let y_min_max_bytes: &[u8] = unsafe { any_as_u8_slice(&y_min_max) };
            Rc::new(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("shared_y_range_buffer_fallback"),
                    contents: y_min_max_bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                }),
            )
        };

        // Create the bind group layout if it doesn't exist
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shared_range_bind_group_layout"),
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

        // Create the shared bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shared_range_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: x_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: y_buffer.as_entire_binding(),
                },
            ],
        });

        self.range_bind_group = Some(bind_group);
    }

    /// Get the GPU-calculated min/max values
    /// These are updated after GPU calculation completes
    pub fn get_gpu_y_bounds(&self) -> Option<(f32, f32)> {
        match (self.gpu_min_y, self.gpu_max_y) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    /// Update GPU-calculated Y bounds
    pub fn set_gpu_y_bounds(&mut self, min: f32, max: f32) {
        self.gpu_min_y = Some(min);
        self.gpu_max_y = Some(max);
        // Mark as dirty to trigger re-render with new bounds
        self.mark_dirty();
    }

    /// Get a metric by reference
    pub fn get_metric(&self, metric_ref: &MetricRef) -> Option<&MetricSeries> {
        self.data_groups
            .get(metric_ref.group_index)?
            .metrics
            .get(metric_ref.metric_index)
    }

    /// Get a mutable metric by reference
    pub fn get_metric_mut(&mut self, metric_ref: &MetricRef) -> Option<&mut MetricSeries> {
        self.data_groups
            .get_mut(metric_ref.group_index)?
            .metrics
            .get_mut(metric_ref.metric_index)
    }

    /// Find a metric by name and return its reference
    pub fn find_metric(&self, name: &str) -> Option<MetricRef> {
        for (group_idx, group) in self.data_groups.iter().enumerate() {
            for (metric_idx, metric) in group.metrics.iter().enumerate() {
                if metric.name == name {
                    return Some(MetricRef {
                        group_index: group_idx,
                        metric_index: metric_idx,
                    });
                }
            }
        }
        None
    }

    /// Check if dependencies for a computed metric are ready
    pub fn dependencies_ready(&self, metric: &MetricSeries) -> bool {
        for dep_ref in &metric.dependencies {
            if let Some(dep_metric) = self.get_metric(dep_ref) {
                if dep_metric.y_buffers.is_empty() {
                    return false;
                }
            } else {
                return false; // Dependency not found
            }
        }
        true
    }

    /// Get dependency buffers for computation
    pub fn get_dependency_buffers(&self, metric: &MetricSeries) -> Option<Vec<&wgpu::Buffer>> {
        let mut buffers = Vec::new();
        for dep_ref in &metric.dependencies {
            let dep_metric = self.get_metric(dep_ref)?;
            if dep_metric.y_buffers.is_empty() {
                return None;
            }
            buffers.push(&dep_metric.y_buffers[0]);
        }
        Some(buffers)
    }

    /// Get all computed metrics that need computation
    pub fn get_metrics_needing_computation(&self) -> Vec<MetricRef> {
        let mut refs = Vec::new();
        for (group_idx, group) in self.data_groups.iter().enumerate() {
            for (metric_idx, metric) in group.metrics.iter().enumerate() {
                if metric.needs_computation() {
                    refs.push(MetricRef {
                        group_index: group_idx,
                        metric_index: metric_idx,
                    });
                }
            }
        }
        refs
    }

    pub fn set_preset_and_symbol(
        &mut self,
        preset: Option<&ChartPreset>,
        symbol_name: Option<String>,
    ) {
        if let Some(p) = preset {
            self.preset = Some(p.clone());
        } else {
            self.preset = None;
        }
        self.symbol = symbol_name.clone();
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

impl MetricSeries {
    /// Create a regular metric from data
    pub fn new(
        y_buffers: Vec<wgpu::Buffer>,
        y_raw: ArrayBuffer,
        color: [f32; 3],
        name: String,
    ) -> Self {
        let is_ready = !y_buffers.is_empty();
        Self {
            y_buffers,
            y_raw,
            color,
            visible: true,
            name,
            is_computed: false,
            compute_type: None,
            dependencies: vec![],
            is_computed_ready: is_ready,
            compute_version: 0,
        }
    }

    /// Create a computed metric placeholder
    pub fn new_computed(
        name: String,
        color: [f32; 3],
        compute_type: ComputeOp,
        dependencies: Vec<MetricRef>,
    ) -> Self {
        Self {
            y_buffers: vec![],
            y_raw: ArrayBuffer::new(0),
            color,
            visible: true,
            name,
            is_computed: true,
            compute_type: Some(compute_type),
            dependencies,
            is_computed_ready: false,
            compute_version: 0,
        }
    }

    /// Check if this metric needs computation
    pub fn needs_computation(&self) -> bool {
        self.is_computed && !self.is_computed_ready
    }

    /// Set computed buffer and raw data
    pub fn set_computed_data(&mut self, buffer: wgpu::Buffer, computed_values: Vec<f32>) {
        self.y_buffers = vec![buffer];
        self.is_computed_ready = true;
        self.compute_version += 1;

        // Convert to ArrayBuffer for CPU access
        let js_array = js_sys::Float32Array::new_with_length(computed_values.len() as u32);
        for (i, &value) in computed_values.iter().enumerate() {
            js_array.set_index(i as u32, value);
        }
        self.y_raw = js_array.buffer();
    }

    /// Invalidate computation (when dependencies change)
    pub fn invalidate_computation(&mut self) {
        self.y_buffers.clear();
        self.is_computed_ready = false;
        self.y_raw = ArrayBuffer::new(0);
    }
}

// From: https://stackoverflow.com/questions/28127165/how-to-convert-struct-to-u8
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::core::slice::from_raw_parts((p as *const T) as *const u8, ::core::mem::size_of::<T>())
}
