//! Bridge implementations for connecting subsystems

use crate::{IntegrationError, Result};
use gpu_charts_config::GpuChartsConfig;
use gpu_charts_data::{BufferHandle, BufferMetadata, DataManager, DataManagerConfig, DataSource};
use gpu_charts_renderer::{
    PerformanceMetrics as RendererMetrics, Phase2Config, Phase2Renderer, Viewport,
};
use gpu_charts_shared::ChartConfiguration;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Bridge between DataManager and the rest of the system
#[derive(Clone)]
pub struct DataManagerBridge {
    /// The underlying data manager
    data_manager: Arc<DataManager>,

    /// Active handles
    active_handles: Arc<RwLock<HashMap<Uuid, BufferHandle>>>,

    /// Configuration
    config: Arc<RwLock<DataManagerConfig>>,

    /// Statistics
    stats: Arc<RwLock<DataManagerStats>>,
}

impl DataManagerBridge {
    /// Create a new data manager bridge
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: &GpuChartsConfig,
    ) -> Result<Self> {
        let dm_config = DataManagerConfig {
            max_memory_bytes: config.data.cache_size,
            cache_ttl_seconds: 3600,
            enable_prefetching: config.data.prefetch_enabled,
            enable_speculative_caching: config.data.prefetch_enabled,
            buffer_pool: None, // Will be set by renderer bridge
        };

        let data_manager = Arc::new(DataManager::new(device, queue, dm_config.clone()));

        Ok(Self {
            data_manager,
            active_handles: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(dm_config)),
            stats: Arc::new(RwLock::new(DataManagerStats::default())),
        })
    }

    /// Load data with handle tracking
    pub async fn load_data(&self, source: DataSource, metadata: BufferMetadata) -> Result<Uuid> {
        let handle = self
            .data_manager
            .load_data(source, metadata)
            .await
            .map_err(|e| IntegrationError::DataManager(e.to_string()))?;

        let id = Uuid::new_v4();
        self.active_handles.write().insert(id, handle);

        self.stats.write().data_loads += 1;

        Ok(id)
    }

    /// Get a handle by ID
    pub fn get_handle(&self, id: &Uuid) -> Option<BufferHandle> {
        self.active_handles.read().get(id).cloned()
    }

    /// Release a handle
    pub fn release_handle(&self, id: &Uuid) {
        self.active_handles.write().remove(id);
        self.stats.write().handles_released += 1;
    }

    /// Update configuration
    pub async fn update_config(&self, config: &GpuChartsConfig) -> Result<()> {
        let mut dm_config = self.config.write();
        dm_config.max_memory_bytes = config.data.cache_size;
        dm_config.enable_prefetching = config.data.prefetch_enabled;
        dm_config.enable_speculative_caching = config.data.prefetch_enabled;

        // TODO: Apply config to data manager

        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> DataManagerStats {
        self.stats.read().clone()
    }

    /// Get the underlying data manager
    pub fn data_manager(&self) -> &Arc<DataManager> {
        &self.data_manager
    }

    /// Prefetch data based on viewport
    pub async fn prefetch_viewport_data(&self, viewport: &Viewport, distance: f32) -> Result<()> {
        // Calculate prefetch range
        let time_range = viewport.x_max - viewport.x_min;
        let prefetch_start = viewport.x_min - time_range * distance;
        let prefetch_end = viewport.x_max + time_range * distance;

        // TODO: Implement actual prefetching logic
        log::debug!(
            "Prefetching data for range: {} - {}",
            prefetch_start,
            prefetch_end
        );

        self.stats.write().prefetch_requests += 1;

        Ok(())
    }
}

/// Bridge between Renderer and the rest of the system
#[derive(Clone)]
pub struct RendererBridge {
    /// The underlying Phase 2 renderer
    renderer: Arc<RwLock<Phase2Renderer>>,

    /// Render configuration
    config: Arc<RwLock<Phase2Config>>,

    /// Statistics
    stats: Arc<RwLock<RendererStats>>,

    /// Device and queue references
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
}

