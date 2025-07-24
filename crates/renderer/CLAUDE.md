# Renderer Crate - CLAUDE.md

This file provides guidance for working with the renderer crate, which implements the pure GPU rendering engine for the GPU Charts system.

## Overview

The renderer crate provides:
- WebGPU-based rendering engine
- Multiple chart type renderers (line, candlestick, bar, area)
- Axis rendering with dynamic labels
- Grid and background rendering
- Shader management and compilation
- Render pipeline optimization
- Surface and texture management

## Architecture Position

```
shared-types
    ↑
├── config-system
│   ↑
└── renderer (this crate)
    ↑
└── wasm-bridge
```

This crate handles all GPU rendering operations and is used by wasm-bridge.

## Key Components

### Renderer (`src/lib.rs`)
Main rendering orchestrator:

```rust
pub struct Renderer {
    pub render_engine: RenderEngine,
    config: GpuChartsConfig,
    
    // Specialized renderers
    plot_renderer: PlotRenderer,
    candlestick_renderer: CandlestickRenderer,
    x_axis_renderer: XAxisRenderer,
    y_axis_renderer: YAxisRenderer,
}

impl Renderer {
    pub async fn render(&mut self, data_store: &DataStore) -> Result<(), RendererError>;
    pub fn resize(&mut self, width: u32, height: u32);
    pub fn set_chart_type(&mut self, chart_type: ChartType);
}
```

### RenderEngine (`src/render_engine.rs`)
Core WebGPU resource management:

```rust
pub struct RenderEngine {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub depth_texture: wgpu::TextureView,
}
```

### Specialized Renderers

#### PlotRenderer (`src/drawables/plot.rs`)
Line chart rendering with WebGPU:
- Vertex generation for line strips
- Anti-aliasing techniques
- Dynamic line width
- Color gradients

#### CandlestickRenderer (`src/drawables/candlestick.rs`)
Financial candlestick charts:
- OHLC data visualization
- Bullish/bearish coloring
- Volume overlay support
- Wick and body rendering

#### Axis Renderers (`src/drawables/[x|y]_axis.rs`)
Dynamic axis rendering:
- Automatic label generation
- Scientific notation support
- Grid line integration
- Tick mark positioning

## Shader System

### Shader Organization
Each renderer has co-located WGSL shaders:

```
src/drawables/
├── plot/
│   ├── mod.rs           # PlotRenderer implementation
│   ├── vertex.wgsl      # Vertex shader
│   └── fragment.wgsl    # Fragment shader
├── candlestick/
│   ├── mod.rs
│   ├── compute.wgsl     # Compute shader for data processing
│   ├── vertex.wgsl
│   └── fragment.wgsl
└── axis/
    ├── mod.rs
    └── shaders.wgsl     # Combined shaders for axes
```

### Shader Loading Pattern

```rust
impl PlotRenderer {
    fn create_pipeline(device: &Device) -> RenderPipeline {
        let vertex_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Plot Vertex Shader"),
            source: ShaderSource::Wgsl(include_str!("vertex.wgsl").into()),
        });
        
        // Pipeline creation...
    }
}
```

## Rendering Pipeline

### Frame Rendering Flow

1. **Begin Frame**: Acquire surface texture
2. **Clear Pass**: Clear with background color
3. **Grid Pass**: Render grid lines
4. **Data Pass**: Render chart data
5. **Axis Pass**: Render axes and labels
6. **UI Pass**: Render overlays (if any)
7. **Present**: Submit command buffer

### Example Render Implementation

```rust
pub async fn render(&mut self, data_store: &DataStore) -> Result<(), RendererError> {
    // Get current texture
    let output = self.render_engine.surface.get_current_texture()?;
    let view = output.texture.create_view(&Default::default());
    
    // Create command encoder
    let mut encoder = self.render_engine.device.create_command_encoder(&Default::default());
    
    // Clear pass
    {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(/* depth attachment */),
            ..Default::default()
        });
        
        // Render components
        self.grid_renderer.render(&mut render_pass, data_store);
        self.plot_renderer.render(&mut render_pass, data_store);
        self.x_axis_renderer.render(&mut render_pass, data_store);
        self.y_axis_renderer.render(&mut render_pass, data_store);
    }
    
    // Submit
    self.render_engine.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    
    Ok(())
}
```

## Performance Optimizations

### Buffer Management

```rust
// Reuse buffers across frames
struct BufferPool {
    vertex_buffers: Vec<Buffer>,
    index_buffers: Vec<Buffer>,
    uniform_buffers: Vec<Buffer>,
}

impl BufferPool {
    fn get_or_create_buffer(&mut self, size: u64, usage: BufferUsages) -> &Buffer {
        // Reuse existing buffer or create new
    }
}
```

### Instanced Rendering

```rust
// For rendering many similar objects (e.g., candlesticks)
struct InstanceData {
    position: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
}

fn render_instanced(&self, instances: &[InstanceData]) {
    // Upload instance data
    // Draw with instance count
}
```

### LOD System

```rust
fn select_lod(&self, zoom_level: f32, data_density: f32) -> LodLevel {
    match (zoom_level, data_density) {
        (z, d) if z < 0.5 && d > 1000.0 => LodLevel::Simplified,
        (z, d) if z > 2.0 || d < 100.0 => LodLevel::Full,
        _ => LodLevel::Normal,
    }
}
```

## Shader Examples

### Vertex Shader (Line Plot)
```wgsl
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) value: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Transform to clip space
    output.clip_position = vec4<f32>(
        input.position.x * 2.0 - 1.0,
        input.value * 2.0 - 1.0,
        0.0,
        1.0
    );
    
    // Color based on value
    output.color = value_to_color(input.value);
    
    return output;
}
```

### Compute Shader (Data Processing)
```wgsl
@group(0) @binding(0) var<storage, read> input_data: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_data: array<vec2<f32>>;
@group(0) @binding(2) var<uniform> params: TransformParams;

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    if (index >= arrayLength(&input_data)) {
        return;
    }
    
    // Transform data point to screen space
    let value = input_data[index];
    let x = f32(index) / f32(params.data_count);
    let y = (value - params.min_value) / (params.max_value - params.min_value);
    
    output_data[index] = vec2<f32>(x, y);
}
```

## Best Practices

1. **Minimize State Changes**: Batch similar draw calls
2. **Use Compute Shaders**: For data transformation
3. **Profile GPU Usage**: Use browser tools
4. **Avoid Shader Compilation**: Cache pipelines
5. **Optimize Buffer Updates**: Use mapping when possible

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("Surface error: {0}")]
    SurfaceError(#[from] wgpu::SurfaceError),
    
    #[error("Pipeline creation failed: {0}")]
    PipelineError(String),
    
    #[error("Shader compilation failed: {0}")]
    ShaderError(String),
    
    #[error("Buffer creation failed: {0}")]
    BufferError(String),
}
```

## Testing

Testing GPU code requires special consideration:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vertex_generation() {
        let data = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let vertices = generate_line_vertices(&data);
        
        assert_eq!(vertices.len(), data.len() * 2); // x,y for each point
    }
    
    #[test]
    fn test_color_interpolation() {
        let color = value_to_color(0.5);
        assert_eq!(color, [0.5, 0.5, 1.0, 1.0]); // Example
    }
}
```

## Future Enhancements

- WebGL fallback support
- Advanced anti-aliasing (FXAA/TAA)
- Custom shader hot-reloading
- Shader caching system
- Multi-pass rendering effects
- Post-processing pipeline
- WebGPU compute animations
- Tessellation for smooth curves