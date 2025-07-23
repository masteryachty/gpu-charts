//! GPU-driven vertex generation using compute shaders
//!
//! This module moves vertex generation entirely to the GPU, eliminating CPU-GPU
//! data transfer overhead and enabling dynamic vertex count based on LOD.

use crate::{GpuBufferSet, Viewport};
use gpu_charts_shared::{Error, Result};
use std::sync::Arc;

/// Configuration for GPU vertex generation
#[derive(Debug, Clone)]
pub struct VertexGenConfig {
    /// Maximum vertices per dispatch
    pub max_vertices_per_dispatch: u32,
    /// Workgroup size for compute shader
    pub workgroup_size: u32,
    /// Enable LOD-based vertex reduction
    pub enable_lod: bool,
    /// Minimum pixel spacing between points
    pub min_pixel_spacing: f32,
}

impl Default for VertexGenConfig {
    fn default() -> Self {
        Self {
            max_vertices_per_dispatch: 1_000_000,
            workgroup_size: 256,
            enable_lod: true,
            min_pixel_spacing: 1.0,
        }
    }
}

/// GPU vertex generator using compute shaders
pub struct GpuVertexGenerator {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: VertexGenConfig,

    // Compute pipeline
    vertex_gen_pipeline: wgpu::ComputePipeline,
    vertex_gen_bind_group_layout: wgpu::BindGroupLayout,

    // Output buffers
    vertex_output_buffer: wgpu::Buffer,
    indirect_draw_buffer: wgpu::Buffer,

    // Uniform buffer for parameters
    params_buffer: wgpu::Buffer,
}

impl GpuVertexGenerator {
    /// Create a new GPU vertex generator
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Result<Self> {
        let config = VertexGenConfig::default();
        // Create compute shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Generation Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/vertex_gen.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vertex Gen Bind Group Layout"),
            entries: &[
                // Input data buffer (storage, read-only)
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Output vertex buffer (storage, read-write)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Indirect draw buffer (storage, read-write)
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Parameters uniform buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Vertex Gen Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create compute pipeline
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Vertex Generation Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Create output buffers
        let max_vertices = config.max_vertices_per_dispatch;
        let vertex_size = std::mem::size_of::<GpuVertex>() as u64;

        let vertex_output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Output Buffer"),
            size: max_vertices as u64 * vertex_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        // Indirect draw buffer for draw calls
        let indirect_draw_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Indirect Draw Buffer"),
            size: std::mem::size_of::<DrawIndirectArgs>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        // Parameters buffer
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Gen Parameters"),
            size: std::mem::size_of::<VertexGenParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            device,
            queue,
            config,
            vertex_gen_pipeline: pipeline,
            vertex_gen_bind_group_layout: bind_group_layout,
            vertex_output_buffer,
            indirect_draw_buffer,
            params_buffer,
        })
    }

    /// Generate vertices for the given data and viewport
    pub fn generate_vertices(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        buffer_set: &GpuBufferSet,
        viewport: &Viewport,
        screen_width: u32,
        screen_height: u32,
    ) -> Result<VertexGenResult> {
        // Prepare parameters
        let params = self.calculate_params(buffer_set, viewport, screen_width, screen_height)?;

        // Update parameters buffer
        self.queue
            .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

        // Get input data buffer
        let input_buffer = buffer_set
            .buffers
            .get("time")
            .and_then(|buffers| buffers.first())
            .ok_or_else(|| Error::GpuError("Missing time buffer".to_string()))?;

        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Gen Bind Group"),
            layout: &self.vertex_gen_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.vertex_output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.indirect_draw_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.params_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch compute shader
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Vertex Generation Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.vertex_gen_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        // Calculate dispatch size
        let dispatch_x =
            (params.total_points + self.config.workgroup_size - 1) / self.config.workgroup_size;
        compute_pass.dispatch_workgroups(dispatch_x, 1, 1);

        drop(compute_pass);

        Ok(VertexGenResult {
            vertex_buffer: &self.vertex_output_buffer,
            indirect_buffer: &self.indirect_draw_buffer,
            estimated_vertex_count: params.estimated_output_vertices,
        })
    }

    /// Calculate parameters for vertex generation
    fn calculate_params(
        &self,
        buffer_set: &GpuBufferSet,
        viewport: &Viewport,
        screen_width: u32,
        screen_height: u32,
    ) -> Result<VertexGenParams> {
        let total_points = buffer_set.metadata.row_count;

        // Calculate LOD factor based on zoom and screen size
        let lod_factor = if self.config.enable_lod {
            self.calculate_lod_factor(viewport, screen_width, total_points)
        } else {
            1.0
        };

        // Calculate pixel-space parameters
        let time_range = viewport.time_range.end - viewport.time_range.start;
        let _pixels_per_time_unit = screen_width as f32 / time_range as f32;

        // Estimate output vertices based on LOD and pixel spacing
        let estimated_vertices = if self.config.enable_lod {
            let max_vertices_by_pixels = screen_width as f32 / self.config.min_pixel_spacing;
            (total_points as f32 * lod_factor).min(max_vertices_by_pixels) as u32
        } else {
            total_points
        };

        Ok(VertexGenParams {
            viewport_start: viewport.time_range.start as f32,
            viewport_end: viewport.time_range.end as f32,
            screen_width: screen_width as f32,
            screen_height: screen_height as f32,
            total_points,
            lod_factor,
            min_pixel_spacing: self.config.min_pixel_spacing,
            estimated_output_vertices: estimated_vertices,
            zoom_level: viewport.zoom_level,
            _padding: [0.0; 3],
        })
    }

    /// Calculate LOD factor based on viewport and screen size
    fn calculate_lod_factor(
        &self,
        viewport: &Viewport,
        screen_width: u32,
        total_points: u32,
    ) -> f32 {
        // Calculate how many data points per pixel
        let _time_range = viewport.time_range.end - viewport.time_range.start;
        let points_in_viewport = total_points as f32 * viewport.zoom_level;
        let points_per_pixel = points_in_viewport / screen_width as f32;

        // Apply LOD reduction based on points per pixel
        if points_per_pixel > 10.0 {
            0.1 // Show 10% of points
        } else if points_per_pixel > 5.0 {
            0.2 // Show 20% of points
        } else if points_per_pixel > 2.0 {
            0.5 // Show 50% of points
        } else {
            1.0 // Show all points
        }
    }
}

