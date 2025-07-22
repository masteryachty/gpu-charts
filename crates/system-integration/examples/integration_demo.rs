//! System Integration demonstration

use gpu_charts_config::GpuChartsConfig;
use gpu_charts_data::{BufferMetadata, DataSource};
use gpu_charts_integration::{
    lifecycle::LifecycleEvent,
    unified_api::{ChartBuilder, ChartType},
    SystemIntegration,
};
use gpu_charts_renderer::{PerformanceMetrics, Viewport};
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("GPU Charts System Integration Demo\n");

    // Create mock GPU device and queue
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or("Failed to find adapter")?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("GPU Charts Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )
        .await?;

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    // Create configuration
    let config = GpuChartsConfig::default();

    // Create system integration
    let mut system = SystemIntegration::new(device.clone(), queue.clone(), config).await?;

    // Subscribe to lifecycle events
    let mut lifecycle_rx = system.lifecycle.subscribe();
    tokio::spawn(async move {
        while let Some(event) = lifecycle_rx.recv().await {
            match event {
                LifecycleEvent::StateChanged(old, new) => {
                    println!("Lifecycle state changed: {:?} -> {:?}", old, new);
                }
                LifecycleEvent::InitComplete => {
                    println!("System initialization complete!");
                }
                LifecycleEvent::ErrorOccurred(error) => {
                    eprintln!("Error occurred: {}", error);
                }
                _ => {}
            }
        }
    });

    // Get the unified API
    let api = system.api();

    // Initialize the system
    println!("Initializing system...");
    api.initialize().await?;

    // Create a chart using the fluent builder
    println!("\nCreating a line chart...");
    let chart_id = ChartBuilder::new(ChartType::Line)
        .with_visual_config(gpu_charts_shared::VisualConfig {
            background_color: [0.0, 0.0, 0.0, 1.0],
            grid_color: [0.2, 0.2, 0.2, 1.0],
            text_color: [1.0, 1.0, 1.0, 1.0],
            margin_percent: 0.05,
            show_grid: true,
            show_axes: true,
        })
        .add_data(
            DataSource::Http {
                url: "https://api.example.com/data".to_string(),
                headers: std::collections::HashMap::new(),
            },
            BufferMetadata {
                row_count: 10000,
                column_count: 2,
                data_type: gpu_charts_shared::DataType::F32,
                byte_size: 10000 * 2 * 4,
            },
        )
        .with_viewport(Viewport {
            x_min: 0.0,
            x_max: 1000.0,
            y_min: -100.0,
            y_max: 100.0,
        })
        .build(api)
        .await?;

    println!("Created chart with ID: {}", chart_id);

    // List all charts
    println!("\nActive charts:");
    for chart_info in api.list_charts() {
        println!(
            "  - {} ({:?}): {} data sources",
            chart_info.id, chart_info.chart_type, chart_info.data_count
        );
    }

    // Simulate rendering
    println!("\nSimulating render cycle...");

    // Create a mock surface texture
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: 1920,
        height: 1080,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Mock Surface Texture"),
        size: wgpu::Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface_config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let surface_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Create command encoder
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    // Render the chart
    let metrics = PerformanceMetrics {
        fps: 60.0,
        frame_time: 16.67,
        gpu_time: 10.0,
        cpu_time: 5.0,
        draw_calls: 15,
        vertices_rendered: 50000,
        gpu_memory_used: 100 * 1024 * 1024,
    };

    match api.render_chart(chart_id, &mut encoder, &surface_view, &metrics) {
        Ok(_) => println!("Chart rendered successfully!"),
        Err(e) => eprintln!("Render error: {}", e),
    }

    // Submit commands
    queue.submit(std::iter::once(encoder.finish()));

    // Get system statistics
    println!("\nSystem Statistics:");
    let stats = system.get_stats();
    println!("{}", serde_json::to_string_pretty(&stats)?);

    // Test error recovery
    println!("\nTesting error recovery...");
    let error = gpu_charts_integration::IntegrationError::Renderer(
        "Simulated GPU out of memory".to_string(),
    );
    let recovery_strategy = system.recovery.handle_error(&error);
    println!("Recovery strategy for GPU OOM: {:?}", recovery_strategy);

    // Update configuration
    println!("\nUpdating configuration...");
    let mut new_config = GpuChartsConfig::default();
    new_config.rendering.target_fps = 144;
    new_config.performance.lod_bias = 0.5;
    system.update_config(new_config).await?;
    println!("Configuration updated!");

    // Simulate viewport update
    println!("\nUpdating viewport...");
    api.update_viewport(
        chart_id,
        Viewport {
            x_min: 100.0,
            x_max: 900.0,
            y_min: -50.0,
            y_max: 50.0,
        },
    )?;

    // Clean up
    println!("\nCleaning up...");
    api.delete_chart(chart_id).await?;

    // Shutdown
    println!("\nShutting down system...");
    api.shutdown().await?;

    println!("\nDemo completed successfully!");

    Ok(())
}
