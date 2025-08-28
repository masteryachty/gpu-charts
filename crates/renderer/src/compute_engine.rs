//! Compute engine for managing GPU compute operations on metrics
//! This handles all pre-render computations like mid price, moving averages, etc.

use crate::compute::{CloseExtractor, EmaCalculator, EmaPeriod, MidPriceCalculator};
use crate::buffer_pool::GpuResourceManager;
use config_system::ComputeOp;
use data_manager::{data_store::MetricRef, DataStore};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use wgpu::{CommandEncoder, Device, Queue};
use wgpu::util::DeviceExt;
use js_sys;
use bytemuck;

/// Structure to hold pending GPU readback operations
struct PendingReadback {
    staging_buffer: wgpu::Buffer,
    metric_ref: MetricRef,
    element_count: u32,
    mapping_started: bool,
    mapping_complete: Rc<RefCell<bool>>,
}

/// Manages all compute operations for metrics
pub struct ComputeEngine {
    device: Rc<Device>,
    _queue: Rc<Queue>,
    
    // GPU resource manager for optimized buffer and readback management
    resource_manager: GpuResourceManager,

    // Compute calculators
    mid_price_calculator: Option<MidPriceCalculator>,
    ema_calculator: Option<EmaCalculator>,
    close_extractor: Option<CloseExtractor>,

    // Track which metrics have been computed this frame
    computed_metrics: HashMap<MetricRef, u64>, // metric_ref -> compute_version
    
    // Cache for candle close prices buffer
    candle_close_buffer: Option<wgpu::Buffer>,
}

impl ComputeEngine {
    /// Create a new compute engine
    pub fn new(device: Rc<Device>, queue: Rc<Queue>) -> Self {
        let mid_price_calculator = MidPriceCalculator::new(device.clone(), queue.clone()).ok();
        let ema_calculator = EmaCalculator::new(device.clone(), queue.clone()).ok();
        
        // Create close extractor for candle data
        let infrastructure = Rc::new(
            crate::compute::ComputeInfrastructure::new(device.clone(), queue.clone())
        );
        let close_extractor = CloseExtractor::new(infrastructure).ok();

        Self {
            device: device.clone(),
            _queue: queue,
            resource_manager: GpuResourceManager::new(device),
            mid_price_calculator,
            ema_calculator,
            close_extractor,
            computed_metrics: HashMap::new(),
            candle_close_buffer: None,
        }
    }

    /// Set the candle buffer for EMA calculations  
    pub fn set_candle_buffer(
        &mut self, 
        candle_buffer: Option<(&wgpu::Buffer, u32)>,
        encoder: &mut CommandEncoder
    ) {
        if let Some((buffer, count)) = candle_buffer {
            // Extract close prices from candles if we have the extractor
            if let Some(extractor) = &self.close_extractor {
                match extractor.extract(buffer, count, encoder) {
                    Ok(result) => {
                        self.candle_close_buffer = Some(result.output_buffer);
                    }
                    Err(e) => {
                        log::error!("[ComputeEngine] Failed to extract close prices: {}", e);
                    }
                }
            }
        } else {
            self.candle_close_buffer = None;
        }
    }
    
    /// Run all necessary compute passes before rendering
    /// This should be called BEFORE min/max calculation
    pub fn run_compute_passes(&mut self, encoder: &mut CommandEncoder, data_store: &mut DataStore) {
        // Get all metrics that need computation
        let metrics_to_compute = data_store.get_metrics_needing_computation();

        if metrics_to_compute.is_empty() {
            return;
        }

        // Process metrics that need computation

        // Sort metrics by dependency order (simple topological sort)
        let sorted_metrics = self.sort_by_dependencies(&metrics_to_compute, data_store);


        // Process each metric
        for metric_ref in sorted_metrics {
            self.compute_metric(encoder, data_store, &metric_ref);
        }
    }