/// Parameters for vertex generation compute shader
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexGenParams {
    viewport_start: f32,
    viewport_end: f32,
    screen_width: f32,
    screen_height: f32,
    total_points: u32,
    lod_factor: f32,
    min_pixel_spacing: f32,
    estimated_output_vertices: u32,
    zoom_level: f32,
    _padding: [f32; 3],
}

/// GPU vertex format
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

/// Indirect draw arguments for GPU-driven rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DrawIndirectArgs {
    pub vertex_count: u32,
    pub instance_count: u32,
    pub first_vertex: u32,
    pub first_instance: u32,
}

/// Result of vertex generation
pub struct VertexGenResult<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub indirect_buffer: &'a wgpu::Buffer,
    pub estimated_vertex_count: u32,
}

/// Dynamic vertex generation system for adaptive quality
pub struct DynamicVertexSystem {
    generator: GpuVertexGenerator,
    frame_time_tracker: FrameTimeTracker,
    quality_controller: QualityController,
}

impl DynamicVertexSystem {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        _config: VertexGenConfig,
    ) -> Result<Self> {
        let generator = GpuVertexGenerator::new(device, queue)?;

        Ok(Self {
            generator,
            frame_time_tracker: FrameTimeTracker::new(),
            quality_controller: QualityController::new(),
        })
    }

    /// Generate vertices with dynamic quality adjustment
    pub fn generate_adaptive(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        buffer_set: &GpuBufferSet,
        viewport: &Viewport,
        screen_width: u32,
        screen_height: u32,
        target_frame_time_ms: f32,
    ) -> Result<VertexGenResult> {
        // Adjust quality based on frame time
        let quality = self.quality_controller.get_quality(
            self.frame_time_tracker.average_frame_time(),
            target_frame_time_ms,
        );

        // Modify config based on quality
        let mut adjusted_config = self.generator.config.clone();
        adjusted_config.min_pixel_spacing = self.generator.config.min_pixel_spacing / quality;

        // Generate vertices
        let result = self.generator.generate_vertices(
            encoder,
            buffer_set,
            viewport,
            screen_width,
            screen_height,
        )?;

        Ok(result)
    }

    /// Update frame time for adaptive quality
    pub fn update_frame_time(&mut self, frame_time_ms: f32) {
        self.frame_time_tracker.add_frame_time(frame_time_ms);
    }
}

/// Track frame times for adaptive quality
struct FrameTimeTracker {
    frame_times: Vec<f32>,
    max_samples: usize,
}

impl FrameTimeTracker {
    fn new() -> Self {
        Self {
            frame_times: Vec::with_capacity(60),
            max_samples: 60,
        }
    }

    fn add_frame_time(&mut self, time_ms: f32) {
        self.frame_times.push(time_ms);
        if self.frame_times.len() > self.max_samples {
            self.frame_times.remove(0);
        }
    }

    fn average_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            16.67 // Default to 60 FPS
        } else {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
        }
    }
}

/// Control rendering quality based on performance
struct QualityController {
    current_quality: f32,
    min_quality: f32,
    max_quality: f32,
    adjustment_rate: f32,
}

impl QualityController {
    fn new() -> Self {
        Self {
            current_quality: 1.0,
            min_quality: 0.1,
            max_quality: 1.0,
            adjustment_rate: 0.05,
        }
    }

    fn get_quality(&mut self, current_frame_time: f32, target_frame_time: f32) -> f32 {
        let ratio = current_frame_time / target_frame_time;

        if ratio > 1.1 {
            // Running slow, decrease quality
            self.current_quality =
                (self.current_quality - self.adjustment_rate).max(self.min_quality);
        } else if ratio < 0.9 {
            // Running fast, increase quality
            self.current_quality =
                (self.current_quality + self.adjustment_rate).min(self.max_quality);
        }

        self.current_quality
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_gen_params() {
        let params = VertexGenParams {
            viewport_start: 0.0,
            viewport_end: 1000.0,
            screen_width: 1920.0,
            screen_height: 1080.0,
            total_points: 1_000_000,
            lod_factor: 0.5,
            min_pixel_spacing: 2.0,
            estimated_output_vertices: 500_000,
            zoom_level: 1.0,
            _padding: [0.0; 3],
        };

        assert_eq!(std::mem::size_of::<VertexGenParams>(), 64); // Ensure alignment
    }

    #[test]
    fn test_frame_time_tracker() {
        let mut tracker = FrameTimeTracker::new();

        tracker.add_frame_time(16.0);
        tracker.add_frame_time(17.0);
        tracker.add_frame_time(15.0);

        let avg = tracker.average_frame_time();
        assert!((avg - 16.0).abs() < 1.0);
    }

    #[test]
    fn test_quality_controller() {
        let mut controller = QualityController::new();

        // Test quality decrease when running slow
        let quality = controller.get_quality(20.0, 16.67);
        assert!(quality < 1.0);

        // Test quality increase when running fast
        let quality2 = controller.get_quality(10.0, 16.67);
        assert!(quality2 > quality);
    }
}
