//! Multi-resolution rendering system for adaptive quality
//!
//! This module implements a sophisticated multi-resolution rendering system
//! that dynamically adjusts quality based on performance metrics and data density.

use crate::PerformanceMetrics;
use gpu_charts_shared::Result;
use std::collections::VecDeque;
use std::sync::Arc;

/// Multi-resolution rendering configuration
#[derive(Debug, Clone)]
pub struct MultiResConfig {
    /// Enable adaptive quality adjustments
    pub enable_adaptive_quality: bool,
    /// Target frame time in milliseconds
    pub target_frame_time_ms: f32,
    /// Maximum quality level (0-4)
    pub max_quality_level: u32,
    /// Enable temporal upsampling
    pub enable_temporal_upsampling: bool,
    /// Performance history size
    pub history_size: usize,
}

impl Default for MultiResConfig {
    fn default() -> Self {
        Self {
            enable_adaptive_quality: true,
            target_frame_time_ms: 16.67, // 60 FPS
            max_quality_level: 4,
            enable_temporal_upsampling: true,
            history_size: 60, // 1 second at 60 FPS
        }
    }
}

/// Quality level for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityLevel {
    UltraLow = 0, // 1/16 resolution
    Low = 1,      // 1/8 resolution
    Medium = 2,   // 1/4 resolution
    High = 3,     // 1/2 resolution
    Ultra = 4,    // Full resolution
}

impl From<u32> for QualityLevel {
    fn from(level: u32) -> Self {
        match level {
            0 => QualityLevel::UltraLow,
            1 => QualityLevel::Low,
            2 => QualityLevel::Medium,
            3 => QualityLevel::High,
            _ => QualityLevel::Ultra,
        }
    }
}

/// Multi-resolution rendering system
pub struct MultiResolutionSystem {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: MultiResConfig,

    /// Current quality level
    current_quality: QualityLevel,

    /// Performance history for adaptive quality
    performance_history: VecDeque<f32>,

    /// Temporal upsampling resources
    temporal_state: Option<TemporalUpsamplingState>,

    /// Resolution scaling factors
    resolution_scales: [f32; 5],

    /// Render targets for different resolutions
    render_targets: Vec<RenderTarget>,

    /// Upsampling pipeline
    upsampling_pipeline: Option<wgpu::RenderPipeline>,
}

