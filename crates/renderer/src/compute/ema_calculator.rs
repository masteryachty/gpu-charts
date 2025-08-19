//! EMA (Exponential Moving Average) calculator using GPU compute shaders

use super::{ComputeInfrastructure, ComputeProcessor, ComputeResult};
use std::collections::HashMap;
use std::rc::Rc;
use wgpu::util::DeviceExt;

/// Available EMA periods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmaPeriod {
    Ema9 = 9,
    Ema20 = 20,
    Ema50 = 50,
    Ema100 = 100,
    Ema200 = 200,
}

impl EmaPeriod {
    /// Get all available periods
    pub fn all() -> Vec<EmaPeriod> {
        vec![
            EmaPeriod::Ema9,
            EmaPeriod::Ema20,
            EmaPeriod::Ema50,
            EmaPeriod::Ema100,
            EmaPeriod::Ema200,
        ]
    }
    
    /// Get the period value as u32
    pub fn value(&self) -> u32 {
        *self as u32
    }
    
    /// Get the column name for this EMA
    pub fn column_name(&self) -> String {
        format!("ema_{}", self.value())
    }
}

/// Parameters for the EMA compute shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct EmaParams {
    element_count: u32,
    period: u32,
    alpha_numerator: u32,
    alpha_denominator: u32,
}

/// Calculates EMAs from price data using GPU compute
pub struct EmaCalculator {
    infrastructure: ComputeInfrastructure,
    pipeline: wgpu::ComputePipeline,
    multi_pipeline: wgpu::ComputePipeline, // For calculating multiple EMAs at once
    bind_group_layout: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,
    // Cache for computed EMA buffers
    cache: HashMap<(EmaPeriod, u32), wgpu::Buffer>, // (period, element_count) -> buffer
}

impl EmaCalculator {
    /// Create a new EMA calculator
    pub fn new(device: Rc<wgpu::Device>, queue: Rc<wgpu::Queue>) -> Result<Self, String> {
        let infrastructure = ComputeInfrastructure::new(device.clone(), queue);

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("EMA Compute Bind Group Layout"),
            entries: &[
                // Price data buffer (input)
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
                // EMA output buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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
                    binding: 2,
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

        // Load shader
        let shader_source = include_str!("ema_compute.wgsl");
        
        // Create single EMA pipeline
        let pipeline = infrastructure.create_compute_pipeline(
            shader_source,
            "compute_ema",
            &bind_group_layout,
        )?;
        
        // Create multi-EMA pipeline
        let multi_pipeline = infrastructure.create_compute_pipeline(
            shader_source,
            "compute_ema_multi",
            &bind_group_layout,
        )?;

        // Create params buffer
        let params = EmaParams {
            element_count: 0,
            period: 0,
            alpha_numerator: 2,
            alpha_denominator: 1,
        };

        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("EMA Params Buffer"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            infrastructure,
            pipeline,
            multi_pipeline,
            bind_group_layout,
            params_buffer,
            cache: HashMap::new(),
        })
    }

    /// Calculate a single EMA from price buffer
    pub fn calculate_single(
        &mut self,
        price_buffer: &wgpu::Buffer,
        element_count: u32,
        period: EmaPeriod,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeResult, String> {
        log::info!("[EmaCalculator] Calculating EMA {} for {} elements", period.value(), element_count);
        
        // Calculate and log alpha value for this period
        let alpha = 2.0 / (period.value() as f32 + 1.0);
        log::info!("[EmaCalculator] EMA {} alpha value: {:.6}, weight of new data: {:.2}%", 
            period.value(), alpha, alpha * 100.0);
        
        // Log expected behavior based on data count
        if element_count < period.value() {
            log::warn!("[EmaCalculator] Only {} data points for EMA {} - will use all available data for initial average", 
                element_count, period.value());
        } else if element_count < period.value() * 3 {
            log::info!("[EmaCalculator] {} data points for EMA {} - limited divergence expected (need {}+ for full divergence)", 
                element_count, period.value(), period.value() * 3);
        } else {
            log::info!("[EmaCalculator] {} data points for EMA {} - sufficient data for proper EMA behavior", 
                element_count, period.value());
        }
        
        // Check cache - but for now, let's disable caching to debug the issue
        // if let Some(cached_buffer) = self.cache.get(&(period, element_count)) {
        //     log::info!("[EmaCalculator] Using cached EMA buffer for period {}", period.value());
        //     return Ok(ComputeResult {
        //         output_buffer: cached_buffer.clone(),
        //         element_count,
        //     });
        // }
        log::info!("[EmaCalculator] Cache disabled for debugging - computing fresh EMA {}", period.value());

        // Update params
        let params = EmaParams {
            element_count,
            period: period.value(),
            alpha_numerator: 2,
            alpha_denominator: period.value() + 1,
        };
        
        log::info!("[EmaCalculator] Creating params buffer - period: {}, alpha_denominator: {}, element_count: {}", 
            params.period, params.alpha_denominator, params.element_count);

        // Create a fresh params buffer for this specific EMA calculation to avoid stale data
        let params_buffer = self.infrastructure.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("EMA {} Params Buffer", period.value())),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create output buffer
        let output_buffer = self.infrastructure.create_compute_buffer(
            (element_count * 4) as u64, // f32 = 4 bytes
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
            &format!("EMA {} Output Buffer", period.value()),
        );

        // Create bind group
        let bind_group = self
            .infrastructure
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("EMA {} Compute Bind Group", period.value())),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: price_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

