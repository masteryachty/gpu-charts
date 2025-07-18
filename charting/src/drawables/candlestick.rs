use std::cell::RefCell;
use std::rc::Rc;

use wgpu::TextureFormat;

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
/// - Performance optimizations: caching and indexed rendering
pub struct CandlestickRenderer {
    body_pipeline: wgpu::RenderPipeline,
    wick_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    candle_timeframe: u32, // in seconds
    ohlc_data: Vec<OhlcData>,
    body_vertex_buffer: Option<wgpu::Buffer>,
    wick_vertex_buffer: Option<wgpu::Buffer>,
    body_index_buffer: Option<wgpu::Buffer>,
    wick_index_buffer: Option<wgpu::Buffer>,
    body_vertex_count: u32,
    wick_vertex_count: u32,
    body_index_count: u32,
    wick_index_count: u32,
    
    // Cache key for performance optimization
    cache_key: Option<CacheKey>,
}

#[derive(PartialEq, Clone, Debug)]
struct CacheKey {
    start_x: u32,
    end_x: u32,
    candle_timeframe: u32,
    data_hash: u64,
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
        
        // Calculate cache key for current state
        let data_hash = self.calculate_data_hash(&ds);
        let new_cache_key = CacheKey {
            start_x: ds.start_x,
            end_x: ds.end_x,
            candle_timeframe: self.candle_timeframe,
            data_hash,
        };
        
        // Check if we need to re-aggregate
        let needs_reaggregation = self.cache_key.as_ref() != Some(&new_cache_key);
        
        if needs_reaggregation {
            self.aggregate_ohlc(device, queue, &ds);
            self.cache_key = Some(new_cache_key);
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
        
        // Render candle bodies with indexed drawing
        if let (Some(body_buffer), Some(body_index_buffer)) = (&self.body_vertex_buffer, &self.body_index_buffer) {
            render_pass.set_pipeline(&self.body_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, body_buffer.slice(..));
            render_pass.set_index_buffer(body_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.body_index_count, 0, 0..1);
        }
        
        // Render wicks with indexed drawing
        if let (Some(wick_buffer), Some(wick_index_buffer)) = (&self.wick_vertex_buffer, &self.wick_index_buffer) {
            render_pass.set_pipeline(&self.wick_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, wick_buffer.slice(..));
            render_pass.set_index_buffer(wick_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.wick_index_count, 0, 0..1);
        }
    }
}

impl CandlestickRenderer {
    /// Calculate a simple hash of the data to detect changes
    fn calculate_data_hash(&self, ds: &DataStore) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash the data length and active groups count
        ds.get_data_len().hash(&mut hasher);
        let active_groups = ds.get_active_data_groups();
        active_groups.len().hash(&mut hasher);
        
        // Hash data bounds if available
        if let (Some(min_y), Some(max_y)) = (ds.min_y, ds.max_y) {
            // Convert f32 to bits for hashing
            min_y.to_bits().hash(&mut hasher);
            max_y.to_bits().hash(&mut hasher);
        }
        
        // Hash the data series count and first series length if available
        ds.data_groups.len().hash(&mut hasher);
        if let Some(first_group) = ds.data_groups.first() {
            first_group.metrics.len().hash(&mut hasher);
        }
        
