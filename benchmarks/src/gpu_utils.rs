//! GPU utilities for benchmarking
//! 
//! This module provides GPU-specific utilities that complement the main
//! BenchmarkGpu struct in lib.rs

use wgpu::util::DeviceExt;

/// GPU memory allocation patterns for benchmarking
pub struct GpuMemoryBenchmark {
    pub device: std::sync::Arc<wgpu::Device>,
}

impl GpuMemoryBenchmark {
    pub fn new(device: std::sync::Arc<wgpu::Device>) -> Self {
        Self { device }
    }
    
    /// Benchmark buffer allocation performance
    pub fn benchmark_allocation(&self, size: u64, count: usize) -> std::time::Duration {
        let start = std::time::Instant::now();
        
        let mut buffers = Vec::with_capacity(count);
        for i in 0..count {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Benchmark Buffer {}", i)),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            buffers.push(buffer);
        }
        
        // Force GPU to process all allocations
        self.device.poll(wgpu::Maintain::Wait);
        
        let duration = start.elapsed();
        
        // Cleanup
        drop(buffers);
        
        duration
    }
    
    /// Benchmark buffer upload performance
    pub fn benchmark_upload(&self, queue: &wgpu::Queue, data: &[u8]) -> std::time::Duration {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Upload Benchmark Buffer"),
            size: data.len() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        
        let start = std::time::Instant::now();
        
        queue.write_buffer(&buffer, 0, data);
        queue.submit(None);
        self.device.poll(wgpu::Maintain::Wait);
        
        start.elapsed()
    }
}

/// GPU compute benchmarking utilities
pub struct GpuComputeBenchmark {
    pub device: std::sync::Arc<wgpu::Device>,
    pub queue: std::sync::Arc<wgpu::Queue>,
}

impl GpuComputeBenchmark {
    pub fn new(device: std::sync::Arc<wgpu::Device>, queue: std::sync::Arc<wgpu::Queue>) -> Self {
        Self { device, queue }
    }
    
    /// Create a simple compute pipeline for benchmarking
    pub fn create_compute_pipeline(&self, shader_source: &str) -> wgpu::ComputePipeline {
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Benchmark Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Benchmark Compute Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
            compilation_options: Default::default(),
        })
    }
    
    /// Benchmark compute dispatch performance
    pub fn benchmark_dispatch(
        &self,
        pipeline: &wgpu::ComputePipeline,
        workgroups: (u32, u32, u32),
    ) -> std::time::Duration {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Benchmark Compute Encoder"),
        });
        
        let start = std::time::Instant::now();
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Benchmark Compute Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(pipeline);
            compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }
        
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);
        
        start.elapsed()
    }
}

/// GPU render benchmarking utilities
pub struct GpuRenderBenchmark {
    pub device: std::sync::Arc<wgpu::Device>,
    pub queue: std::sync::Arc<wgpu::Queue>,
}

impl GpuRenderBenchmark {
    pub fn new(device: std::sync::Arc<wgpu::Device>, queue: std::sync::Arc<wgpu::Queue>) -> Self {
        Self { device, queue }
    }
    
    /// Create a test render target
    pub fn create_render_target(&self, width: u32, height: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Benchmark Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    }
    
    /// Benchmark render pass performance
    pub fn benchmark_render_pass(
        &self,
        target: &wgpu::TextureView,
        vertex_count: u32,
        instance_count: u32,
    ) -> std::time::Duration {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Benchmark Render Encoder"),
        });
        
        let start = std::time::Instant::now();
        
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Benchmark Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            // In a real benchmark, we'd set pipeline and draw here
            // render_pass.draw(0..vertex_count, 0..instance_count);
        }
        
        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::Maintain::Wait);
        
        start.elapsed()
    }
}

/// Simple test shaders for benchmarking
pub mod test_shaders {
    /// Simple compute shader that adds two buffers
    pub const COMPUTE_ADD: &str = r#"
        @group(0) @binding(0) var<storage, read> input_a: array<f32>;
        @group(0) @binding(1) var<storage, read> input_b: array<f32>;
        @group(0) @binding(2) var<storage, read_write> output: array<f32>;
        
        @compute @workgroup_size(256)
        fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
            let index = global_id.x;
            if (index < arrayLength(&input_a)) {
                output[index] = input_a[index] + input_b[index];
            }
        }
    "#;
    
    /// Simple vertex shader for line rendering
    pub const VERTEX_LINE: &str = r#"
        struct VertexOutput {
            @builtin(position) position: vec4<f32>,
            @location(0) color: vec4<f32>,
        }
        
        @vertex
        fn main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
            var out: VertexOutput;
            let x = f32(vertex_index) * 0.01 - 1.0;
            let y = sin(x * 10.0) * 0.5;
            out.position = vec4<f32>(x, y, 0.0, 1.0);
            out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
            return out;
        }
    "#;
    
    /// Simple fragment shader
    pub const FRAGMENT_SOLID: &str = r#"
        @fragment
        fn main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
            return color;
        }
    "#;
}