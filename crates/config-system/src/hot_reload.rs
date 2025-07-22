//! Hot-reload system for zero-downtime configuration updates

use crate::{ConfigError, GpuChartsConfig, Result};
use arc_swap::ArcSwap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

/// Configuration update event
#[derive(Debug, Clone)]
pub struct ConfigUpdateEvent {
    pub timestamp: Instant,
    pub old_version: String,
    pub new_version: String,
    pub changed_fields: Vec<String>,
}

/// Hot-reload configuration manager
pub struct HotReloadManager {
    /// Current configuration wrapped in ArcSwap for lock-free reads
    current_config: Arc<ArcSwap<GpuChartsConfig>>,

    /// Configuration history for rollback
    history: Arc<RwLock<Vec<(Instant, Arc<GpuChartsConfig>)>>>,

    /// Update event broadcaster
    update_tx: broadcast::Sender<ConfigUpdateEvent>,

    /// Validation function
    validator: Arc<dyn Fn(&GpuChartsConfig) -> Result<()> + Send + Sync>,

    /// Maximum history size
    max_history: usize,
}

impl HotReloadManager {
    /// Create a new hot-reload manager
    pub fn new(
        initial_config: GpuChartsConfig,
        validator: impl Fn(&GpuChartsConfig) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        let (update_tx, _) = broadcast::channel(100);

        Self {
            current_config: Arc::new(ArcSwap::from_pointee(initial_config.clone())),
            history: Arc::new(RwLock::new(vec![(
                Instant::now(),
                Arc::new(initial_config),
            )])),
            update_tx,
            validator: Arc::new(validator),
            max_history: 10,
        }
    }

    /// Get the current configuration
    pub fn current(&self) -> Arc<GpuChartsConfig> {
        self.current_config.load_full()
    }

    /// Update configuration with validation and rollback support
    pub async fn update(&self, new_config: GpuChartsConfig) -> Result<()> {
        // Validate the new configuration
        (self.validator)(&new_config)?;

        let old_config = self.current_config.load_full();

        // Check if update is needed
        if self.config_equals(&old_config, &new_config) {
            return Ok(());
        }

        // Calculate changes
        let changed_fields = self.calculate_changes(&old_config, &new_config);

        // Create update event
        let event = ConfigUpdateEvent {
            timestamp: Instant::now(),
            old_version: old_config.version.clone(),
            new_version: new_config.version.clone(),
            changed_fields,
        };

        // Update configuration atomically
        let new_config_arc = Arc::new(new_config);
        self.current_config.store(new_config_arc.clone());

        // Add to history
        {
            let mut history = self.history.write();
            history.push((Instant::now(), new_config_arc));

            // Trim history if needed
            if history.len() > self.max_history {
                history.remove(0);
            }
        }

        // Broadcast update event
        let _ = self.update_tx.send(event);

        Ok(())
    }

    /// Rollback to a previous configuration
    pub async fn rollback(&self, steps: usize) -> Result<()> {
        let history = self.history.read();

        if steps >= history.len() {
            return Err(ConfigError::HotReload(
                "Not enough history for rollback".to_string(),
            ));
        }

        let target_idx = history.len() - 1 - steps;
        let target_config = history[target_idx].1.clone();

        drop(history);

        // Update to the target configuration
        self.current_config.store(target_config);

        Ok(())
    }

    /// Subscribe to configuration updates
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigUpdateEvent> {
        self.update_tx.subscribe()
    }

    /// Get configuration history
    pub fn get_history(&self) -> Vec<(Instant, Arc<GpuChartsConfig>)> {
        self.history.read().clone()
    }

    /// Calculate configuration differences
    fn calculate_changes(&self, old: &GpuChartsConfig, new: &GpuChartsConfig) -> Vec<String> {
        let mut changes = Vec::new();

        // Check version
        if old.version != new.version {
            changes.push("version".to_string());
        }

        // Check rendering config
        if !self.rendering_equals(&old.rendering, &new.rendering) {
            changes.push("rendering".to_string());
        }

        // Check data config
        if !self.data_equals(&old.data, &new.data) {
            changes.push("data".to_string());
        }

        // Check performance config
        if !self.performance_equals(&old.performance, &new.performance) {
            changes.push("performance".to_string());
        }

        // Check features
        if !self.features_equals(&old.features, &new.features) {
            changes.push("features".to_string());
        }

        // Check telemetry
        if !self.telemetry_equals(&old.telemetry, &new.telemetry) {
            changes.push("telemetry".to_string());
        }

        changes
    }

    /// Check if two configurations are equal
    fn config_equals(&self, a: &GpuChartsConfig, b: &GpuChartsConfig) -> bool {
        // Serialize and compare for deep equality
        match (serde_json::to_string(a), serde_json::to_string(b)) {
            (Ok(a_str), Ok(b_str)) => a_str == b_str,
            _ => false,
        }
    }

