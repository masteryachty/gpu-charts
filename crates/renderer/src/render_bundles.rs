//! Render bundles for caching static rendering commands
//!
//! This module implements render bundle caching to reduce CPU overhead
//! by pre-recording rendering commands that don't change frequently.

use crate::{GpuBufferSet, Viewport};
use gpu_charts_shared::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// Configuration for render bundle system
#[derive(Debug, Clone)]
pub struct RenderBundleConfig {
    /// Maximum number of cached bundles
    pub max_bundles: usize,
    /// Enable automatic bundle invalidation
    pub auto_invalidate: bool,
    /// Bundle lifetime in frames
    pub lifetime_frames: u32,
}

impl Default for RenderBundleConfig {
    fn default() -> Self {
        Self {
            max_bundles: 100,
            auto_invalidate: true,
            lifetime_frames: 300, // 5 seconds at 60 FPS
        }
    }
}

/// Key for identifying render bundles
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BundleKey {
    /// Data handle ID
    pub data_id: uuid::Uuid,
    /// Viewport hash
    pub viewport_hash: u64,
    /// Render configuration hash
    pub config_hash: u64,
    /// Quality level
    pub quality_level: u32,
}

/// Cached render bundle with metadata
pub struct CachedBundle {
    /// The render bundle
    pub bundle: wgpu::RenderBundle,
    /// Creation frame
    pub created_frame: u64,
    /// Last used frame
    pub last_used_frame: u64,
    /// Number of times used
    pub use_count: u32,
    /// Bundle statistics
    pub stats: BundleStats,
}

/// Statistics for a render bundle
#[derive(Debug, Clone, Default)]
pub struct BundleStats {
    pub draw_calls: u32,
    pub vertices: u32,
    pub instances: u32,
    pub recording_time_ms: f32,
}

/// Render bundle system for caching rendering commands
pub struct RenderBundleSystem {
    device: Arc<wgpu::Device>,
    config: RenderBundleConfig,

    /// Cached bundles
    bundles: HashMap<BundleKey, CachedBundle>,

    /// Bundle encoder configuration
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,

    /// Current frame counter
    current_frame: u64,

    /// Performance metrics
    metrics: BundleMetrics,
}

/// Performance metrics for bundle system
#[derive(Debug, Default)]
struct BundleMetrics {
    cache_hits: u64,
    cache_misses: u64,
    total_bundles_created: u64,
    total_recording_time_ms: f32,
}

impl RenderBundleSystem {
    /// Create new render bundle system
    pub fn new(
        device: Arc<wgpu::Device>,
        config: RenderBundleConfig,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        Self {
            device,
            config,
            bundles: HashMap::new(),
            color_format,
            depth_format,
            current_frame: 0,
            metrics: BundleMetrics::default(),
        }
    }

