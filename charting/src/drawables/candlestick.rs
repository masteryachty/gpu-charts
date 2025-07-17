use std::cell::RefCell;
use std::rc::Rc;

use wgpu::TextureFormat;
use wgpu::util::DeviceExt;

use crate::calcables::OhlcData;
use crate::renderer::data_store::DataStore;
use crate::renderer::render_engine::RenderEngine;

use super::plot::RenderListener;

/// Renders candlestick charts for financial data visualization.
/// 
/// This renderer aggregates tick data into OHLC (Open, High, Low, Close) candles
/// based on a configurable time frame, then renders them using WebGPU.
/// 
/// Features:
/// - Configurable candle timeframe (e.g., 1 minute, 5 minutes, etc.)
/// - Automatic OHLC aggregation from tick data
/// - Partial candle rendering at view edges
/// - Separate rendering passes for bodies and wicks
/// - Color coding: green for bullish, red for bearish, yellow for doji
pub struct CandlestickRenderer {
    body_pipeline: wgpu::RenderPipeline,
    wick_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    candle_timeframe: u32, // in seconds
    last_aggregation_time: u32,
    ohlc_data: Vec<OhlcData>,
    body_vertex_buffer: Option<wgpu::Buffer>,
    wick_vertex_buffer: Option<wgpu::Buffer>,
    body_vertex_count: u32,
    wick_vertex_count: u32,
}

impl RenderListener for CandlestickRenderer {
    fn on_render(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: Rc<RefCell<DataStore>>,
    ) {
        let ds = data_store.borrow();
        
        // Check if we have data
        let data_len = ds.get_data_len();
        if data_len == 0 {
            return;
        }
        
        // Update timeframe from DataStore
        self.candle_timeframe = ds.candle_timeframe;
        
        // Check if we need to re-aggregate (data changed or timeframe changed)
        let current_time_range = (ds.start_x, ds.end_x);
        let needs_reaggregation = self.last_aggregation_time != current_time_range.1;
        
        if needs_reaggregation {
            self.aggregate_ohlc(device, queue, &ds);
            self.last_aggregation_time = current_time_range.1;
        }
        
        // Only render if we have OHLC data
        if self.ohlc_data.is_empty() {
            return;
        }
        
        // Begin render pass
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Candlestick Render Pass"),
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
        
        // Create bind group for rendering
        let bind_group = self.create_bind_group(device, &ds);
        
        // Render candle bodies
        if let Some(body_buffer) = &self.body_vertex_buffer {
            render_pass.set_pipeline(&self.body_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, body_buffer.slice(..));
            render_pass.draw(0..self.body_vertex_count, 0..1);
        }
        
        // Render wicks
        if let Some(wick_buffer) = &self.wick_vertex_buffer {
            render_pass.set_pipeline(&self.wick_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, wick_buffer.slice(..));
            render_pass.draw(0..self.wick_vertex_count, 0..1);
        }
    }
}

