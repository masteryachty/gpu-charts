use wgpu::util::DeviceExt;

/// GPU-accelerated OHLC candle aggregator using compute shaders.
///
/// This module processes tick data in parallel on the GPU to generate
/// OHLC (Open, High, Low, Close) candles for financial charting.
pub struct CandleAggregator {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    // Cached buffers for reuse
    params_buffer: Option<wgpu::Buffer>,
    output_buffer: Option<wgpu::Buffer>,
    last_num_candles: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CandleParams {
    start_timestamp: u32,
    candle_timeframe: u32,
    num_candles: u32,
    tick_count: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuOhlcCandle {
    pub timestamp: u32,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
}

impl CandleAggregator {
    /// Creates a new GPU candle aggregator with compiled compute pipeline.
    #[allow(dead_code)]
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Candle Aggregation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("candle_aggregation.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Candle Aggregation Bind Group Layout"),
            entries: &[
                // Timestamps buffer (storage, read)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Prices buffer (storage, read)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output candles buffer (storage, read_write)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Parameters uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
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
            label: Some("Candle Aggregation Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Candle Aggregation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            cache: None,
            compilation_options: Default::default(),
        });

        Self {
            pipeline,
            bind_group_layout,
            params_buffer: None,
            output_buffer: None,
            last_num_candles: 0,
        }
    }

    /// Aggregates tick data into OHLC candles using GPU compute.
    ///
    /// Returns a buffer containing the computed candles in GPU memory.
    /// The buffer can be used directly for rendering without CPU readback.
    pub fn aggregate_candles(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        timestamps_buffer: &wgpu::Buffer,
        prices_buffer: &wgpu::Buffer,
        tick_count: u32,
        start_timestamp: u32,
        candle_timeframe: u32,
        num_candles: u32,
    ) -> &wgpu::Buffer {
        // Create or reuse output buffer
        let candle_size = std::mem::size_of::<GpuOhlcCandle>();
        let output_size = (num_candles as usize * candle_size) as u64;

        if self.last_num_candles != num_candles || self.output_buffer.is_none() {
            self.output_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Candle Output Buffer"),
                size: output_size,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::VERTEX,
                mapped_at_creation: false,
            }));
            self.last_num_candles = num_candles;
        }

        // Create or update params buffer
        let params = CandleParams {
            start_timestamp,
            candle_timeframe,
            num_candles,
            tick_count,
        };

        if self.params_buffer.is_none() {
            self.params_buffer = Some(device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Candle Params"),
                    contents: bytemuck::cast_slice(&[params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                },
            ));
        } else {
            // Update existing buffer
            queue.write_buffer(
                self.params_buffer.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&[params]),
            );
        }

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Candle Aggregation Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: timestamps_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: prices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.output_buffer.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params_buffer.as_ref().unwrap().as_entire_binding(),
                },
            ],
        });

        // Dispatch compute work
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Candle Aggregation Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Each workgroup processes one candle
            compute_pass.dispatch_workgroups(num_candles, 1, 1);
        }

        self.output_buffer.as_ref().unwrap()
    }

    /// Aggregates tick data from multiple buffer chunks.
    ///
    /// This handles the case where data is split across multiple GPU buffers.
    pub fn aggregate_candles_chunked(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        timestamps_chunks: &[wgpu::Buffer],
        prices_chunks: &[wgpu::Buffer],
        chunk_sizes: &[u32],
        total_tick_count: u32,
        start_timestamp: u32,
        candle_timeframe: u32,
        num_candles: u32,
    ) -> &wgpu::Buffer {
        // For multiple chunks, we need to either:
        // 1. Concatenate chunks into a single buffer (simple but uses more memory)
        // 2. Process each chunk separately and merge results (complex but memory efficient)

        // For now, implement option 1 for simplicity
        // TODO: Implement option 2 for better memory efficiency with very large datasets

        if timestamps_chunks.len() == 1 && prices_chunks.len() == 1 {
            // Single chunk - use direct method
            return self.aggregate_candles(
                device,
                queue,
                encoder,
                &timestamps_chunks[0],
                &prices_chunks[0],
                total_tick_count,
                start_timestamp,
                candle_timeframe,
                num_candles,
            );
        }

        // Multiple chunks - concatenate into single buffer
        let total_time_size = (total_tick_count * 4) as u64; // u32 = 4 bytes
        let total_price_size = (total_tick_count * 4) as u64; // f32 = 4 bytes

        let concat_time_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Concatenated Time Buffer"),
            size: total_time_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let concat_price_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Concatenated Price Buffer"),
            size: total_price_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy chunks to concatenated buffers
        let mut offset = 0u64;
        for (i, chunk_size) in chunk_sizes.iter().enumerate() {
            let size = (*chunk_size * 4) as u64;

            encoder.copy_buffer_to_buffer(
                &timestamps_chunks[i],
                0,
                &concat_time_buffer,
                offset,
                size,
            );

            encoder.copy_buffer_to_buffer(&prices_chunks[i], 0, &concat_price_buffer, offset, size);

            offset += size;
        }

        // Process concatenated buffers
        self.aggregate_candles(
            device,
            queue,
            encoder,
            &concat_time_buffer,
            &concat_price_buffer,
            total_tick_count,
            start_timestamp,
            candle_timeframe,
            num_candles,
        )
    }

    /// Returns the size of the output buffer in bytes.
    #[allow(dead_code)]
    pub fn get_output_buffer_size(&self) -> u64 {
        (self.last_num_candles as usize * std::mem::size_of::<GpuOhlcCandle>()) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_params_layout() {
        // Verify struct layout matches WGSL expectations
        assert_eq!(std::mem::size_of::<CandleParams>(), 16);
        assert_eq!(std::mem::size_of::<GpuOhlcCandle>(), 20);
    }
}