    /// Get or create a render bundle (returns whether it was cached)
    pub fn get_or_create_bundle<F>(&mut self, key: BundleKey, create_fn: F) -> Result<bool>
    where
        F: FnOnce(&mut wgpu::RenderBundleEncoder) -> Result<BundleStats>,
    {
        self.current_frame += 1;

        // Check cache
        if self.bundles.contains_key(&key) {
            if let Some(cached) = self.bundles.get_mut(&key) {
                cached.last_used_frame = self.current_frame;
                cached.use_count += 1;
                self.metrics.cache_hits += 1;
            }
            return Ok(true); // Was cached
        }

        // Cache miss - create new bundle
        self.metrics.cache_misses += 1;

        // Evict old bundles if necessary
        if self.bundles.len() >= self.config.max_bundles {
            self.evict_oldest();
        }

        // Record new bundle
        let start_time = std::time::Instant::now();

        let encoder_desc = wgpu::RenderBundleEncoderDescriptor {
            label: Some("Render Bundle Encoder"),
            color_formats: &[Some(self.color_format)],
            depth_stencil: self
                .depth_format
                .map(|format| wgpu::RenderBundleDepthStencil {
                    format,
                    depth_read_only: false,
                    stencil_read_only: false,
                }),
            sample_count: 1,
            multiview: None,
        };
        let mut encoder = self.device.create_render_bundle_encoder(&encoder_desc);
        let stats = create_fn(&mut encoder)?;
        let bundle = encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some("Cached Render Bundle"),
        });

        let recording_time_ms = start_time.elapsed().as_secs_f32() * 1000.0;

        // Update metrics
        self.metrics.total_bundles_created += 1;
        self.metrics.total_recording_time_ms += recording_time_ms;

        // Cache the bundle
        let cached = CachedBundle {
            bundle,
            created_frame: self.current_frame,
            last_used_frame: self.current_frame,
            use_count: 1,
            stats: BundleStats {
                recording_time_ms,
                ..stats
            },
        };

        self.bundles.insert(key, cached);

        Ok(false) // Was not cached
    }

    /// Execute a cached bundle
    pub fn execute_bundle<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        key: &BundleKey,
    ) -> bool {
        if let Some(cached) = self.bundles.get(key) {
            render_pass.execute_bundles(std::iter::once(&cached.bundle));
            true
        } else {
            false
        }
    }

    /// Invalidate bundles matching a predicate
    pub fn invalidate_bundles<F>(&mut self, predicate: F)
    where
        F: Fn(&BundleKey) -> bool,
    {
        self.bundles.retain(|key, _| !predicate(key));
    }

    /// Invalidate all bundles for a specific data handle
    pub fn invalidate_data(&mut self, data_id: &uuid::Uuid) {
        self.invalidate_bundles(|key| &key.data_id == data_id);
    }

    /// Clean up expired bundles
    pub fn cleanup(&mut self) {
        if !self.config.auto_invalidate {
            return;
        }

        let expiry_frame = self
            .current_frame
            .saturating_sub(self.config.lifetime_frames as u64);

        self.bundles
            .retain(|_, cached| cached.last_used_frame > expiry_frame);
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> serde_json::Value {
        let total_requests = self.metrics.cache_hits + self.metrics.cache_misses;
        let hit_rate = if total_requests > 0 {
            self.metrics.cache_hits as f32 / total_requests as f32
        } else {
            0.0
        };

        serde_json::json!({
            "cache_size": self.bundles.len(),
            "cache_hits": self.metrics.cache_hits,
            "cache_misses": self.metrics.cache_misses,
            "hit_rate": hit_rate,
            "total_bundles_created": self.metrics.total_bundles_created,
            "avg_recording_time_ms": if self.metrics.total_bundles_created > 0 {
                self.metrics.total_recording_time_ms / self.metrics.total_bundles_created as f32
            } else {
                0.0
            },
            "current_frame": self.current_frame,
        })
    }

    /// Evict the oldest bundle
    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self
            .bundles
            .iter()
            .min_by_key(|(_, cached)| cached.last_used_frame)
            .map(|(k, v)| (k.clone(), v.last_used_frame))
        {
            self.bundles.remove(&oldest_key);
        }
    }
}

/// Helper for creating bundle keys
impl BundleKey {
    pub fn new(
        data_id: uuid::Uuid,
        viewport: &Viewport,
        config_hash: u64,
        quality_level: u32,
    ) -> Self {
        // Hash viewport parameters
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        // Hash relevant viewport properties
        (viewport.time_range.start, viewport.time_range.end).hash(&mut hasher);
        (viewport.width as u32, viewport.height as u32).hash(&mut hasher);
        viewport.zoom_level.to_bits().hash(&mut hasher);

        Self {
            data_id,
            viewport_hash: hasher.finish(),
            config_hash,
            quality_level,
        }
    }
}

/// Specialized bundle system for chart rendering
pub struct ChartBundleSystem {
    base: RenderBundleSystem,

    /// Pipeline states for different chart types
    pipelines: HashMap<String, wgpu::RenderPipeline>,
}

impl ChartBundleSystem {
    pub fn new(
        device: Arc<wgpu::Device>,
        config: RenderBundleConfig,
        color_format: wgpu::TextureFormat,
    ) -> Self {
        let base = RenderBundleSystem::new(device, config, color_format, None);

        Self {
            base,
            pipelines: HashMap::new(),
        }
    }

    /// Register a pipeline for a chart type
    pub fn register_pipeline(&mut self, chart_type: &str, pipeline: wgpu::RenderPipeline) {
        self.pipelines.insert(chart_type.to_string(), pipeline);
    }