impl RendererBridge {
    /// Create a new renderer bridge
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: &GpuChartsConfig,
    ) -> Result<Self> {
        let phase2_config = Phase2Config {
            enable_multi_resolution: config.rendering.resolution_scale != 1.0,
            enable_indirect_draw: config.performance.indirect_drawing,
            enable_gpu_vertex_gen: config.performance.gpu_culling,
            enable_vertex_compression: config.performance.vertex_compression,
            enable_render_bundles: true,
            target_fps: config.rendering.target_fps as f32,
        };

        // Default surface format and size - will be updated when surface is created
        let renderer = Phase2Renderer::new(
            device.clone(),
            queue.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            (1920, 1080),
        )
        .map_err(|e| IntegrationError::Renderer(e.to_string()))?;

        Ok(Self {
            renderer: Arc::new(RwLock::new(renderer)),
            config: Arc::new(RwLock::new(phase2_config)),
            stats: Arc::new(RwLock::new(RendererStats::default())),
            device,
            queue,
        })
    }

    /// Render a frame
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        buffer_handles: &[BufferHandle],
        viewport: &Viewport,
        metrics: &RendererMetrics,
    ) -> Result<()> {
        // Convert handles to buffer sets
        let buffer_sets = buffer_handles
            .iter()
            .filter_map(|handle| handle.get_buffer_set())
            .collect::<Vec<_>>();

        if buffer_sets.is_empty() {
            return Err(IntegrationError::Renderer(
                "No valid buffer sets".to_string(),
            ));
        }

        // Render with Phase 2 optimizations
        self.renderer
            .write()
            .render_optimized(encoder, surface_view, &buffer_sets, viewport, metrics)
            .map_err(|e| IntegrationError::Renderer(e.to_string()))?;

        self.stats.write().frames_rendered += 1;

        Ok(())
    }

    /// Update configuration
    pub async fn update_config(&self, config: &GpuChartsConfig) -> Result<()> {
        let phase2_config = Phase2Config {
            enable_multi_resolution: config.rendering.resolution_scale != 1.0,
            enable_indirect_draw: config.performance.indirect_drawing,
            enable_gpu_vertex_gen: config.performance.gpu_culling,
            enable_vertex_compression: config.performance.vertex_compression,
            enable_render_bundles: true,
            target_fps: config.rendering.target_fps as f32,
        };

        self.renderer.write().update_config(phase2_config.clone());
        *self.config.write() = phase2_config;

        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> RendererStats {
        let renderer_stats = self.renderer.read().get_stats();
        let mut stats = self.stats.read().clone();

        // Merge renderer stats
        if let Some(frame_count) = renderer_stats.get("frame_count").and_then(|v| v.as_u64()) {
            stats.total_frames = frame_count;
        }

        stats
    }

    /// Get quality level
    pub fn get_quality_level(&self) -> String {
        self.renderer.read().get_quality_level()
    }

    /// Handle resize
    pub fn handle_resize(&self, new_size: (u32, u32)) -> Result<()> {
        // TODO: Recreate renderer with new size
        log::info!("Renderer resize to: {:?}", new_size);
        self.stats.write().resize_events += 1;
        Ok(())
    }
}

/// Data manager statistics
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DataManagerStats {
    pub data_loads: u64,
    pub handles_released: u64,
    pub prefetch_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub memory_used_mb: f64,
}

/// Renderer statistics
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RendererStats {
    pub frames_rendered: u64,
    pub total_frames: u64,
    pub resize_events: u64,
    pub quality_changes: u64,
    pub current_fps: f32,
    pub average_frame_time_ms: f32,
}

/// Memory pressure coordinator
pub struct MemoryPressureCoordinator {
    data_bridge: DataManagerBridge,
    renderer_bridge: RendererBridge,
    pressure_threshold: f32,
}

impl MemoryPressureCoordinator {
    pub fn new(data_bridge: DataManagerBridge, renderer_bridge: RendererBridge) -> Self {
        Self {
            data_bridge,
            renderer_bridge,
            pressure_threshold: 0.9, // 90% memory usage
        }
    }

    /// Handle memory pressure
    pub async fn handle_memory_pressure(&self, current_usage: f64, max_memory: f64) -> Result<()> {
        let usage_ratio = current_usage / max_memory;

        if usage_ratio > self.pressure_threshold as f64 {
            log::warn!(
                "Memory pressure detected: {:.1}% usage",
                usage_ratio * 100.0
            );

            // Reduce quality to free memory
            let mut config = self.renderer_bridge.config.write();
            config.enable_vertex_compression = true;
            config.enable_multi_resolution = true;

            // TODO: Trigger cache eviction in data manager

            Ok(())
        } else {
            Ok(())
        }
    }
}

/// Buffer sharing protocol between DataManager and Renderer
pub struct BufferSharingProtocol {
    /// Shared buffer registry
    shared_buffers: Arc<RwLock<HashMap<Uuid, SharedBufferInfo>>>,
}

#[derive(Clone)]
struct SharedBufferInfo {
    buffer: Arc<wgpu::Buffer>,
    usage: wgpu::BufferUsages,
    size: u64,
    last_access: std::time::Instant,
}

impl BufferSharingProtocol {
    pub fn new() -> Self {
        Self {
            shared_buffers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a shared buffer
    pub fn register_buffer(
        &self,
        id: Uuid,
        buffer: Arc<wgpu::Buffer>,
        usage: wgpu::BufferUsages,
        size: u64,
    ) {
        let info = SharedBufferInfo {
            buffer,
            usage,
            size,
            last_access: std::time::Instant::now(),
        };

        self.shared_buffers.write().insert(id, info);
    }

    /// Get a shared buffer
    pub fn get_buffer(&self, id: &Uuid) -> Option<Arc<wgpu::Buffer>> {
        let mut buffers = self.shared_buffers.write();
        if let Some(info) = buffers.get_mut(id) {
            info.last_access = std::time::Instant::now();
            Some(info.buffer.clone())
        } else {
            None
        }
    }

    /// Clean up old buffers
    pub fn cleanup_old_buffers(&self, max_age: std::time::Duration) {
        let now = std::time::Instant::now();
        let mut buffers = self.shared_buffers.write();

        buffers.retain(|_, info| now.duration_since(info.last_access) < max_age);
    }
}