impl CandlestickRenderer {
    pub fn new(
        engine: Rc<RefCell<RenderEngine>>,
        color_format: TextureFormat,
        _data_store: Rc<RefCell<DataStore>>,
    ) -> Self {
        let device = &engine.borrow().device;
        
        
        // Create shader modules
        let shader = device.create_shader_module(wgpu::include_wgsl!("candlestick.wgsl"));
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Candlestick Bind Group Layout"),
            entries: &[
                // X range (min/max timestamps)
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
                // Y range (price min/max)
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
                // Candle timeframe
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
            ],
        });
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Candlestick Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Vertex layout for candlestick data - using u32 for timestamp to avoid precision loss
        const CANDLE_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
            0 => Uint32,     // timestamp (u32 to avoid precision loss)
            1 => Float32,    // open
            2 => Float32,    // high
            3 => Float32,    // low
            4 => Float32,    // close
        ];
        
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<u32>() + std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &CANDLE_VERTEX_ATTRIBUTES,
        };
        
        // Create body rendering pipeline (filled rectangles)
        let body_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Candlestick Body Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_body"),
                compilation_options: Default::default(),
                buffers: &[vertex_buffer_layout.clone()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_candle"),
                compilation_options: Default::default(),
                targets: &[Some(color_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
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
        
        // Create wick rendering pipeline (lines)
        let wick_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Candlestick Wick Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_wick"),
                compilation_options: Default::default(),
                buffers: &[vertex_buffer_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_wick"),
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
            body_pipeline,
            wick_pipeline,
            bind_group_layout,
            candle_timeframe: 60, // Default 1 minute
            last_aggregation_time: 0,
            ohlc_data: Vec::new(),
            body_vertex_buffer: None,
            wick_vertex_buffer: None,
            body_vertex_count: 0,
            wick_vertex_count: 0,
        }
    }
    
    
    /// Aggregates tick data into OHLC candles.
    /// 
    /// This method:
    /// 1. Calculates candle boundaries to include partial candles at view edges
    /// 2. Uses binary search to efficiently find relevant ticks for each candle
    /// 3. Computes OHLC values for each time period
    /// 4. Creates GPU vertex buffers for rendering
    fn aggregate_ohlc(&mut self, device: &wgpu::Device, _queue: &wgpu::Queue, ds: &DataStore) {
        // Calculate candle boundaries to include partial candles
        // Find the first candle that starts at or before the view start
        let first_candle_start = (ds.start_x / self.candle_timeframe) * self.candle_timeframe;
        
        // Find the last candle that ends at or after the view end
        let last_candle_end = ((ds.end_x + self.candle_timeframe - 1) / self.candle_timeframe) * self.candle_timeframe;
        
        let extended_time_range = last_candle_end - first_candle_start;
        let num_candles = (extended_time_range / self.candle_timeframe) as usize;
        
        
        self.ohlc_data.clear();
        
        // Get the active data groups
        let active_groups = ds.get_active_data_groups();
        if active_groups.is_empty() {
            return;
        }
        
        // Use the first data group and first metric (price data)
        let data_series = &active_groups[0];
        if data_series.metrics.is_empty() {
            return;
        }
        
        // Access the time and price data
        use js_sys::Uint32Array;
        use js_sys::Float32Array;
        
        // Get time data from x_raw ArrayBuffer
        let time_array = Uint32Array::new(&data_series.x_raw);
        let total_ticks = time_array.length() as usize;
        
        // Get the first metric's y data (price)
        let metric = &data_series.metrics[0];
        let price_array = Float32Array::new(&metric.y_raw);
        
        // For each candle time period, aggregate the tick data
        for candle_idx in 0..num_candles {
            let candle_start = first_candle_start + (candle_idx as u32 * self.candle_timeframe);
            let candle_end = candle_start + self.candle_timeframe;
            
            // Find ticks within this candle's time range
            let mut open_price = None;
            let mut high_price = None;
            let mut low_price = None;
            let mut close_price = None;
            let mut _last_timestamp = 0u32;
            
            // Binary search to find the first tick in this candle's range
            let mut start_idx = 0;
            let mut end_idx = total_ticks;
            
            // Find first tick >= candle_start
            while start_idx < end_idx {
                let mid = start_idx + (end_idx - start_idx) / 2;
                let timestamp = time_array.get_index(mid as u32);
                if timestamp < candle_start {
                    start_idx = mid + 1;
                } else {
                    end_idx = mid;
                }
            }
            
            // Process ticks in this candle's time range
            for tick_idx in start_idx..total_ticks {
                let timestamp = time_array.get_index(tick_idx as u32);
                
                // Stop if we've passed this candle's end time
                if timestamp >= candle_end {
                    break;
                }
                
                // Get the actual price from the price array
                let price = price_array.get_index(tick_idx as u32);
                
                // Update OHLC values
                if open_price.is_none() {
                    open_price = Some(price);
                }
                high_price = Some(high_price.map_or(price, |h: f32| h.max(price)));
                low_price = Some(low_price.map_or(price, |l: f32| l.min(price)));
                close_price = Some(price);
                _last_timestamp = timestamp;
            }
            
            // If we found data for this candle, add it
            if let (Some(open), Some(high), Some(low), Some(close)) = 
                (open_price, high_price, low_price, close_price) {
                self.ohlc_data.push(OhlcData {
                    open,
                    high,
                    low,
                    close,
                });
            }
        }
        
        
        // Create vertex buffers for bodies and wicks
        self.create_vertex_buffers(device, ds);
    }
    
    /// Creates GPU vertex buffers for candle bodies and wicks.
    /// 
    /// Each candle body is rendered as two triangles (6 vertices).
    /// Each candle has two wicks rendered as lines (4 vertices total).
    fn create_vertex_buffers(&mut self, device: &wgpu::Device, ds: &DataStore) {
        // Body vertices: 6 vertices per candle (2 triangles)
        // Each vertex: [timestamp (u32), open, high, low, close (f32s)]
        let mut body_vertices: Vec<u8> = Vec::new();
        let mut wick_vertices: Vec<u8> = Vec::new();
        
        // Calculate first candle start for partial candles
        let first_candle_start = (ds.start_x / self.candle_timeframe) * self.candle_timeframe;
        
        for (i, ohlc) in self.ohlc_data.iter().enumerate() {
            // Calculate the correct candle position including partial candles
            let candle_start = first_candle_start + (i as u32 * self.candle_timeframe);
            let candle_mid_absolute = candle_start + (self.candle_timeframe / 2);
            
            
            // Create vertices for candle body (rectangle from open to close)
            let _body_top = ohlc.open.max(ohlc.close);
            let _body_bottom = ohlc.open.min(ohlc.close);
            
            // Create vertex data - need to store u32 timestamp followed by f32 values
            // For each of the 6 vertices of the two triangles
            for _ in 0..6 {
                // Push timestamp as bytes
                body_vertices.extend_from_slice(&candle_mid_absolute.to_ne_bytes());
                // Push OHLC as f32
                body_vertices.extend_from_slice(&ohlc.open.to_ne_bytes());
                body_vertices.extend_from_slice(&ohlc.high.to_ne_bytes());
                body_vertices.extend_from_slice(&ohlc.low.to_ne_bytes());
                body_vertices.extend_from_slice(&ohlc.close.to_ne_bytes());
            }
            
            // Create vertices for wicks (2 lines: high to body top, body bottom to low)
            for _ in 0..4 {
                // Push timestamp as bytes
                wick_vertices.extend_from_slice(&candle_mid_absolute.to_ne_bytes());
                // Push OHLC as f32
                wick_vertices.extend_from_slice(&ohlc.open.to_ne_bytes());
                wick_vertices.extend_from_slice(&ohlc.high.to_ne_bytes());
                wick_vertices.extend_from_slice(&ohlc.low.to_ne_bytes());
                wick_vertices.extend_from_slice(&ohlc.close.to_ne_bytes());
            }
        }
        
        // Create GPU buffers
        if !body_vertices.is_empty() {
            self.body_vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick Body Vertex Buffer"),
                contents: &body_vertices,
                usage: wgpu::BufferUsages::VERTEX,
            }));
            // Each vertex is u32 + 4*f32 = 20 bytes
            self.body_vertex_count = (body_vertices.len() / 20) as u32;
        }
        
        if !wick_vertices.is_empty() {
            self.wick_vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick Wick Vertex Buffer"),
                contents: &wick_vertices,
                usage: wgpu::BufferUsages::VERTEX,
            }));
            // Each vertex is u32 + 4*f32 = 20 bytes
            self.wick_vertex_count = (wick_vertices.len() / 20) as u32;
        }
    }
    
    fn create_bind_group(&self, device: &wgpu::Device, data_store: &DataStore) -> wgpu::BindGroup {
        use wgpu::util::DeviceExt;
        
        // Create uniform buffers
        let x_min_max = glm::vec2(data_store.start_x, data_store.end_x);
        let x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Candlestick X Range Buffer"),
            contents: bytemuck::cast_slice(&[x_min_max.x, x_min_max.y]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        let y_min_max = glm::vec2(
            data_store.min_y.unwrap_or(0.0),
            data_store.max_y.unwrap_or(1.0),
        );
        let y_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Candlestick Y Range Buffer"),
            contents: bytemuck::cast_slice(&[y_min_max.x, y_min_max.y]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        
        let timeframe_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Candlestick Timeframe Buffer"),
            contents: bytemuck::cast_slice(&[self.candle_timeframe as f32]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Candlestick Bind Group"),
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
                    resource: timeframe_buffer.as_entire_binding(),
                },
            ],
        })
    }
}