    /// Sort metrics by their dependencies to ensure correct computation order
    fn sort_by_dependencies(
        &self,
        metrics: &[MetricRef],
        data_store: &DataStore,
    ) -> Vec<MetricRef> {
        // Simple implementation - metrics with no dependencies first
        // In a more complex system, we'd do a proper topological sort
        let mut sorted = Vec::new();
        let mut remaining: Vec<MetricRef> = metrics.to_vec();

        while !remaining.is_empty() {
            let mut made_progress = false;

            remaining.retain(|metric_ref| {
                if let Some(metric) = data_store.get_metric(metric_ref) {
                    // Check if all dependencies are already computed
                    let deps_ready = metric.dependencies.iter().all(|dep| {
                        // Dependency is ready if it's not computed or already processed
                        if let Some(dep_metric) = data_store.get_metric(dep) {
                            !dep_metric.is_computed || dep_metric.is_computed_ready
                        } else {
                            false
                        }
                    });

                    if deps_ready {
                        sorted.push(*metric_ref);
                        made_progress = true;
                        false // Remove from remaining
                    } else {
                        true // Keep in remaining
                    }
                } else {
                    false // Remove invalid refs
                }
            });

            if !made_progress && !remaining.is_empty() {
                log::error!("[ComputeEngine] Circular dependency detected or missing dependencies");
                break;
            }
        }

        sorted
    }

    /// Compute a specific metric
    fn compute_metric(
        &mut self,
        encoder: &mut CommandEncoder,
        data_store: &mut DataStore,
        metric_ref: &MetricRef,
    ) {
        // Check if already computed this frame
        if let Some(&version) = self.computed_metrics.get(metric_ref) {
            if let Some(metric) = data_store.get_metric(metric_ref) {
                if metric.compute_version == version {
                    return;
                }
            }
        }

        // Get metric info
        let (name, compute_type, dependencies) = {
            match data_store.get_metric(metric_ref) {
                Some(metric) => {
                    if !metric.is_computed || metric.compute_type.is_none() {
                        log::warn!(
                            "[ComputeEngine] Metric {} is not a computed metric",
                            metric.name
                        );
                        return;
                    }

                    if !data_store.dependencies_ready(metric) {
                        log::warn!("[ComputeEngine] Dependencies not ready for {}", metric.name);
                        return;
                    }

                    (
                        metric.name.clone(),
                        metric.compute_type.clone().unwrap(),
                        metric.dependencies.clone(),
                    )
                }
                None => {
                    log::error!("[ComputeEngine] Metric not found: {metric_ref:?}");
                    return;
                }
            }
        };

        // Perform computation based on type
        match compute_type {
            ComputeOp::Average => {
                if (name == "mid_price" || name == "Mid") && dependencies.len() == 2 {
                    self.compute_mid_price(encoder, data_store, metric_ref, &dependencies);
                } else {
                    log::warn!("[ComputeEngine] Average computation for {name} not implemented");
                }
            }
            ComputeOp::Sum => {
                log::warn!("[ComputeEngine] Sum computation not yet implemented");
            }
            ComputeOp::Difference => {
                log::warn!("[ComputeEngine] Difference computation not yet implemented");
            }
            ComputeOp::Product => {
                log::warn!("[ComputeEngine] Product computation not yet implemented");
            }
            ComputeOp::Ratio => {
                log::warn!("[ComputeEngine] Ratio computation not yet implemented");
            }
            ComputeOp::Min => {
                log::warn!("[ComputeEngine] Min computation not yet implemented");
            }
            ComputeOp::Max => {
                log::warn!("[ComputeEngine] Max computation not yet implemented");
            }
            ComputeOp::WeightedAverage { weights: _ } => {
                // EMA calculations use WeightedAverage with empty weights array
                // Check for EMA patterns: "ema_9", "ema_20", etc
                if name.starts_with("ema_") {
                    // Computing EMA for metric
                    self.compute_ema(encoder, data_store, metric_ref, &name, &dependencies);
                } else {
                    log::warn!("[ComputeEngine] Weighted average for {name} not implemented");
                }
            }
        }
    }