    /// Create a bundle for line chart rendering
    pub fn create_line_chart_bundle(
        &mut self,
        key: BundleKey,
        _buffer_set: &GpuBufferSet,
        vertex_count: u32,
    ) -> Result<bool> {
        // For render bundles, we can't reference external buffers.
        // The bundle needs to be created with the actual rendering commands
        // This is a simplified version that just tracks whether a bundle exists
        self.base.get_or_create_bundle(key, |_encoder| {
            // In a real implementation, we would record actual draw commands here
            // For now, just return stats
            Ok(BundleStats {
                draw_calls: 1,
                vertices: vertex_count,
                instances: 1,
                recording_time_ms: 0.0,
            })
        })
    }

    /// Create a bundle for candlestick chart rendering
    pub fn create_candlestick_bundle(
        &mut self,
        key: BundleKey,
        _buffer_set: &GpuBufferSet,
        candle_count: u32,
    ) -> Result<bool> {
        // Simplified version for compilation
        self.base.get_or_create_bundle(key, |_encoder| {
            Ok(BundleStats {
                draw_calls: 1,
                vertices: candle_count * 6,
                instances: 1,
                recording_time_ms: 0.0,
            })
        })
    }
}

/// Bundle optimizer for analyzing and improving bundle usage
pub struct BundleOptimizer {
    /// Threshold for considering a bundle "hot"
    hot_threshold: u32,
    /// Threshold for considering a bundle "cold"
    cold_threshold: u32,
}

impl BundleOptimizer {
    pub fn new() -> Self {
        Self {
            hot_threshold: 100,
            cold_threshold: 10,
        }
    }

    /// Analyze bundle usage and provide recommendations
    pub fn analyze(&self, system: &RenderBundleSystem) -> BundleAnalysis {
        let mut hot_bundles = 0;
        let mut cold_bundles = 0;
        let mut total_memory = 0;

        for (_, cached) in &system.bundles {
            if cached.use_count > self.hot_threshold {
                hot_bundles += 1;
            } else if cached.use_count < self.cold_threshold {
                cold_bundles += 1;
            }

            // Estimate memory usage
            total_memory += cached.stats.vertices * 32; // Rough estimate
        }

        BundleAnalysis {
            hot_bundles,
            cold_bundles,
            total_bundles: system.bundles.len(),
            estimated_memory_kb: (total_memory / 1024) as usize,
            recommendations: self.generate_recommendations(system),
        }
    }

    fn generate_recommendations(&self, system: &RenderBundleSystem) -> Vec<String> {
        let mut recommendations = Vec::new();

        let total_requests = system.metrics.cache_hits + system.metrics.cache_misses;
        if total_requests > 0 {
            let hit_rate = system.metrics.cache_hits as f32 / total_requests as f32;

            if hit_rate < 0.7 {
                recommendations.push(
                    "Low cache hit rate. Consider increasing max_bundles or adjusting lifetime_frames.".into()
                );
            }
        }

        if system.bundles.len() as f32 > system.config.max_bundles as f32 * 0.9 {
            recommendations.push(
                "Cache nearly full. Consider increasing max_bundles to avoid evictions.".into(),
            );
        }

        recommendations
    }
}

/// Bundle usage analysis
#[derive(Debug)]
pub struct BundleAnalysis {
    pub hot_bundles: usize,
    pub cold_bundles: usize,
    pub total_bundles: usize,
    pub estimated_memory_kb: usize,
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_key_hashing() {
        let viewport1 = Viewport {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            zoom_level: 1.0,
            time_range: gpu_charts_shared::TimeRange::new(0, 1000),
        };

        let viewport2 = Viewport {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            zoom_level: 1.0,
            time_range: gpu_charts_shared::TimeRange::new(0, 2000), // Different range
        };

        let id = uuid::Uuid::new_v4();
        let key1 = BundleKey::new(id, &viewport1, 12345, 1);
        let key2 = BundleKey::new(id, &viewport2, 12345, 1);

        assert_ne!(key1.viewport_hash, key2.viewport_hash);
    }
}
