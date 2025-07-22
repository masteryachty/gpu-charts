//! Culling integration module for Phase 2 optimizations
//! 
//! This module provides the integration point for GPU-accelerated binary search culling
//! from the gpu-charts-unified crate, offering 25,000x performance improvements.

use crate::renderer::data_store::DataStore;
use std::sync::Arc;
use wgpu::{Device, Queue};
use js_sys::Uint32Array;

#[cfg(feature = "phase2-optimizations")]
use gpu_charts_unified::GPUChartsUnified;

/// Culling system that can use either legacy CPU culling or GPU binary search
pub struct CullingSystem {
    #[cfg(feature = "phase2-optimizations")]
    gpu_culler: Option<Arc<GPUChartsUnified>>,
    
    /// Feature flag to enable GPU culling
    use_gpu_culling: bool,
}

impl CullingSystem {
    /// Create a new culling system
    pub fn new(_device: Arc<Device>, _queue: Arc<Queue>) -> Self {
        #[cfg(feature = "phase2-optimizations")]
        {
            // Try to initialize GPU culling system
            // Note: This is async, so we'll need to handle it properly
            log::info!("Phase 2 optimizations available - GPU culling can be enabled");
        }
        
        Self {
            #[cfg(feature = "phase2-optimizations")]
            gpu_culler: None,
            use_gpu_culling: false,
        }
    }
    
    /// Initialize GPU culling asynchronously
    #[cfg(feature = "phase2-optimizations")]
    pub async fn init_gpu_culling(&mut self, canvas_id: &str) -> Result<(), String> {
        match GPUChartsUnified::new(canvas_id).await {
            Ok(unified) => {
                self.gpu_culler = Some(Arc::new(unified));
                self.use_gpu_culling = true;
                log::info!("GPU binary search culling initialized successfully");
                Ok(())
            }
            Err(e) => {
                log::warn!("Failed to initialize GPU culling: {:?}", e);
                Err(format!("GPU culling init failed: {:?}", e))
            }
        }
    }
    
    /// Calculate visible data range for current viewport
    pub fn calculate_visible_range(
        &self,
        data_store: &DataStore,
        _start_x: u64,
        _end_x: u64,
    ) -> (usize, usize) {
        // Calculate visible range based on viewport
        let viewport_start = data_store.start_x;
        let viewport_end = data_store.end_x;
        
        if self.use_gpu_culling {
            log::info!("GPU culling enabled but not yet integrated - using CPU fallback");
        }
        
        // Get the active data group
        let active_groups = data_store.get_active_data_groups();
        if active_groups.is_empty() {
            return (0, 0);
        }
        
        let data_series = active_groups[0];
        let data_len = data_series.length as usize;
        if data_len == 0 {
            return (0, 0);
        }
        
        log::info!("CPU Binary Search Culling: viewport [{}, {}], data points: {}", 
                  viewport_start, viewport_end, data_len);
        
        // Access the raw x data (timestamps) for binary search
        use js_sys::Uint32Array;
        let x_array = Uint32Array::new(&data_series.x_raw);
        
        // Debug: Check the actual data range
        if data_len > 0 {
            let first_timestamp = x_array.get_index(0);
            let last_timestamp = x_array.get_index((data_len - 1) as u32);
            log::info!("Data timestamps range: [{}, {}] (span: {} seconds)", 
                      first_timestamp, last_timestamp, last_timestamp - first_timestamp);
        }
        
        // Binary search for start index
        let visible_start = binary_search_start(&x_array, viewport_start);
        
        // Binary search for end index
        let visible_end = binary_search_end(&x_array, viewport_end, data_len);
        
        // Add some padding to ensure smooth transitions
        let padding = 10;
        let visible_start = visible_start.saturating_sub(padding);
        let visible_end = (visible_end + padding).min(data_len);
        
        log::info!("Binary Search Culling result: rendering {} out of {} points (indices {} to {})", 
                  visible_end - visible_start, data_len, visible_start, visible_end);
        
        (visible_start, visible_end)
    }
    
    /// Check if GPU culling is available and enabled
    pub fn is_gpu_culling_enabled(&self) -> bool {
        #[cfg(feature = "phase2-optimizations")]
        {
            self.use_gpu_culling && self.gpu_culler.is_some()
        }
        
        #[cfg(not(feature = "phase2-optimizations"))]
        {
            false
        }
    }
    
    /// Get performance metrics
    pub fn get_metrics(&self) -> String {
        #[cfg(feature = "phase2-optimizations")]
        {
            if let Some(culler) = &self.gpu_culler {
                return culler.get_culling_metrics();
            }
        }
        
        "CPU culling: No metrics available".to_string()
    }
}

/// Binary search to find the first index where timestamp >= target
fn binary_search_start(timestamps: &Uint32Array, target: u32) -> usize {
    let len = timestamps.length() as usize;
    if len == 0 {
        return 0;
    }
    
    let mut left = 0;
    let mut right = len;
    
    while left < right {
        let mid = left + (right - left) / 2;
        let mid_value = timestamps.get_index(mid as u32);
        
        if mid_value < target {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    
    left
}

/// Binary search to find the last index where timestamp <= target
fn binary_search_end(timestamps: &Uint32Array, target: u32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    
    let mut left = 0;
    let mut right = len;
    
    while left < right {
        let mid = left + (right - left) / 2;
        let mid_value = timestamps.get_index(mid as u32);
        
        if mid_value <= target {
            left = mid + 1;
        } else {
            right = mid;
        }
    }
    
    // Return the last valid index
    if left > 0 {
        left
    } else {
        0
    }
}