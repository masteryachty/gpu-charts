//! Compute engine for managing GPU compute operations on metrics
//! This handles all pre-render computations like mid price, moving averages, etc.

use crate::compute::MidPriceCalculator;
use data_manager::{
    data_store::MetricRef,
    DataStore,
};
use config_system::ComputeOp;
use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{CommandEncoder, Device, Queue};

/// Manages all compute operations for metrics
pub struct ComputeEngine {
    _device: Rc<Device>,
    _queue: Rc<Queue>,

    // Compute calculators
    mid_price_calculator: Option<MidPriceCalculator>,

    // Track which metrics have been computed this frame
    computed_metrics: HashMap<MetricRef, u64>, // metric_ref -> compute_version
}

impl ComputeEngine {
    /// Create a new compute engine
    pub fn new(device: Rc<Device>, queue: Rc<Queue>) -> Self {
        let mid_price_calculator = MidPriceCalculator::new(device.clone(), queue.clone()).ok();

        Self {
            _device: device,
            _queue: queue,
            mid_price_calculator,
            computed_metrics: HashMap::new(),
        }
    }

    /// Run all necessary compute passes before rendering
    /// This should be called BEFORE min/max calculation
    pub fn run_compute_passes(&mut self, encoder: &mut CommandEncoder, data_store: &mut DataStore) {
        log::debug!("[ComputeEngine] Starting compute passes...");

        // Get all metrics that need computation
        let metrics_to_compute = data_store.get_metrics_needing_computation();

        if metrics_to_compute.is_empty() {
            log::debug!("[ComputeEngine] No metrics need computation");
            return;
        }

        log::debug!(
            "[ComputeEngine] Found {} metrics needing computation",
            metrics_to_compute.len()
        );

        // Sort metrics by dependency order (simple topological sort)
        let sorted_metrics = self.sort_by_dependencies(&metrics_to_compute, data_store);

        // Process each metric
        for metric_ref in sorted_metrics {
            self.compute_metric(encoder, data_store, &metric_ref);
        }

        log::debug!("[ComputeEngine] Compute passes complete");
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
                    log::debug!(
                        "[ComputeEngine] Metric already computed this frame: {}",
                        metric.name
                    );
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

        log::debug!("[ComputeEngine] Computing metric: {name}");

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
            ComputeOp::WeightedAverage { weights } => {
                log::warn!(
                    "[ComputeEngine] Weighted average with {} weights not yet implemented",
                    weights.len()
                );
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

        log::debug!("[ComputeEngine] Computing mid price for {element_count} elements");

        // Compute mid price
        match calculator.calculate(dep_buffers[0], dep_buffers[1], element_count, encoder) {
            Ok(result) => {
                // Create a temporary vector to collect computed values
                // In a real implementation, we'd schedule async readback
                let computed_values = vec![0.0f32; element_count as usize];

                // Update the metric
                if let Some(metric) = data_store.get_metric_mut(metric_ref) {
                    metric.set_computed_data(result.output_buffer, computed_values);

                    // Track that we computed this metric
                    self.computed_metrics
                        .insert(*metric_ref, metric.compute_version);

                    log::debug!("[ComputeEngine] Successfully computed mid price");
                }
            }
            Err(e) => {
                log::error!("[ComputeEngine] Failed to compute mid price: {e}");
            }
        }
    }

    /// Clear computed metrics tracking (call at start of frame)
    pub fn clear_frame_cache(&mut self) {
        self.computed_metrics.clear();
    }
}