    /// Compute mid price from bid/ask
    fn compute_mid_price(
        &mut self,
        encoder: &mut CommandEncoder,
        data_store: &mut DataStore,
        metric_ref: &MetricRef,
        dependencies: &[MetricRef],
    ) {
        
        let Some(calculator) = &self.mid_price_calculator else {
            log::error!("[ComputeEngine] No mid price calculator available");
            return;
        };

        // Get dependency buffers
        let dep_buffers: Vec<&wgpu::Buffer> = dependencies
            .iter()
            .filter_map(|dep| data_store.get_metric(dep).and_then(|m| m.y_buffers.first()))
            .collect();

        if dep_buffers.len() != 2 {
            log::error!(
                "[ComputeEngine] Expected 2 buffers for mid price, got {}",
                dep_buffers.len()
            );
            return;
        }

        // Get element count from the data group
        let element_count = data_store
            .data_groups
            .first()
            .map(|g| g.length)
            .unwrap_or(0);

        if element_count == 0 {
            log::error!("[ComputeEngine] No data elements for computation");
            return;
        }

        // Compute mid price
        match calculator.calculate(dep_buffers[0], dep_buffers[1], element_count, encoder) {
            Ok(result) => {
                log::warn!("[PERF] Skipping CPU readback for mid_price; using GPU buffer directly ({} elements)", element_count);
                // Skip CPU readback: use GPU buffer directly.
                // Previously we copied the full computed buffer to a staging buffer and scheduled
                // an async readback, which caused significant stalls (~700ms) for large datasets.
                // Rendering consumes the GPU buffer directly, and tooltips/analytics can be
                // implemented with targeted, small readbacks if ever needed.
                
                // Store the GPU buffer in the metric
                if let Some(metric) = data_store.get_metric_mut(metric_ref) {
                    // Store the GPU buffer, CPU data will be filled after readback
                    metric.set_computed_data(result.output_buffer, vec![]);
                    self.computed_metrics
                        .insert(*metric_ref, metric.compute_version);
                } else {
                    log::error!("[ComputeEngine] ❌ Failed to find metric {:?} for storing computed data", metric_ref);
                }
            }
            Err(e) => {
                log::error!("[ComputeEngine] Failed to compute mid price: {e}");
            }
        }
    }