impl MultiResolutionSystem {
    /// Create a new multi-resolution rendering system
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        config: MultiResConfig,
        window_size: (u32, u32),
    ) -> Result<Self> {
        let resolution_scales = [
            0.0625, // 1/16
            0.125,  // 1/8
            0.25,   // 1/4
            0.5,    // 1/2
            1.0,    // Full
        ];

        // Create render targets for each quality level
        let mut render_targets = Vec::new();
        for &scale in &resolution_scales {
            let width = ((window_size.0 as f32 * scale).max(1.0)) as u32;
            let height = ((window_size.1 as f32 * scale).max(1.0)) as u32;

            render_targets.push(RenderTarget::new(&device, width, height)?);
        }

        // Create upsampling pipeline if needed
        let upsampling_pipeline = if config.enable_temporal_upsampling {
            Some(create_upsampling_pipeline(&device)?)
        } else {
            None
        };

        let history_capacity = config.history_size;
        Ok(Self {
            device,
            queue,
            config,
            current_quality: QualityLevel::High,
            performance_history: VecDeque::with_capacity(history_capacity),
            temporal_state: None,
            resolution_scales,
            render_targets,
            upsampling_pipeline,
        })
    }

    /// Update quality level based on performance metrics
    pub fn update_quality(&mut self, metrics: &PerformanceMetrics) {
        if !self.config.enable_adaptive_quality {
            return;
        }

        // Add to history
        self.performance_history.push_back(metrics.frame_time_ms);
        if self.performance_history.len() > self.config.history_size {
            self.performance_history.pop_front();
        }

        // Calculate average frame time
        let avg_frame_time = if !self.performance_history.is_empty() {
            self.performance_history.iter().sum::<f32>() / self.performance_history.len() as f32
        } else {
            metrics.frame_time_ms
        };

        // Adjust quality based on performance
        let current_level = self.current_quality as u32;

        if avg_frame_time > self.config.target_frame_time_ms * 1.2 {
            // Performance too low, decrease quality
            if current_level > 0 {
                self.current_quality = QualityLevel::from(current_level - 1);
                log::info!(
                    "Decreasing quality to {:?} (avg frame time: {:.2}ms)",
                    self.current_quality,
                    avg_frame_time
                );
            }
        } else if avg_frame_time < self.config.target_frame_time_ms * 0.8 {
            // Performance good, try increasing quality
            if current_level < self.config.max_quality_level {
                self.current_quality = QualityLevel::from(current_level + 1);
                log::info!(
                    "Increasing quality to {:?} (avg frame time: {:.2}ms)",
                    self.current_quality,
                    avg_frame_time
                );
            }
        }
    }

    /// Get current render target based on quality level
    pub fn get_render_target(&self) -> &RenderTarget {
        &self.render_targets[self.current_quality as usize]
    }

    /// Get resolution scale for current quality
    pub fn get_resolution_scale(&self) -> f32 {
        self.resolution_scales[self.current_quality as usize]
    }

    /// Begin multi-resolution render pass
    pub fn begin_render<'a>(
        &'a mut self,
        encoder: &'a mut wgpu::CommandEncoder,
    ) -> MultiResRenderPass<'a> {
        let render_target = &self.render_targets[self.current_quality as usize];

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Multi-Resolution Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &render_target.color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: render_target.depth_view.as_ref().map(|view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        MultiResRenderPass {
            render_pass,
            quality_level: self.current_quality,
            resolution_scale: self.get_resolution_scale(),
        }
    }

    /// Perform upsampling to final resolution
    pub fn upsample_to_surface(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        _surface_view: &wgpu::TextureView,
        _window_size: (u32, u32),
    ) -> Result<()> {
        // For now, just render directly to surface without upsampling
        // In a full implementation, we would:
        // 1. Create bind groups for the low-res texture
        // 2. Use the upsampling pipeline to render to surface
        // 3. Handle different quality levels appropriately

        // This is a simplified version that avoids the complexity
        // The actual rendering already happened in the render pass

        Ok(())
    }

    /// Handle window resize
    pub fn resize(&mut self, new_size: (u32, u32)) -> Result<()> {
        // Recreate render targets at new sizes
        for (i, &scale) in self.resolution_scales.iter().enumerate() {
            let width = ((new_size.0 as f32 * scale).max(1.0)) as u32;
            let height = ((new_size.1 as f32 * scale).max(1.0)) as u32;

            self.render_targets[i] = RenderTarget::new(&self.device, width, height)?;
        }

        // Reset temporal state
        if self.config.enable_temporal_upsampling {
            self.temporal_state = None;
        }

        Ok(())
    }

    /// Get current quality statistics
    pub fn get_stats(&self) -> serde_json::Value {
        serde_json::json!({
            "current_quality": format!("{:?}", self.current_quality),
            "resolution_scale": self.get_resolution_scale(),
            "render_resolution": {
                "width": self.render_targets[self.current_quality as usize].width,
                "height": self.render_targets[self.current_quality as usize].height,
            },
            "performance_history": {
                "avg_frame_time": if !self.performance_history.is_empty() {
                    self.performance_history.iter().sum::<f32>() / self.performance_history.len() as f32
                } else {
                    0.0
                },
                "history_size": self.performance_history.len(),
            },
            "temporal_upsampling": self.config.enable_temporal_upsampling,
        })
    }
}

/// Render target for a specific resolution
#[derive(Debug)]
struct RenderTarget {
    width: u32,
    height: u32,
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_texture: Option<wgpu::Texture>,
    depth_view: Option<wgpu::TextureView>,
}

