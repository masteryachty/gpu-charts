//! Render Bundle wrapper for charting library
//! 
//! This module wraps render bundles for WebGPU/WASM compatibility
//! to reduce CPU overhead by caching static rendering commands.

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use wgpu::RenderBundleEncoderDescriptor;

/// Configuration for render bundles
#[derive(Debug, Clone)]
pub struct ChartRenderBundleConfig {
    /// Enable render bundles
    pub enabled: bool,
    /// Maximum cached bundles
    pub max_bundles: usize,
    /// Bundle lifetime in frames
    pub lifetime_frames: u32,
    /// Cache static elements (axes, grid)
    pub cache_static_elements: bool,
    /// Cache plot data
    pub cache_plot_data: bool,
}

impl Default for ChartRenderBundleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_bundles: 50,
            lifetime_frames: 300, // ~5 seconds at 60fps
            cache_static_elements: true,
            cache_plot_data: false, // Dynamic data changes frequently
        }
    }
}

/// Bundle cache key
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BundleKey {
    /// Element type (plot, x_axis, y_axis, grid)
    pub element_type: String,
    /// Viewport hash
    pub viewport_hash: u64,
    /// Data hash (for dynamic elements)
    pub data_hash: u64,
}

/// Cached render bundle with metadata
pub struct CachedBundle {
    pub bundle: wgpu::RenderBundle,
    pub created_frame: u64,
    pub last_used_frame: u64,
    pub hit_count: u32,
}

/// Render bundle system for charts
pub struct ChartRenderBundles {
    config: ChartRenderBundleConfig,
    bundles: HashMap<BundleKey, CachedBundle>,
    current_frame: u64,
    device: std::sync::Arc<wgpu::Device>,
}

impl ChartRenderBundles {
    /// Create new render bundle system
    pub fn new(device: std::sync::Arc<wgpu::Device>) -> Self {
        Self {
            config: ChartRenderBundleConfig::default(),
            bundles: HashMap::new(),
            current_frame: 0,
            device,
        }
    }
    
    /// Set configuration
    pub fn set_config(&mut self, config: ChartRenderBundleConfig) {
        let enabled = config.enabled;
        self.config = config;
        if !enabled {
            self.bundles.clear();
        }
    }
    
    /// Check if a bundle exists for the given key
    pub fn has_bundle(&self, key: &BundleKey) -> bool {
        self.config.enabled && self.bundles.contains_key(key)
    }
    
    /// Get statistics for render bundles
    pub fn get_cache_stats(&self) -> serde_json::Value {
        let total_hits: u32 = self.bundles.values().map(|b| b.hit_count).sum();
        let hit_rate = if self.bundles.is_empty() {
            0.0
        } else {
            total_hits as f32 / self.bundles.len() as f32
        };
        
        serde_json::json!({
            "enabled": self.config.enabled,
            "total_bundles": self.bundles.len(),
            "cache_hits": total_hits,
            "hit_rate": hit_rate,
            "avg_bundle_age": self.get_stats().avg_bundle_age,
            "current_frame": self.current_frame,
        })
    }
    
    /// Advance frame counter
    pub fn advance_frame(&mut self) {
        self.current_frame += 1;
    }
    
    /// Invalidate bundles matching a pattern
    pub fn invalidate_bundles<F>(&mut self, predicate: F) 
    where
        F: Fn(&BundleKey) -> bool,
    {
        self.bundles.retain(|key, _| !predicate(key));
    }
    
    /// Invalidate all bundles
    pub fn invalidate_all(&mut self) {
        self.bundles.clear();
        log::info!("Invalidated all render bundles");
    }
    
    /// Clean up old bundles
    fn cleanup_old_bundles(&mut self) {
        // Remove bundles that haven't been used recently
        let lifetime = self.config.lifetime_frames as u64;
        let current_frame = self.current_frame;
        
        self.bundles.retain(|key, bundle| {
            let age = current_frame - bundle.last_used_frame;
            if age > lifetime {
                log::debug!("Evicting old bundle {:?} (age: {} frames)", key, age);
                false
            } else {
                true
            }
        });
        
        // Enforce max bundle limit
        if self.bundles.len() > self.config.max_bundles {
            // Sort by last used frame and remove oldest
            let mut bundles: Vec<_> = self.bundles.iter()
                .map(|(k, v)| (k.clone(), v.last_used_frame))
                .collect();
            bundles.sort_by_key(|(_, frame)| *frame);
            
            let to_remove = bundles.len() - self.config.max_bundles;
            for (key, _) in bundles.iter().take(to_remove) {
                self.bundles.remove(key);
                log::debug!("Evicting bundle {:?} due to cache size limit", key);
            }
        }
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> RenderBundleStats {
        let total_bundles = self.bundles.len();
        let total_hits: u32 = self.bundles.values().map(|b| b.hit_count).sum();
        let avg_age = if total_bundles > 0 {
            let total_age: u64 = self.bundles.values()
                .map(|b| self.current_frame - b.created_frame)
                .sum();
            total_age as f32 / total_bundles as f32
        } else {
            0.0
        };
        
        RenderBundleStats {
            total_bundles,
            total_hits,
            current_frame: self.current_frame,
            avg_bundle_age: avg_age,
            cache_enabled: self.config.enabled,
        }
    }
}

/// Render bundle statistics
#[derive(Debug, Clone)]
pub struct RenderBundleStats {
    pub total_bundles: usize,
    pub total_hits: u32,
    pub current_frame: u64,
    pub avg_bundle_age: f32,
    pub cache_enabled: bool,
}

/// Helper to compute viewport hash
pub fn compute_viewport_hash(start_x: u32, end_x: u32, width: u32, height: u32) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    start_x.hash(&mut hasher);
    end_x.hash(&mut hasher);
    width.hash(&mut hasher);
    height.hash(&mut hasher);
    hasher.finish()
}

/// Helper to compute data hash
pub fn compute_data_hash(data_len: u32, min_y: f32, max_y: f32) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    data_len.hash(&mut hasher);
    min_y.to_bits().hash(&mut hasher);
    max_y.to_bits().hash(&mut hasher);
    hasher.finish()
}