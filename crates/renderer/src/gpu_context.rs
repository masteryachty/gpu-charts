//! Persistent GPU context for eliminating initialization overhead
//!
//! This module provides a persistent GPU context that can be reused across frames,
//! eliminating the ~100ms GPU initialization overhead observed in benchmarks.

use crate::timing::Timer;
use gpu_charts_shared::{Error, Result};
use std::sync::Arc;

/// Persistent GPU context that maintains device/queue across frames
pub struct PersistentGpuContext {
    /// WebGPU instance (singleton)
    pub instance: wgpu::Instance,
    /// Primary adapter
    pub adapter: Arc<wgpu::Adapter>,
    /// Primary device
    pub device: Arc<wgpu::Device>,
    /// Primary queue
    pub queue: Arc<wgpu::Queue>,
    /// Device features
    pub features: wgpu::Features,
    /// Device limits
    pub limits: wgpu::Limits,
    /// Creation timestamp for performance tracking
    pub creation_time: Timer,
}

impl PersistentGpuContext {
    /// Create a new persistent GPU context
    /// This should be called once at application startup and the result stored
    pub async fn new() -> Result<Arc<Self>> {
        let start_time = Timer::now();

        // Create instance with appropriate backends
        let backends = if cfg!(target_arch = "wasm32") {
            wgpu::Backends::BROWSER_WEBGPU
        } else {
            wgpu::Backends::all()
        };

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        // Request adapter with high performance preference
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| {
                Error::GpuError(format!("Failed to find suitable GPU adapter: {:?}", e))
            })?;

        let adapter = Arc::new(adapter);

        // Request features for optimal performance
        let mut features = wgpu::Features::empty();

        // Enable timestamp queries if available (for GPU timing)
        if adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            features |= wgpu::Features::TIMESTAMP_QUERY;
        }

        // Enable timestamp queries inside passes if available
        if adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES)
        {
            features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES;
        }

        // Enable pipeline statistics if available
        if adapter
            .features()
            .contains(wgpu::Features::PIPELINE_STATISTICS_QUERY)
        {
            features |= wgpu::Features::PIPELINE_STATISTICS_QUERY;
        }

        // Use browser-compatible limits for WebGPU
        // downlevel_webgl2_defaults() provides limits that work in all browsers
        let limits = wgpu::Limits::downlevel_webgl2_defaults();
        log::info!("Using browser-compatible WebGPU limits (downlevel_webgl2_defaults)");

        // Log what we're about to request
        log::info!("Requesting device with limits: {:?}", limits);
        log::info!("Requesting device with features: {:?}", features);

        // Create device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("GPU Charts Persistent Device"),
                required_features: features,
                required_limits: limits.clone(),
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|e| {
                log::error!("Device request failed with error: {:?}", e);
                Error::GpuError(format!("Failed to create GPU device: {:?}", e))
            })?;

        log::info!("Device created successfully!");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        log::info!(
            "GPU context initialized in {:.2}ms with features: {:?}",
            start_time.elapsed_millis(),
            features
        );

        Ok(Arc::new(Self {
            instance,
            adapter,
            device,
            queue,
            features,
            limits,
            creation_time: start_time,
        }))
    }

    /// Create a new surface for rendering
    /// Surfaces are cheap to create and don't need to be persisted
    pub fn create_surface(
        &self,
        window: impl Into<wgpu::SurfaceTarget<'static>>,
    ) -> Result<wgpu::Surface<'static>> {
        self.instance
            .create_surface(window)
            .map_err(|e| Error::GpuError(format!("Failed to create surface: {:?}", e)))
    }

    /// Check if GPU timing is supported
    pub fn supports_gpu_timing(&self) -> bool {
        self.features.contains(wgpu::Features::TIMESTAMP_QUERY)
    }

    /// Get uptime of the GPU context in seconds
    pub fn uptime_secs(&self) -> f64 {
        self.creation_time.elapsed_secs()
    }

    /// Get device statistics
    pub fn get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "adapter_info": {
                "name": self.adapter.get_info().name,
                "vendor": self.adapter.get_info().vendor,
                "device": self.adapter.get_info().device,
                "device_type": format!("{:?}", self.adapter.get_info().device_type),
                "backend": format!("{:?}", self.adapter.get_info().backend),
            },
            "features": {
                "timestamp_query": self.features.contains(wgpu::Features::TIMESTAMP_QUERY),
                "timestamp_query_inside_passes": self.features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES),
                "pipeline_statistics": self.features.contains(wgpu::Features::PIPELINE_STATISTICS_QUERY),
            },
            "limits": {
                "max_texture_dimension_2d": self.limits.max_texture_dimension_2d,
                "max_buffer_size": self.limits.max_buffer_size,
                "max_vertex_buffers": self.limits.max_vertex_buffers,
                "max_compute_workgroup_size_x": self.limits.max_compute_workgroup_size_x,
            },
            "uptime_seconds": self.uptime_secs() as u64,
        })
    }
}

/// Helper to create a renderer with persistent GPU context
pub async fn create_renderer_with_persistent_context(
    context: Arc<PersistentGpuContext>,
    window: impl Into<wgpu::SurfaceTarget<'static>>,
    width: u32,
    height: u32,
) -> Result<crate::Renderer> {
    // Create surface for this window
    let surface = context.create_surface(window)?;

    // Create renderer with persistent device/queue
    crate::Renderer::new_with_device(
        Arc::clone(&context.device),
        Arc::clone(&context.queue),
        surface,
        width,
        height,
    )
}

/// Global context holder for applications that want a singleton
/// This is optional - applications can manage their own instance
pub struct GlobalGpuContext {
    context: Arc<PersistentGpuContext>,
}

impl GlobalGpuContext {
    /// Initialize the global context
    pub async fn initialize() -> Result<Self> {
        let context = PersistentGpuContext::new().await?;
        Ok(Self { context })
    }

    /// Get the context
    pub fn get(&self) -> Arc<PersistentGpuContext> {
        Arc::clone(&self.context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_persistent_context_creation() {
        // Create context
        let ctx = PersistentGpuContext::new().await.unwrap();

        // Should have valid device and queue
        assert!(ctx.uptime().as_millis() < 1000);
    }

    #[tokio::test]
    async fn test_global_context() {
        // Initialize global context
        let global = GlobalGpuContext::initialize().await.unwrap();
        let ctx1 = global.get();

        // Multiple gets should return same instance
        let ctx2 = global.get();
        assert!(Arc::ptr_eq(&ctx1, &ctx2));
    }
}