    /// Compute EMA from price data
    fn compute_ema(
        &mut self,
        encoder: &mut CommandEncoder,
        data_store: &mut DataStore,
        metric_ref: &MetricRef,
        name: &str,
        dependencies: &[MetricRef],
    ) {
        // Computing EMA for metric
        
        let Some(calculator) = &mut self.ema_calculator else {
            log::error!("[ComputeEngine] No EMA calculator available");
            return;
        };

        // Parse EMA period from name (e.g., "EMA 20" or "ema_20" -> 20)
        let period_value = name
            .to_lowercase()
            .replace("ema", "")
            .replace("_", "")
            .trim()
            .parse::<u32>()
            .unwrap_or(0);

        log::info!("[ComputeEngine] Parsed EMA period from '{}' = {}", name, period_value);

        let period = match period_value {
            9 => EmaPeriod::Ema9,
            20 => EmaPeriod::Ema20,
            50 => EmaPeriod::Ema50,
            100 => EmaPeriod::Ema100,
            200 => EmaPeriod::Ema200,
            _ => {
                log::error!("[ComputeEngine] Invalid EMA period: {}", period_value);
                return;
            }
        };

        // Check if we have candle close prices to use instead of raw trades
        let (price_buffer, element_count) = if let Some(ref candle_close_buffer) = self.candle_close_buffer {
            let candle_count = (candle_close_buffer.size() / 4) as u32; // f32 = 4 bytes
            log::info!("[ComputeEngine] Using candle close buffer: {} candles, buffer size: {} bytes", 
                candle_count, candle_close_buffer.size());
            (candle_close_buffer, candle_count)
        } else {
            // Fall back to raw trade prices (tick-based)
            if dependencies.is_empty() {
                log::error!("[ComputeEngine] No dependencies for EMA calculation");
                return;
            }

            let price_buffer = match data_store.get_metric(&dependencies[0]) {
                Some(metric) => {
                    match metric.y_buffers.first() {
                        Some(buffer) => buffer,
                        None => {
                            log::error!("[ComputeEngine] No price buffer found for EMA");
                            return;
                        }
                    }
                },
                None => {
                    log::error!("[ComputeEngine] Price metric not found for EMA");
                    return;
                }
            };

            // Get element count from the data group
            let element_count = data_store
                .data_groups
                .first()
                .map(|g| g.length)
                .unwrap_or(0);
            
            (price_buffer, element_count)
        };

        if element_count == 0 {
            log::error!("[ComputeEngine] No data elements for EMA computation");
            return;
        }

        // Compute EMA
        log::info!("[ComputeEngine] Calling calculator.calculate_single with:");
        log::info!("  - price_buffer size: {} bytes", price_buffer.size());
        log::info!("  - element_count: {}", element_count);
        log::info!("  - period: {:?} (value={})", period, period.value());
        
        match calculator.calculate_single(price_buffer, element_count, period, encoder) {
            Ok(result) => {
                log::info!("[ComputeEngine] EMA calculation successful, output buffer size: {} bytes", 
                    result.output_buffer.size());
                
                // Create a temporary vector to collect computed values
                // In a real implementation, we'd schedule async readback
                let computed_values = vec![0.0f32; element_count as usize];

                // Check if we need to create x_buffers for candle-based EMAs first
                let group_idx = metric_ref.group_index;
                let needs_x_buffers = if self.candle_close_buffer.is_some() {
                    // Check if the group needs x_buffers
                    data_store.data_groups.get(group_idx)
                        .map(|g| g.x_buffers.is_empty())
                        .unwrap_or(false)
                } else {
                    false
                };
                
                // Create x_buffers if needed (before updating the metric)
                if needs_x_buffers {
                    // Estimate candle period from data range
                    let start = data_store.start_x;
                    let period = (data_store.end_x - data_store.start_x) / element_count.max(1);
                    
                    // Create timestamps for each candle
                    let mut timestamps = Vec::with_capacity(element_count as usize);
                    for i in 0..element_count {
                        timestamps.push(start + i * period);
                    }
                    
                    // Create JavaScript ArrayBuffer for raw data
                    let js_array = js_sys::Uint32Array::new_with_length(element_count);
                    for (i, &ts) in timestamps.iter().enumerate() {
                        js_array.set_index(i as u32, ts);
                    }
                    let x_raw = js_array.buffer();
                    
                    // Create GPU buffer
                    let x_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Candle EMA X Buffer"),
                        contents: bytemuck::cast_slice(&timestamps),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    });
                    
                    // Update the group with the new x_buffers
                    if let Some(group) = data_store.data_groups.get_mut(group_idx) {
                        group.x_buffers = vec![x_buffer];
                        group.x_raw = x_raw;
                        group.length = element_count;
                    }
                    
                    // Mark this group as active so it gets rendered
                    if !data_store.active_data_group_indices.contains(&group_idx) {
                        data_store.active_data_group_indices.push(group_idx);
                    }
                }

                // Update the metric
                if let Some(metric) = data_store.get_metric_mut(metric_ref) {
                    log::info!("[ComputeEngine] Updating metric '{}' with EMA data", metric.name);
                    metric.set_computed_data(result.output_buffer, computed_values);

                    // Track that we computed this metric
                    self.computed_metrics
                        .insert(*metric_ref, metric.compute_version);
                    
                    log::info!("[ComputeEngine] EMA {} computation complete", name);
                } else {
                    log::error!("[ComputeEngine] Failed to get metric for updating EMA");
                }
            }
            Err(e) => {
                log::error!("[ComputeEngine] ✗ Failed to compute {}: {}", name, e);
            }
        }
    }

    /// Clear computed metrics tracking (call at start of frame)
    pub fn clear_frame_cache(&mut self) {
        self.computed_metrics.clear();
    }
    
    /// Process pending GPU readbacks using optimized ring buffer
    /// This should be called periodically (e.g., each frame) to check readback status
    pub fn process_readbacks(&mut self, _data_store: &mut DataStore) {
        self.resource_manager.readback_ring.process_readbacks(&self.device);
    }
    
    /// Advance frame for resource management
    pub fn advance_frame(&mut self) {
        self.resource_manager.advance_frame(&self.device);
    }
}
