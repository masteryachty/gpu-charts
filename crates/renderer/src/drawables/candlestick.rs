use std::rc::Rc;

use nalgebra_glm as glm;
use wgpu::util::DeviceExt;
use wgpu::TextureFormat;

use crate::calcables::CandleAggregator;
use data_manager::DataStore;

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
}

#[derive(PartialEq, Clone, Debug)]
struct CacheKey {
    start_x: u32,
    end_x: u32,
    candle_timeframe: u32,
    data_hash: u64,
}

impl CandlestickRenderer {
    /// Get the GPU candles buffer if available
    pub fn get_candles_buffer(&self) -> Option<&wgpu::Buffer> {
        self.gpu_candles_buffer.as_ref()
    }
    
    /// Get the number of candles
    pub fn get_num_candles(&self) -> u32 {
        self.num_candles
    }
    
    /// Calculate the optimal candle timeframe based on the visible time range
    /// Aims to show the most candles while keeping the count under 100
    fn calculate_optimal_timeframe(&self, data_store: &DataStore) -> u32 {
        // Define available timeframes (in seconds)
        const TIMEFRAMES: &[u32] = &[
            1,       // 1 second
            5,       // 5 seconds
            10,      // 10 seconds
            30,      // 30 seconds
            60,      // 1 minute
            300,     // 5 minutes
            900,     // 15 minutes
            1800,    // 30 minutes
            3600,    // 1 hour
            14400,   // 4 hours
            86400,   // 1 day
            604800,  // 1 week
            2592000, // 30 days (1 month)
        ];

        let time_range = data_store.end_x - data_store.start_x;
        let target_candles = 300;

        // Find the smallest timeframe that keeps candle count under target
        let mut selected_timeframe = TIMEFRAMES[0];

        for &timeframe in TIMEFRAMES.iter() {
            let candle_count = time_range / timeframe;

            if candle_count <= target_candles {
                selected_timeframe = timeframe;
                break;
            }
        }

        // If even the largest timeframe produces too many candles,
        // use a custom timeframe that produces exactly the target count
        if time_range / TIMEFRAMES.last().unwrap() > target_candles {
            selected_timeframe = time_range / target_candles;
        }

        selected_timeframe
    }

    /// Get a human-readable description of the timeframe
    pub fn get_timeframe_description(&self) -> String {
        match self.candle_timeframe {
            1 => "1s".to_string(),
            5 => "5s".to_string(),
            10 => "10s".to_string(),
            30 => "30s".to_string(),
            60 => "1m".to_string(),
            300 => "5m".to_string(),
            900 => "15m".to_string(),
            1800 => "30m".to_string(),
            3600 => "1h".to_string(),
            14400 => "4h".to_string(),
            86400 => "1d".to_string(),
            604800 => "1w".to_string(),
            2592000 => "1M".to_string(),
            n if n < 60 => format!("{n}s"),
            n if n < 3600 => format!("{}m", n / 60),
            n if n < 86400 => format!("{}h", n / 3600),
            n => format!("{}d", n / 86400),
        }
    }
    
    /// Prepare candles without rendering - used by compute engine
    pub fn prepare_candles(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        data_store: &DataStore,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        log::info!("[CandlestickRenderer] prepare_candles called");
        
        // Check if we have data
        let data_len = data_store.get_data_len();
        log::info!("[CandlestickRenderer] Data length: {}", data_len);
        if data_len == 0 {
            log::warn!("[CandlestickRenderer] No data available, skipping candle preparation");
            return;
        }

        // Automatically select optimal candle timeframe based on time range
        self.candle_timeframe = self.calculate_optimal_timeframe(data_store);
        log::info!("[CandlestickRenderer] Selected candle timeframe: {} seconds", self.candle_timeframe);

        // Calculate cache key for current state
        let data_hash = self.calculate_data_hash(data_store);
        let new_cache_key = CacheKey {
            start_x: data_store.start_x,
            end_x: data_store.end_x,
            candle_timeframe: self.candle_timeframe,
            data_hash,
        };

        // Check if we need to re-aggregate
        let needs_reaggregation = self.cache_key.as_ref() != Some(&new_cache_key);

        if needs_reaggregation {
            // Run aggregation
            self.aggregate_ohlc(encoder, device, queue, data_store);
            self.cache_key = Some(new_cache_key);
        }
    }
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        data_store: &DataStore,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        // Prepare candles if not already done (this is idempotent due to caching)
        self.prepare_candles(encoder, data_store, device, queue);

        // Only render if we have GPU candles
        if self.gpu_candles_buffer.is_none() || self.num_candles == 0 {
            return;
        }