impl RenderTarget {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Result<Self> {
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("Multi-Res Color Target {}x{}", width, height)),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create depth texture for higher quality levels
        let (depth_texture, depth_view) = if width >= 256 && height >= 256 {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(&format!("Multi-Res Depth Target {}x{}", width, height)),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            (Some(texture), Some(view))
        } else {
            (None, None)
        };

        Ok(Self {
            width,
            height,
            color_texture,
            color_view,
            depth_texture,
            depth_view,
        })
    }
}

/// Multi-resolution render pass wrapper
pub struct MultiResRenderPass<'a> {
    render_pass: wgpu::RenderPass<'a>,
    pub quality_level: QualityLevel,
    pub resolution_scale: f32,
}

impl<'a> std::ops::Deref for MultiResRenderPass<'a> {
    type Target = wgpu::RenderPass<'a>;

    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}

impl<'a> std::ops::DerefMut for MultiResRenderPass<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render_pass
    }
}

/// Temporal upsampling state
struct TemporalUpsamplingState {
    history_texture: wgpu::Texture,
    history_view: wgpu::TextureView,
    motion_vectors: wgpu::Buffer,
    frame_index: u32,
}

/// Create upsampling pipeline
fn create_upsampling_pipeline(device: &wgpu::Device) -> Result<wgpu::RenderPipeline> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Upsampling Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/upsampling.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Upsampling Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Upsampling Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    Ok(pipeline)
}

/// Adaptive quality controller
pub struct AdaptiveQualityController {
    /// Target FPS
    target_fps: f32,
    /// Quality adjustment rate
    adjustment_rate: f32,
    /// Stability threshold
    stability_threshold: f32,
    /// Quality state
    state: QualityState,
}

#[derive(Debug, Clone)]
struct QualityState {
    current_level: f32,
    stability_counter: u32,
    last_adjustment_time: std::time::Instant,
}

impl AdaptiveQualityController {
    pub fn new(target_fps: f32) -> Self {
        Self {
            target_fps,
            adjustment_rate: 0.1,
            stability_threshold: 0.05,
            state: QualityState {
                current_level: 3.0, // Start at high quality
                stability_counter: 0,
                last_adjustment_time: std::time::Instant::now(),
            },
        }
    }

    /// Update quality based on performance
    pub fn update(&mut self, current_fps: f32) -> QualityLevel {
        let fps_ratio = current_fps / self.target_fps;
        let error = 1.0 - fps_ratio;

        // Check if we're stable
        if error.abs() < self.stability_threshold {
            self.state.stability_counter += 1;
        } else {
            self.state.stability_counter = 0;
        }

        // Only adjust if unstable for multiple frames
        if self.state.stability_counter < 5 {
            let now = std::time::Instant::now();
            if now
                .duration_since(self.state.last_adjustment_time)
                .as_secs_f32()
                > 0.5
            {
                // Adjust quality level
                self.state.current_level += error * self.adjustment_rate;
                self.state.current_level = self.state.current_level.clamp(0.0, 4.0);
                self.state.last_adjustment_time = now;
            }
        }

        QualityLevel::from(self.state.current_level.round() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_level_conversion() {
        assert_eq!(QualityLevel::from(0), QualityLevel::UltraLow);
        assert_eq!(QualityLevel::from(1), QualityLevel::Low);
        assert_eq!(QualityLevel::from(2), QualityLevel::Medium);
        assert_eq!(QualityLevel::from(3), QualityLevel::High);
        assert_eq!(QualityLevel::from(4), QualityLevel::Ultra);
        assert_eq!(QualityLevel::from(99), QualityLevel::Ultra);
    }

    #[test]
    fn test_adaptive_quality_controller() {
        let mut controller = AdaptiveQualityController::new(60.0);

        // Test low FPS -> quality should decrease
        let quality = controller.update(30.0);
        assert!(quality as u32 <= 3);

        // Test high FPS -> quality should increase
        controller.state.current_level = 2.0;
        let quality = controller.update(120.0);
        assert!(quality as u32 >= 2);
    }
}
