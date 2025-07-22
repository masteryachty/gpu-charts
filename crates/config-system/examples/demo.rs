//! Configuration system demonstration

use gpu_charts_config::{
    auto_tuning::PerformanceMetrics, parser::ConfigFormat, ConfigSystemBuilder,
    ConfigurationSystem, SystemEvent,
};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("GPU Charts Configuration System Demo\n");

    // Build configuration system
    let (mut system, mut event_rx) = ConfigSystemBuilder::new()
        .with_config_file("examples/config.yaml")
        .with_auto_tuning(true)
        .with_file_watching(true)
        .build()
        .await?;

    // Spawn event handler
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                SystemEvent::ConfigUpdated(update) => {
                    println!("Configuration updated:");
                    println!("  Old version: {}", update.old_version);
                    println!("  New version: {}", update.new_version);
                    println!("  Changed fields: {:?}", update.changed_fields);
                }
                SystemEvent::FileChanged(file_event) => {
                    println!("Configuration file changed: {:?}", file_event.path);
                }
                SystemEvent::AutoTuneSuggestion(config) => {
                    println!("Auto-tuning suggestion received");
                    println!("  Target FPS: {}", config.rendering.target_fps);
                    println!("  Resolution scale: {}", config.rendering.resolution_scale);
                }
                SystemEvent::ValidationError(error) => {
                    eprintln!("Validation error: {}", error);
                }
                SystemEvent::SystemError(error) => {
                    eprintln!("System error: {}", error);
                }
            }
        }
    });

    // Display current configuration
    let config = system.current();
    println!("Current Configuration:");
    println!("  Version: {}", config.version);
    println!("  Target FPS: {}", config.rendering.target_fps);
    println!("  Resolution Scale: {}", config.rendering.resolution_scale);
    println!("  GPU Culling: {}", config.performance.gpu_culling);
    println!("  Auto-tuning: {}", config.performance.auto_tuning.enabled);
    println!();

    // List available presets
    println!("Available Presets:");
    for preset in system.list_presets() {
        println!("  {} - {}", preset.name, preset.description);
    }
    println!();

    // Demonstrate preset application
    println!("Applying 'performance' preset...");
    system.apply_preset("performance", None).await?;

    let config = system.current();
    println!("After preset:");
    println!("  Target FPS: {}", config.rendering.target_fps);
    println!("  Resolution Scale: {}", config.rendering.resolution_scale);
    println!("  Antialiasing: {}", config.rendering.antialiasing);
    println!();

    // Simulate performance metrics
    println!("Simulating performance metrics...");
    let metrics = PerformanceMetrics {
        avg_fps: 45.0,
        min_fps: 30.0,
        max_fps: 55.0,
        frame_time_p50: 22.0,
        frame_time_p90: 33.0,
        frame_time_p99: 45.0,
        gpu_utilization: 95.0,
        gpu_memory_used: 6_000_000_000,
        cpu_utilization: 60.0,
        avg_draw_calls: 150.0,
        avg_vertices: 1_000_000,
    };

    system.process_performance_metrics(metrics).await?;
    println!("Performance metrics processed - auto-tuning may have adjusted settings");

    let config = system.current();
    println!("After auto-tuning:");
    println!("  Resolution Scale: {}", config.rendering.resolution_scale);
    println!("  LOD Bias: {}", config.performance.lod_bias);
    println!();

    // Export configuration
    let yaml = system.export(ConfigFormat::Yaml)?;
    println!("Exported configuration (first 500 chars):");
    println!("{}", &yaml[..yaml.len().min(500)]);
    println!("...");

    // Keep running to watch for file changes
    println!("\nWatching for configuration file changes. Press Ctrl+C to exit.");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