        // Execute compute pass (single workgroup for sequential calculation)
        self.infrastructure
            .execute_compute(encoder, &self.pipeline, &bind_group, (1, 1, 1));

        // Cache the result
        self.cache.insert((period, element_count), output_buffer.clone());

        Ok(ComputeResult {
            output_buffer,
            element_count,
        })
    }

    /// Calculate multiple EMAs at once from price buffer
    pub fn calculate_multiple(
        &mut self,
        price_buffer: &wgpu::Buffer,
        element_count: u32,
        periods: &[EmaPeriod],
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<HashMap<EmaPeriod, ComputeResult>, String> {
        if periods.is_empty() {
            return Ok(HashMap::new());
        }

        // Update params (element_count is shared)
        let params = EmaParams {
            element_count,
            period: 0, // Will be set per workgroup in shader
            alpha_numerator: 2,
            alpha_denominator: 1, // Will be calculated in shader
        };

        self.infrastructure.queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::cast_slice(&[params]),
        );

        // Create a large output buffer for all EMAs
        let total_size = (element_count * 4 * 5) as u64; // 5 EMAs max, f32 = 4 bytes
        let output_buffer = self.infrastructure.create_compute_buffer(
            total_size,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC,
            "Multi-EMA Output Buffer",
        );

        // Create bind group
        let bind_group = self
            .infrastructure
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Multi-EMA Compute Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: price_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.params_buffer.as_entire_binding(),
                    },
                ],
            });

        // Execute compute pass (5 workgroups, one for each EMA period)
        self.infrastructure
            .execute_compute(encoder, &self.multi_pipeline, &bind_group, (5, 1, 1));

        // Create individual results for each period
        let mut results = HashMap::new();
        for (_i, period) in EmaPeriod::all().iter().enumerate() {
            if periods.contains(period) {
                // Create a view of the output buffer for this EMA
                // Note: In a real implementation, we'd create separate buffers or views
                // For now, we'll return the same buffer with different offsets noted
                results.insert(
                    *period,
                    ComputeResult {
                        output_buffer: output_buffer.clone(),
                        element_count,
                    },
                );
            }
        }

        Ok(results)
    }

    /// Calculate EMAs from raw price data
    pub fn calculate_from_data(
        &mut self,
        price_data: &[f32],
        periods: &[EmaPeriod],
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<HashMap<EmaPeriod, ComputeResult>, String> {
        let element_count = price_data.len() as u32;

        // Create price buffer
        let price_buffer =
            self.infrastructure
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Price Data Buffer"),
                    contents: bytemuck::cast_slice(price_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        // Calculate individual EMAs
        let mut results = HashMap::new();
        for period in periods {
            let result = self.calculate_single(&price_buffer, element_count, *period, encoder)?;
            results.insert(*period, result);
        }

        Ok(results)
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl ComputeProcessor for EmaCalculator {
    fn compute(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) -> Result<ComputeResult, String> {
        // This would be called with specific buffers
        // For now, return an error indicating it needs to be called with data
        Err("EmaCalculator requires price data buffer".to_string())
    }

    fn name(&self) -> &str {
        "EmaCalculator"
    }
}