        // Begin render pass in a block to ensure it's dropped before function returns
        {
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
            let bind_group = match self.create_bind_group(device, data_store) {
                Some(bg) => bg,
                None => return,
            };

            // Render candle bodies
            render_pass.set_pipeline(&self.body_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            // Draw 6 vertices per candle (2 triangles)
            let body_vertex_count = self.num_candles * 6;
            render_pass.draw(0..body_vertex_count, 0..1);

            // Render wicks
            render_pass.set_pipeline(&self.wick_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            // Draw 4 vertices per candle (2 lines)
            let wick_vertex_count = self.num_candles * 4;
            render_pass.draw(0..wick_vertex_count, 0..1);
        } // render_pass drops here
    }
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
        if let (Some(min_y), Some(max_y)) = (ds.gpu_min_y, ds.gpu_max_y) {
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
        device: Rc<wgpu::Device>,
        _queue: Rc<wgpu::Queue>,
        color_format: TextureFormat,
    ) -> Self {
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

        let candle_aggregator = CandleAggregator::new(&device);

        Self {
            body_pipeline,
            wick_pipeline,
            bind_group_layout,
            candle_timeframe: 60, // Default 1 minute
            candle_aggregator,
            gpu_candles_buffer: None,
            num_candles: 0,
            cache_key: None,
        }
    }

    /// Aggregates tick data into OHLC candles using GPU compute shaders.
    ///
    /// This method:
    /// 1. Calculates candle boundaries to include partial candles at view edges
    /// 2. Uses GPU compute shaders to aggregate ticks in parallel
    /// 3. Stores results directly in GPU memory for rendering
    /// 4. Creates vertex buffers from GPU candle data
    fn aggregate_ohlc(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ds: &DataStore,
    ) {
        // Calculate candle boundaries to include partial candles
        // Find the first candle that starts at or before the view start
        let first_candle_start = (ds.start_x / self.candle_timeframe) * self.candle_timeframe;

        // Find the last candle that ends at or after the view end
        let last_candle_end = ds.end_x.div_ceil(self.candle_timeframe) * self.candle_timeframe;

        let extended_time_range = last_candle_end - first_candle_start;
        let num_candles = extended_time_range / self.candle_timeframe;
        self.num_candles = num_candles;

        // Get the active data groups
        let active_groups = ds.get_active_data_groups();
        if active_groups.is_empty() {
            log::warn!("CandlestickRenderer: No active data groups available");
            return;
        }

        // For candlestick, we look for price data to aggregate into OHLC
        let mut price_data = None;

        for (group_idx, group) in active_groups.iter().enumerate() {
            for (metric_idx, metric) in group.metrics.iter().enumerate() {
                if metric.name == "price" {
                    price_data = Some((group_idx, metric_idx));
                    break;
                }
            }
            if price_data.is_some() {
                break;
            }
        }

        let (group_idx, price_idx) = match price_data {
            Some(data) => data,
            None => {
                log::warn!("CandlestickRenderer: Could not find price data for OHLC aggregation");
                return;
            }
        };

        let data_series = &active_groups[group_idx];
        let price_metric = &data_series.metrics[price_idx];

        // Check if we have GPU buffers
        if data_series.x_buffers.is_empty() || price_metric.y_buffers.is_empty() {
            log::warn!("CandlestickRenderer: No GPU buffers available");
            return;
        }

        // Use the passed encoder for GPU work to batch operations

        // Handle multiple buffer chunks if necessary
        let tick_count = data_series.length;

        // Validate array lengths match before aggregation
        if data_series.x_buffers.len() != price_metric.y_buffers.len() {
            log::error!(
                "CandlestickRenderer: X and Y buffer array lengths don't match: {} vs {}",
                data_series.x_buffers.len(),
                price_metric.y_buffers.len()
            );
            return;
        }

        // For now, handle single chunk case (most common)
        // TODO: Implement multi-chunk support for very large datasets
        if data_series.x_buffers.len() == 1 && price_metric.y_buffers.len() == 1 {
            // Validate buffer sizes are consistent
            let x_buffer_size = data_series.x_buffers[0].size();
            let y_buffer_size = price_metric.y_buffers[0].size();
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
                        encoder,
                        &data_series.x_buffers[0],
                        &price_metric.y_buffers[0],
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
                let y_buffer_size = price_metric.y_buffers[i].size();
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
                        encoder,
                        &data_series.x_buffers,
                        &price_metric.y_buffers,
                        &chunk_sizes,
                        tick_count,
                        first_candle_start,
                        self.candle_timeframe,
                        num_candles,
                    )
                    .clone(),
            );
        }
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
            data_store.gpu_min_y.unwrap_or(0.0),
            data_store.gpu_max_y.unwrap_or(1.0),
        );
        let y_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Candlestick Y Range Buffer"),
            contents: bytemuck::cast_slice(&[y_min_max.x, y_min_max.y]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let timeframe_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Candlestick Timeframe Buffer"),
            contents: bytemuck::cast_slice(&[self.candle_timeframe as f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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
