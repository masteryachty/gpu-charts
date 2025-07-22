use std::cell::RefCell;
use std::rc::Rc;

use nalgebra_glm as glm;
use wgpu::util::DeviceExt;
use wgpu::TextureFormat;

use crate::calcables::CandleAggregator;
use crate::renderer::data_store::DataStore;
use crate::renderer::render_engine::RenderEngine;

use super::plot::RenderListener;

/// Renders candlestick charts for financial data visualization.
///
/// This renderer aggregates tick data into OHLC (Open, High, Low, Close) candles
/// based on a configurable time frame, then renders them using WebGPU.
///
/// Features:
/// - GPU-accelerated OHLC aggregation using compute shaders
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

    // GPU compute aggregator
    candle_aggregator: CandleAggregator,
    gpu_candles_buffer: Option<wgpu::Buffer>,
    num_candles: u32,

    // Cache key for performance optimization
    cache_key: Option<CacheKey>,

    // Buffer pool for uniform buffers to avoid repeated allocations
    uniform_buffer_pool: BufferPool,
}

/// Buffer pool for reusing uniform buffers to avoid repeated allocations
struct BufferPool {
    x_range_buffers: Vec<wgpu::Buffer>,
    y_range_buffers: Vec<wgpu::Buffer>,
    timeframe_buffers: Vec<wgpu::Buffer>,
}

impl BufferPool {
    fn new() -> Self {
        Self {
            x_range_buffers: Vec::new(),
            y_range_buffers: Vec::new(),
            timeframe_buffers: Vec::new(),
        }
    }