    fn rendering_equals(&self, a: &crate::RenderingConfig, b: &crate::RenderingConfig) -> bool {
        a.target_fps == b.target_fps
            && (a.resolution_scale - b.resolution_scale).abs() < f32::EPSILON
            && a.antialiasing == b.antialiasing
            && a.vsync == b.vsync
            && a.max_render_passes == b.max_render_passes
            && a.gpu_memory_limit == b.gpu_memory_limit
    }

    fn data_equals(&self, a: &crate::DataConfig, b: &crate::DataConfig) -> bool {
        a.cache_size == b.cache_size
            && a.prefetch_enabled == b.prefetch_enabled
            && (a.prefetch_distance - b.prefetch_distance).abs() < f32::EPSILON
    }

    fn performance_equals(
        &self,
        a: &crate::PerformanceConfig,
        b: &crate::PerformanceConfig,
    ) -> bool {
        a.gpu_culling == b.gpu_culling
            && a.lod_enabled == b.lod_enabled
            && (a.lod_bias - b.lod_bias).abs() < f32::EPSILON
            && a.vertex_compression == b.vertex_compression
            && a.indirect_drawing == b.indirect_drawing
            && a.draw_call_batch_size == b.draw_call_batch_size
    }

    fn features_equals(&self, a: &crate::FeatureFlags, b: &crate::FeatureFlags) -> bool {
        a.scatter_plots == b.scatter_plots
            && a.heatmaps == b.heatmaps
            && a.three_d_charts == b.three_d_charts
            && a.technical_indicators == b.technical_indicators
            && a.annotations == b.annotations
            && a.custom_shaders == b.custom_shaders
            && a.experimental_features == b.experimental_features
    }

    fn telemetry_equals(&self, a: &crate::TelemetryConfig, b: &crate::TelemetryConfig) -> bool {
        a.enabled == b.enabled
            && a.performance_tracking == b.performance_tracking
            && a.error_reporting == b.error_reporting
            && a.usage_analytics == b.usage_analytics
            && a.custom_events == b.custom_events
            && (a.sampling_rate - b.sampling_rate).abs() < f32::EPSILON
    }
}

/// Configuration diff calculator for efficient updates
pub struct ConfigDiffer {
    /// Cached configuration for comparison
    cached_config: Option<Arc<GpuChartsConfig>>,
}

impl ConfigDiffer {
    pub fn new() -> Self {
        Self {
            cached_config: None,
        }
    }

    /// Calculate minimal update operations needed
    pub fn calculate_diff(&mut self, new_config: Arc<GpuChartsConfig>) -> ConfigDiff {
        let diff = match &self.cached_config {
            Some(old) => ConfigDiff {
                rendering_changed: !self.rendering_equals(&old.rendering, &new_config.rendering),
                data_changed: !self.data_equals(&old.data, &new_config.data),
                performance_changed: !self
                    .performance_equals(&old.performance, &new_config.performance),
                features_changed: !self.features_equals(&old.features, &new_config.features),
                full_reload_required: old.version != new_config.version,
            },
            None => ConfigDiff {
                rendering_changed: true,
                data_changed: true,
                performance_changed: true,
                features_changed: true,
                full_reload_required: true,
            },
        };

        self.cached_config = Some(new_config);
        diff
    }

    fn rendering_equals(&self, a: &crate::RenderingConfig, b: &crate::RenderingConfig) -> bool {
        // Use faster comparison for hot path
        std::ptr::eq(a, b)
            || (a.target_fps == b.target_fps
                && a.resolution_scale == b.resolution_scale
                && a.antialiasing == b.antialiasing
                && a.vsync == b.vsync)
    }

    fn data_equals(&self, a: &crate::DataConfig, b: &crate::DataConfig) -> bool {
        std::ptr::eq(a, b)
            || (a.cache_size == b.cache_size && a.prefetch_enabled == b.prefetch_enabled)
    }

    fn performance_equals(
        &self,
        a: &crate::PerformanceConfig,
        b: &crate::PerformanceConfig,
    ) -> bool {
        std::ptr::eq(a, b)
            || (a.gpu_culling == b.gpu_culling
                && a.lod_enabled == b.lod_enabled
                && a.vertex_compression == b.vertex_compression)
    }

    fn features_equals(&self, a: &crate::FeatureFlags, b: &crate::FeatureFlags) -> bool {
        std::ptr::eq(a, b)
            || (a.scatter_plots == b.scatter_plots
                && a.heatmaps == b.heatmaps
                && a.three_d_charts == b.three_d_charts)
    }
}

/// Configuration difference result
#[derive(Debug, Clone)]
pub struct ConfigDiff {
    pub rendering_changed: bool,
    pub data_changed: bool,
    pub performance_changed: bool,
    pub features_changed: bool,
    pub full_reload_required: bool,
}

impl ConfigDiff {
    /// Check if any change requires action
    pub fn has_changes(&self) -> bool {
        self.rendering_changed
            || self.data_changed
            || self.performance_changed
            || self.features_changed
            || self.full_reload_required
    }
}