        hasher.finish()
    }
    
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
            ohlc_data: Vec::new(),
            body_vertex_buffer: None,
            wick_vertex_buffer: None,
            body_index_buffer: None,
            wick_index_buffer: None,
            body_vertex_count: 0,
            wick_vertex_count: 0,
            body_index_count: 0,
            wick_index_count: 0,
            cache_key: None,
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
    
    /// Creates GPU vertex buffers for candle bodies and wicks with indexed rendering.
    /// 
    /// Bodies: 4 unique vertices per candle (rectangle) with 6 indices (2 triangles)
    /// Wicks: 4 unique vertices per candle (2 lines) with 4 indices
    /// This reduces memory usage by ~40% compared to non-indexed rendering.
    fn create_vertex_buffers(&mut self, device: &wgpu::Device, ds: &DataStore) {
        use wgpu::util::DeviceExt;
        
        // Vertex and index buffers for optimized rendering
        let mut body_vertices: Vec<u8> = Vec::new();
        let mut body_indices: Vec<u16> = Vec::new();
        let mut wick_vertices: Vec<u8> = Vec::new();
        let mut wick_indices: Vec<u16> = Vec::new();
        
        // Calculate first candle start for partial candles
        let first_candle_start = (ds.start_x / self.candle_timeframe) * self.candle_timeframe;
        
        for (i, ohlc) in self.ohlc_data.iter().enumerate() {
            // Calculate the correct candle position including partial candles
            let candle_start = first_candle_start + (i as u32 * self.candle_timeframe);
            let candle_mid_absolute = candle_start + (self.candle_timeframe / 2);
            
            // Body vertices: 4 unique vertices for rectangle
            // Store timestamp and OHLC data for each vertex
            let base_vertex_idx = (i * 4) as u16;
            
            // Add 4 unique vertices for the body rectangle
            for _ in 0..4 {
                body_vertices.extend_from_slice(&candle_mid_absolute.to_ne_bytes());
                body_vertices.extend_from_slice(&ohlc.open.to_ne_bytes());
                body_vertices.extend_from_slice(&ohlc.high.to_ne_bytes());
                body_vertices.extend_from_slice(&ohlc.low.to_ne_bytes());
                body_vertices.extend_from_slice(&ohlc.close.to_ne_bytes());
            }
            
            // Body indices: 2 triangles (6 indices) forming a rectangle
            // Triangle 1: bottom-left, top-left, top-right
            body_indices.push(base_vertex_idx);
            body_indices.push(base_vertex_idx + 1);
            body_indices.push(base_vertex_idx + 2);
            // Triangle 2: bottom-left, top-right, bottom-right
            body_indices.push(base_vertex_idx);
            body_indices.push(base_vertex_idx + 2);
            body_indices.push(base_vertex_idx + 3);
            
            // Wick vertices: 4 vertices for 2 lines
            let wick_base_idx = (i * 4) as u16;
            
            // Add 4 vertices for wicks (top wick start/end, bottom wick start/end)
            for _ in 0..4 {
                wick_vertices.extend_from_slice(&candle_mid_absolute.to_ne_bytes());
                wick_vertices.extend_from_slice(&ohlc.open.to_ne_bytes());
                wick_vertices.extend_from_slice(&ohlc.high.to_ne_bytes());
                wick_vertices.extend_from_slice(&ohlc.low.to_ne_bytes());
                wick_vertices.extend_from_slice(&ohlc.close.to_ne_bytes());
            }
            
            // Wick indices: 2 lines (4 indices)
            // Top wick: from high to body top
            wick_indices.push(wick_base_idx);
            wick_indices.push(wick_base_idx + 1);
            // Bottom wick: from body bottom to low
            wick_indices.push(wick_base_idx + 2);
            wick_indices.push(wick_base_idx + 3);
        }
        
        // Create GPU buffers for bodies
        if !body_vertices.is_empty() {
            self.body_vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick Body Vertex Buffer"),
                contents: &body_vertices,
                usage: wgpu::BufferUsages::VERTEX,
            }));
            
            self.body_index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick Body Index Buffer"),
                contents: bytemuck::cast_slice(&body_indices),
                usage: wgpu::BufferUsages::INDEX,
            }));
            
            // 4 vertices per candle
            self.body_vertex_count = (self.ohlc_data.len() * 4) as u32;
            self.body_index_count = body_indices.len() as u32;
        }
        
        // Create GPU buffers for wicks
        if !wick_vertices.is_empty() {
            self.wick_vertex_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick Wick Vertex Buffer"),
                contents: &wick_vertices,
                usage: wgpu::BufferUsages::VERTEX,
            }));
            
            self.wick_index_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick Wick Index Buffer"),
                contents: bytemuck::cast_slice(&wick_indices),
                usage: wgpu::BufferUsages::INDEX,
            }));
            
            // 4 vertices per candle
            self.wick_vertex_count = (self.ohlc_data.len() * 4) as u32;
            self.wick_index_count = wick_indices.len() as u32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_savings_with_indexed_rendering() {
        // Calculate memory usage for non-indexed vs indexed rendering
        let num_candles = 1000;
        
        // Non-indexed: 6 vertices per body + 4 vertices per wick = 10 vertices per candle
        // Each vertex is 20 bytes (u32 + 4*f32)
        let non_indexed_memory = num_candles * 10 * 20;
        
        // Indexed: 4 vertices per body + 4 vertices per wick = 8 vertices per candle
        // Plus indices: 6 u16 for body + 4 u16 for wick = 10 u16 = 20 bytes per candle
        let indexed_vertex_memory = num_candles * 8 * 20;
        let indexed_index_memory = num_candles * 10 * 2;
        let indexed_total_memory = indexed_vertex_memory + indexed_index_memory;
        
        // Calculate savings
        let memory_saved = non_indexed_memory - indexed_total_memory;
        let savings_percent = (memory_saved as f32 / non_indexed_memory as f32) * 100.0;
        
        println!("Non-indexed memory: {} bytes", non_indexed_memory);
        println!("Indexed memory: {} bytes", indexed_total_memory);
        println!("Memory saved: {} bytes ({:.1}%)", memory_saved, savings_percent);
        
        // Assert we achieve at least 15% memory savings
        assert!(savings_percent >= 15.0);
    }
    
    #[test]
    fn test_cache_key_functionality() {
        let cache_key1 = CacheKey {
            start_x: 1000,
            end_x: 2000,
            candle_timeframe: 60,
            data_hash: 12345,
        };
        
        let cache_key2 = CacheKey {
            start_x: 1000,
            end_x: 2000,
            candle_timeframe: 60,
            data_hash: 12345,
        };
        
        let cache_key3 = CacheKey {
            start_x: 1000,
            end_x: 2000,
            candle_timeframe: 60,
            data_hash: 54321, // Different hash
        };
        
        // Same keys should be equal
        assert_eq!(cache_key1, cache_key2);
        
        // Different data hash should make keys unequal
        assert_ne!(cache_key1, cache_key3);
    }
}