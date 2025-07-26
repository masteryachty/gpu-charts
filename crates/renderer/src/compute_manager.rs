//! Centralized compute management for GPU-calculated metrics
//! This runs BEFORE min/max calculation to ensure all buffers are populated

use std::rc::Rc;
use wgpu::{CommandEncoder, Device, Queue};
use data_manager::{DataStore, MetricRef, ComputeType};
use crate::compute::MidPriceCalculator;

/// Manages all compute operations that need to run before rendering
pub struct ComputeManager {
    device: Rc<Device>,
    queue: Rc<Queue>,
    mid_price_calculator: Option<MidPriceCalculator>,
}

impl ComputeManager {
    pub fn new(device: Rc<Device>, queue: Rc<Queue>) -> Self {
        let mid_price_calculator = MidPriceCalculator::new(device.clone(), queue.clone()).ok();
        
        Self {
            device,
            queue,
            mid_price_calculator,
        }
    }
    
    /// Run all compute passes needed for the current data
    /// This should be called BEFORE min/max calculation
    pub fn run_compute_passes(
        &mut self,
        encoder: &mut CommandEncoder,
        data_store: &mut DataStore,
    ) {
        log::info!("[ComputeManager] Running compute passes...");
        
        // Get all metrics that need computation
        let metrics_to_compute = data_store.get_metrics_needing_computation();
        
        if metrics_to_compute.is_empty() {
            log::info!("[ComputeManager] No metrics need computation");
            return;
        }
        
        log::info!("[ComputeManager] Found {} metrics needing computation", 
            metrics_to_compute.len());
        
        // Process each metric that needs computation
        for metric_ref in metrics_to_compute {
            // Check if dependencies are ready
            if let Some(metric) = data_store.get_metric(&metric_ref) {
                if !data_store.dependencies_ready(metric) {
                    log::warn!("[ComputeManager] Skipping '{}' - dependencies not ready", 
                        metric.name);
                    continue;
                }
                
                // Get compute type and process
                if let Some(compute_type) = &metric.compute_type {
                    self.compute_metric(encoder, data_store, &metric_ref, compute_type);
                }
            }
        }
    }
    
    /// Compute a specific metric based on its type
    fn compute_metric(
        &mut self,
        encoder: &mut CommandEncoder,
        data_store: &mut DataStore,
        metric_ref: &MetricRef,
        compute_type: &ComputeType,
    ) {
        match compute_type {
            ComputeType::Average => {
                self.compute_average(encoder, data_store, metric_ref);
            }
            ComputeType::Sum => {
                log::warn!("[ComputeManager] Sum computation not yet implemented");
            }
            ComputeType::Difference => {
                log::warn!("[ComputeManager] Difference computation not yet implemented");
            }
            ComputeType::MovingAverage { period } => {
                log::warn!("[ComputeManager] Moving average computation not yet implemented (period: {})", period);
            }
            ComputeType::RSI { period } => {
                log::warn!("[ComputeManager] RSI computation not yet implemented (period: {})", period);
            }
            ComputeType::BollingerBands { period, std_dev } => {
                log::warn!("[ComputeManager] Bollinger bands computation not yet implemented (period: {}, std_dev: {})", 
                    period, std_dev);
            }
            ComputeType::Custom { shader_name } => {
                log::warn!("[ComputeManager] Custom shader computation not yet implemented: {}", 
                    shader_name);
            }
        }
    }
    
    /// Compute average of dependencies (e.g., mid price from bid/ask)
    fn compute_average(
        &mut self,
        encoder: &mut CommandEncoder,
        data_store: &mut DataStore,
        metric_ref: &MetricRef,
    ) {
        let Some(metric) = data_store.get_metric(metric_ref) else {
            log::error!("[ComputeManager] Metric not found for computation");
            return;
        };
        
        // For mid price, we expect exactly 2 dependencies (bid and ask)
        if metric.name == "mid_price" && metric.dependencies.len() == 2 {
            self.compute_mid_price_direct(encoder, data_store, metric_ref);
        } else {
            log::warn!("[ComputeManager] Average computation for '{}' with {} dependencies not implemented", 
                metric.name, metric.dependencies.len());
        }
    }
    
    /// Compute mid price from bid/ask using the new MetricSeries structure
    fn compute_mid_price_direct(
        &mut self,
        encoder: &mut CommandEncoder,
        data_store: &mut DataStore,
        metric_ref: &MetricRef,
    ) {
        let Some(calculator) = &self.mid_price_calculator else {
            log::error!("[ComputeManager] No mid price calculator available");
            return;
        };
        
        // Get the metric and its dependencies
        let metric = match data_store.get_metric(metric_ref) {
            Some(m) => m.clone(), // Clone to avoid borrow issues
            None => return,
        };
        
        // Get dependency buffers
        let dep_buffers = match data_store.get_dependency_buffers(&metric) {
            Some(buffers) => buffers,
            None => {
                log::error!("[ComputeManager] Failed to get dependency buffers for mid price");
                return;
            }
        };
        
        if dep_buffers.len() != 2 {
            log::error!("[ComputeManager] Expected 2 dependencies for mid price, got {}", 
                dep_buffers.len());
            return;
        }
        
        // Get element count from the first dependency's group
        let element_count = if let Some(dep_metric) = data_store.get_metric(&metric.dependencies[0]) {
            // Find the group containing this metric
            data_store.data_groups.iter()
                .find(|group| group.metrics.iter().any(|m| m.name == dep_metric.name))
                .map(|group| group.length)
                .unwrap_or(0)
        } else {
            0
        };
        
        if element_count == 0 {
            log::error!("[ComputeManager] No data elements found for computation");
            return;
        }
        
        log::info!("[ComputeManager] Computing mid price for {} elements", element_count);
        
        // Use the calculator to compute mid price
        match calculator.calculate(dep_buffers[0], dep_buffers[1], element_count, encoder) {
            Ok(result) => {
                // Create staging buffer for CPU readback
                let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Mid Price Staging Buffer"),
                    size: (element_count * 4) as u64, // 4 bytes per f32
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                
                // Copy GPU buffer to staging for CPU readback
                encoder.copy_buffer_to_buffer(
                    &result.output_buffer,
                    0,
                    &staging_buffer,
                    0,
                    (element_count * 4) as u64,
                );
                
                // Update the metric with computed data
                if let Some(metric) = data_store.get_metric_mut(metric_ref) {
                    // For now, just set the buffer - CPU readback will happen later
                    metric.y_buffers = vec![result.output_buffer];
                    metric.is_computed_ready = true;
                    metric.compute_version += 1;
                    
                    // TODO: Schedule async readback to populate y_raw
                    // This would involve submitting the command buffer and mapping the staging buffer
                    
                    log::info!("[ComputeManager] Successfully computed mid price and updated buffer");
                }
            }
            Err(e) => {
                log::error!("[ComputeManager] Failed to compute mid price: {}", e);
            }
        }
    }
}