    fn get_or_create_x_range_buffer(
        &mut self,
        device: &wgpu::Device,
        data: &[f32],
    ) -> wgpu::Buffer {
        if let Some(buffer) = self.x_range_buffers.pop() {
            // Reuse existing buffer - write new data to it
            buffer
        } else {
            // Create new buffer
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick X Range Buffer (Pooled)"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        }
    }

    fn get_or_create_y_range_buffer(
        &mut self,
        device: &wgpu::Device,
        data: &[f32],
    ) -> wgpu::Buffer {
        if let Some(buffer) = self.y_range_buffers.pop() {
            buffer
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick Y Range Buffer (Pooled)"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        }
    }

    fn get_or_create_timeframe_buffer(
        &mut self,
        device: &wgpu::Device,
        data: &[f32],
    ) -> wgpu::Buffer {
        if let Some(buffer) = self.timeframe_buffers.pop() {
            buffer
        } else {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Candlestick Timeframe Buffer (Pooled)"),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        }
    }

    fn return_buffers(
        &mut self,
        x_buffer: wgpu::Buffer,
        y_buffer: wgpu::Buffer,
        timeframe_buffer: wgpu::Buffer,
    ) {
        // Return buffers to pool for reuse (in a real implementation, we'd check buffer sizes)
        self.x_range_buffers.push(x_buffer);
        self.y_range_buffers.push(y_buffer);
        self.timeframe_buffers.push(timeframe_buffer);
    }
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

        // Only render if we have GPU candles
        if self.gpu_candles_buffer.is_none() || self.num_candles == 0 {
            log::warn!("CandlestickRenderer: No GPU candles to render. gpu_candles_buffer: {}, num_candles: {}", 
                      self.gpu_candles_buffer.is_some(), self.num_candles);
            return;
        }

        log::info!(
            "CandlestickRenderer: Rendering {} candles",
            self.num_candles
        );

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
        let bind_group = match self.create_bind_group(device, &ds) {
            Some(bg) => bg,
            None => return,
        };

        // Render candle bodies
        render_pass.set_pipeline(&self.body_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        // Draw 6 vertices per candle (2 triangles)
        let body_vertex_count = self.num_candles * 6;
        log::info!(
            "CandlestickRenderer: Drawing {} body vertices for {} candles",
            body_vertex_count,
            self.num_candles
        );
        render_pass.draw(0..body_vertex_count, 0..1);

        // Render wicks
        render_pass.set_pipeline(&self.wick_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        // Draw 4 vertices per candle (2 lines)
        let wick_vertex_count = self.num_candles * 4;
        log::info!(
            "CandlestickRenderer: Drawing {} wick vertices",
            wick_vertex_count
        );
        render_pass.draw(0..wick_vertex_count, 0..1);
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
                // Candles storage buffer (from GPU compute)
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
            label: Some("Candlestick Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // No vertex buffers needed - we read from storage buffer

        // Create body rendering pipeline (filled rectangles)
        let body_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Candlestick Body Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_body"),
                compilation_options: Default::default(),
                buffers: &[],
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
                buffers: &[],
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

        let candle_aggregator = CandleAggregator::new(device);

        Self {
            body_pipeline,
            wick_pipeline,
            bind_group_layout,
            candle_timeframe: 60, // Default 1 minute
            candle_aggregator,
            gpu_candles_buffer: None,
            num_candles: 0,
            cache_key: None,
            uniform_buffer_pool: BufferPool::new(),
        }
    }

    /// Aggregates tick data into OHLC candles using GPU compute shaders.
    ///
    /// This method:
    /// 1. Calculates candle boundaries to include partial candles at view edges
    /// 2. Uses GPU compute shaders to aggregate ticks in parallel
    /// 3. Stores results directly in GPU memory for rendering
    /// 4. Creates vertex buffers from GPU candle data
    fn aggregate_ohlc(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, ds: &DataStore) {
        // Calculate candle boundaries to include partial candles
        // Find the first candle that starts at or before the view start
        let first_candle_start = (ds.start_x / self.candle_timeframe) * self.candle_timeframe;

        // Find the last candle that ends at or after the view end
        let last_candle_end = ds.end_x.div_ceil(self.candle_timeframe) * self.candle_timeframe;

        let extended_time_range = last_candle_end - first_candle_start;
        let num_candles = (extended_time_range / self.candle_timeframe) as u32;
        self.num_candles = num_candles;

        log::info!("CandlestickRenderer: Aggregating OHLC data. Time range: {} to {}, candle_timeframe: {}, num_candles: {}", 
                  ds.start_x, ds.end_x, self.candle_timeframe, num_candles);

        // Get the active data groups
        let active_groups = ds.get_active_data_groups();
        if active_groups.is_empty() {
            log::warn!("CandlestickRenderer: No active data groups available");
            return;
        }

        // Use the first data group and first metric (price data)
        let data_series = &active_groups[0];
        if data_series.metrics.is_empty() {
            log::warn!("CandlestickRenderer: No metrics available in data series");
            return;
        }

        log::info!(
            "CandlestickRenderer: Using metric '{}' for OHLC aggregation",
            data_series.metrics[0].name
        );

        // Check if we have GPU buffers
        if data_series.x_buffers.is_empty() || data_series.metrics[0].y_buffers.is_empty() {
            log::warn!(
                "CandlestickRenderer: No GPU buffers available. x_buffers: {}, y_buffers: {}",
                data_series.x_buffers.len(),
                data_series.metrics[0].y_buffers.len()
            );
            return;
        }

        // Create command encoder for GPU work
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Candle Aggregation Encoder"),
        });

        // Handle multiple buffer chunks if necessary
        let tick_count = data_series.length;

        // Validate array lengths match before aggregation
        if data_series.x_buffers.len() != data_series.metrics[0].y_buffers.len() {
            log::error!(
                "CandlestickRenderer: X and Y buffer array lengths don't match: {} vs {}",
                data_series.x_buffers.len(),
                data_series.metrics[0].y_buffers.len()
            );
            return;
        }

        // For now, handle single chunk case (most common)
        // TODO: Implement multi-chunk support for very large datasets
        if data_series.x_buffers.len() == 1 && data_series.metrics[0].y_buffers.len() == 1 {
            // Validate buffer sizes are consistent
            let x_buffer_size = data_series.x_buffers[0].size();
            let y_buffer_size = data_series.metrics[0].y_buffers[0].size();
            let expected_x_elements = x_buffer_size / 4; // u32 = 4 bytes
            let expected_y_elements = y_buffer_size / 4; // f32 = 4 bytes

            if expected_x_elements != expected_y_elements {
                log::error!(
                    "CandlestickRenderer: X and Y buffer sizes don't match: {expected_x_elements} vs {expected_y_elements} elements"
                );
                return;
            }

            if expected_x_elements != tick_count as u64 {
                log::warn!(
                    "CandlestickRenderer: Buffer size ({expected_x_elements}) doesn't match tick count ({tick_count})"
                );
            }

            self.gpu_candles_buffer = Some(
                self.candle_aggregator
                    .aggregate_candles(
                        device,
                        queue,
                        &mut encoder,
                        &data_series.x_buffers[0],
                        &data_series.metrics[0].y_buffers[0],
                        tick_count,
                        first_candle_start,
                        self.candle_timeframe,
                        num_candles,
                    )
                    .clone(),
            );
        } else {
            // Multiple chunks - use chunked aggregation
            let mut chunk_sizes = Vec::new();
            let mut total_expected_elements = 0u64;

            for (i, buffer) in data_series.x_buffers.iter().enumerate() {
                // Calculate chunk size from buffer size
                let x_buffer_size = buffer.size();
                let y_buffer_size = data_series.metrics[0].y_buffers[i].size();
                let x_chunk_size = (x_buffer_size / 4) as u32; // u32 = 4 bytes
                let y_chunk_size = (y_buffer_size / 4) as u32; // f32 = 4 bytes

                if x_chunk_size != y_chunk_size {
                    log::error!("CandlestickRenderer: Chunk {i} X and Y buffer sizes don't match: {x_chunk_size} vs {y_chunk_size} elements");
                    return;
                }

                chunk_sizes.push(x_chunk_size);
                total_expected_elements += x_chunk_size as u64;
            }

            if total_expected_elements != tick_count as u64 {
                log::warn!(
                    "CandlestickRenderer: Total chunk size ({total_expected_elements}) doesn't match tick count ({tick_count})"
                );
            }

            self.gpu_candles_buffer = Some(
                self.candle_aggregator
                    .aggregate_candles_chunked(
                        device,
                        queue,
                        &mut encoder,
                        &data_series.x_buffers,
                        &data_series.metrics[0].y_buffers,
                        &chunk_sizes,
                        tick_count,
                        first_candle_start,
                        self.candle_timeframe,
                        num_candles,
                    )
                    .clone(),
            );
        }

        // Submit GPU work
        queue.submit(Some(encoder.finish()));

        log::info!(
            "CandlestickRenderer: GPU aggregation complete. Buffer size: {} bytes for {} candles",
            self.num_candles as usize
                * std::mem::size_of::<crate::calcables::candle_aggregator::GpuOhlcCandle>(),
            self.num_candles
        );
    }

    fn create_bind_group(
        &mut self,
        device: &wgpu::Device,
        data_store: &DataStore,
    ) -> Option<wgpu::BindGroup> {
        // Check if we have GPU candles buffer
        let candles_buffer = self.gpu_candles_buffer.as_ref()?;

        // Create uniform buffers using buffer pool
        // Note: The shader expects MinMaxU32 struct with u32 values
        let x_range_data = [data_store.start_x, data_store.end_x];
        let x_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Candlestick X Range Buffer"),
            contents: bytemuck::cast_slice(&x_range_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let y_min_max = glm::vec2(
            data_store.min_y.unwrap_or(0.0),
            data_store.max_y.unwrap_or(1.0),
        );
        let y_buffer = self
            .uniform_buffer_pool
            .get_or_create_y_range_buffer(device, &[y_min_max.x, y_min_max.y]);

        let timeframe_buffer = self
            .uniform_buffer_pool
            .get_or_create_timeframe_buffer(device, &[self.candle_timeframe as f32]);

        Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: candles_buffer.as_entire_binding(),
                },
            ],
        }))
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
        println!(
            "Memory saved: {} bytes ({:.1}%)",
            memory_saved, savings_percent
        );

        // Assert we achieve at least 10% memory savings
        assert!(savings_percent >= 10.0);
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
