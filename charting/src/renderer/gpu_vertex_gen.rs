//! GPU Vertex Generation wrapper for charting library
//! 
//! This module wraps the GPU vertex generation system for WebGPU/WASM compatibility
//! and integration with the charting library's rendering pipeline.

use std::sync::Arc;
use wgpu::util::DeviceExt;
use crate::renderer::data_store::DataStore;

/// Configuration for GPU vertex generation
#[derive(Debug, Clone)]
pub struct ChartVertexGenConfig {
    /// Enable GPU vertex generation
    pub enabled: bool,
    /// Maximum vertices to generate per frame
    pub max_vertices: u32,
    /// Minimum pixel spacing between points
    pub min_pixel_spacing: f32,
    /// Enable LOD-based reduction
    pub enable_lod: bool,
    /// Workgroup size for compute shader
    pub workgroup_size: u32,
}

impl Default for ChartVertexGenConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_vertices: 1_000_000,
            min_pixel_spacing: 1.0,
            enable_lod: true,
            workgroup_size: 256,
        }
    }
}

/// GPU vertex generation parameters (for uniform buffer)
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
    output_vertex_count: u32,
    zoom_level: f32,
    _padding: [f32; 3],
}

/// GPU-generated vertex format
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuGeneratedVertex {
    position: [f32; 2],
    color: [f32; 4],
    _padding: [f32; 2],
}

/// Indirect draw arguments for GPU-driven rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DrawIndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

/// GPU vertex generation system for chart rendering
pub struct ChartGpuVertexGen {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: ChartVertexGenConfig,
    
    // Compute pipeline
    vertex_gen_pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    
    // Buffers
    params_buffer: wgpu::Buffer,
    vertex_output_buffer: wgpu::Buffer,
    indirect_draw_buffer: wgpu::Buffer,
    
    // Current state
    current_vertex_count: u32,
}

impl ChartGpuVertexGen {
    /// Create new GPU vertex generation system
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let config = ChartVertexGenConfig::default();
        
        // Create compute shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Chart GPU Vertex Gen Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/chart_vertex_gen.wgsl").into()),
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Vertex Gen Bind Group Layout"),
            entries: &[
                // Input data buffer
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
                // Output vertex buffer
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
                // Indirect draw buffer
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
                // Parameters uniform
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
        let vertex_gen_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Vertex Gen Compute Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        // Create buffers
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Gen Params"),
            size: std::mem::size_of::<VertexGenParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let vertex_output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Generated Vertices"),
            size: (config.max_vertices * std::mem::size_of::<GpuGeneratedVertex>() as u32) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        
        let indirect_draw_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Indirect Draw Args"),
            contents: bytemuck::cast_slice(&[DrawIndirectArgs {
                vertex_count: 0,
                instance_count: 1,
                first_vertex: 0,
                first_instance: 0,
            }]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
        });
        
        Self {
            device,
            queue,
            config,
            vertex_gen_pipeline,
            bind_group_layout,
            params_buffer,
            vertex_output_buffer,
            indirect_draw_buffer,
            current_vertex_count: 0,
        }
    }
    
    /// Generate vertices on GPU for current viewport
    pub fn generate_vertices(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        data_store: &DataStore,
        input_buffer: &wgpu::Buffer,
        total_points: u32,
    ) -> &wgpu::Buffer {
        if !self.config.enabled {
            return &self.vertex_output_buffer;
        }
        
        // Calculate LOD factor based on zoom level
        let viewport_range = (data_store.end_x - data_store.start_x) as f32;
        // Use the full data range as an approximation of total range
        let total_range = if !data_store.data_groups.is_empty() && data_store.data_groups[0].length > 0 {
            data_store.data_groups[0].length as f32
        } else {
            viewport_range
        };
        
        let zoom_level = total_range / viewport_range.max(1.0);
        let lod_factor = if self.config.enable_lod {
            (1.0 / zoom_level.sqrt()).max(0.1).min(1.0)
        } else {
            1.0
        };
        
        // Update parameters
        let params = VertexGenParams {
            viewport_start: data_store.start_x as f32,
            viewport_end: data_store.end_x as f32,
            screen_width: data_store.screen_size.width as f32,
            screen_height: data_store.screen_size.height as f32,
            total_points,
            lod_factor,
            min_pixel_spacing: self.config.min_pixel_spacing,
            output_vertex_count: self.config.max_vertices,
            zoom_level,
            _padding: [0.0; 3],
        };
        
        self.queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
        
        // Create bind group
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Vertex Gen Bind Group"),
            layout: &self.bind_group_layout,
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
            label: Some("GPU Vertex Generation"),
            timestamp_writes: None,
        });
        
        compute_pass.set_pipeline(&self.vertex_gen_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        
        let workgroups = (total_points + self.config.workgroup_size - 1) / self.config.workgroup_size;
        compute_pass.dispatch_workgroups(workgroups, 1, 1);
        
        log::info!("GPU vertex generation dispatched: {} workgroups for {} points", 
                   workgroups, total_points);
        
        &self.vertex_output_buffer
    }
    
    /// Get the indirect draw buffer for GPU-driven rendering
    pub fn get_indirect_buffer(&self) -> &wgpu::Buffer {
        &self.indirect_draw_buffer
    }
    
    /// Update configuration
    pub fn set_config(&mut self, config: ChartVertexGenConfig) {
        self.config = config;
    }
    
    /// Check if GPU vertex generation is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
    
    /// Get vertex buffer layout for pipeline creation
    pub fn get_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuGeneratedVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}