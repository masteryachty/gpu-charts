//! Core rendering engine that manages WebGPU resources

use crate::{gpu_timing::GpuTimingSystem, GpuBufferSet, PerformanceMetrics, Viewport};
use gpu_charts_shared::{Error, Result, VisualConfig};
use std::sync::Arc;

pub struct RenderEngine {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    surface_texture: Option<wgpu::SurfaceTexture>,
    // Performance tracking
    frame_count: u64,
    total_frame_time: f64,
    // GPU timing
    gpu_timing: Option<GpuTimingSystem>,
}

impl RenderEngine {
    /// Create a new render engine with shared device/queue
    pub fn new_with_device(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let config = surface
            .get_default_config(&device.adapter(), width, height)
            .ok_or_else(|| {
                Error::GpuError("No suitable surface configuration found".to_string())
            })?;

        surface.configure(&device, &config);

        // Create GPU timing system if supported
        let gpu_timing = if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            Some(GpuTimingSystem::new(
                Arc::clone(&device),
                Arc::clone(&queue),
            ))
        } else {
            None
        };

        Ok(Self {
            device,
            queue,
            surface,
            config,
            surface_texture: None,
            frame_count: 0,
            total_frame_time: 0.0,
            gpu_timing,
        })
    }

    /// Get the device for creating resources
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get the queue for submitting commands
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Render a frame
    pub fn render(
        &mut self,
        chart_renderer: &mut dyn crate::chart_renderers::ChartRenderer,
        overlay_renderers: &mut [Box<dyn crate::overlays::OverlayRenderer>],
        buffer_sets: &[Arc<GpuBufferSet>],
        viewport: &Viewport,
        visual_config: &VisualConfig,
        metrics: &mut PerformanceMetrics,
    ) -> Result<()> {
        let frame_start = std::time::Instant::now();

        // Get the next frame
        let surface_texture = self
            .surface
            .get_current_texture()
            .map_err(|e| Error::GpuError(format!("Failed to get surface texture: {:?}", e)))?;

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Start GPU timing if available
        if let Some(timing) = &self.gpu_timing {
            timing.begin_timing(&mut encoder, "total_frame", 0);
            timing.begin_timing(&mut encoder, "render_pass", 2);
        }

        // Main render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: visual_config.background_color[0] as f64,
                            g: visual_config.background_color[1] as f64,
                            b: visual_config.background_color[2] as f64,
                            a: visual_config.background_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Create render context
            let context = crate::RenderContext {
                device: &self.device,
                queue: &self.queue,
                viewport: *viewport,
                visual_config,
                frame_time: frame_start.elapsed().as_secs_f32(),
            };

            // Render main chart
            chart_renderer.render(&mut render_pass, buffer_sets, &context);

            // Render overlays
            for overlay in &mut *overlay_renderers {
                overlay.as_mut().render(&mut render_pass, &context);
            }
        }

        // End render pass timing
        if let Some(timing) = &self.gpu_timing {
            timing.end_timing(&mut encoder, "render_pass", 3);
        }

        // Collect draw calls after render pass is dropped
        metrics.draw_calls += chart_renderer.get_draw_call_count();
        for overlay in overlay_renderers {
            metrics.draw_calls += overlay.get_draw_call_count();
        }

        // Resolve GPU timing queries if available
        if let Some(timing) = &self.gpu_timing {
            timing.resolve_queries(&mut encoder);
            timing.end_timing(&mut encoder, "total_frame", 1);
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Present the frame
        surface_texture.present();

        // Update metrics
        let frame_time = frame_start.elapsed();
        self.frame_count += 1;
        self.total_frame_time += frame_time.as_secs_f64() * 1000.0;
        metrics.frame_time_ms = frame_time.as_secs_f32() * 1000.0;

        // Read GPU timing results if available
        if let Some(timing) = &mut self.gpu_timing {
            // This is async but we'll do it on next frame to avoid blocking
            if let Some(gpu_time) = timing.get_timing("total_frame") {
                metrics.gpu_time_ms = gpu_time;
            }
        }

        Ok(())
    }

    /// Handle resize events
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Get performance statistics
    pub fn get_stats(&self) -> serde_json::Value {
        let avg_frame_time = if self.frame_count > 0 {
            self.total_frame_time / self.frame_count as f64
        } else {
            0.0
        };

        let mut stats = serde_json::json!({
            "frame_count": self.frame_count,
            "avg_frame_time_ms": avg_frame_time,
            "fps": if avg_frame_time > 0.0 { 1000.0 / avg_frame_time } else { 0.0 },
            "backend": format!("{:?}", self.device.backend()),
        });

        // Add GPU timing stats if available
        if let Some(timing) = &self.gpu_timing {
            stats["gpu_timing"] = timing.get_stats();
        }

        stats
    }
}

// Helper extension trait to get adapter info
trait DeviceExt {
    fn adapter(&self) -> Arc<wgpu::Adapter>;
    fn backend(&self) -> wgpu::Backend;
}

impl DeviceExt for wgpu::Device {
    fn adapter(&self) -> Arc<wgpu::Adapter> {
        // In a real implementation, we'd store the adapter
        // For now, return a dummy
        unimplemented!("Adapter storage not implemented")
    }

    fn backend(&self) -> wgpu::Backend {
        // Would be determined from adapter
        wgpu::Backend::BrowserWebGpu
    }
}
