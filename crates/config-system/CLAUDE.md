# Config System Crate - CLAUDE.md

This file provides guidance for working with the config-system crate, which manages all configuration and quality presets for the GPU Charts system.

## Overview

The config-system crate provides:
- Centralized configuration management
- Quality preset definitions (Low, Medium, High, Ultra)
- Performance tuning parameters
- Chart appearance settings
- WebGPU feature configuration

## Architecture Position

```
shared-types
    ↑
config-system (this crate)
    ↑
├── data-manager
├── renderer
└── wasm-bridge
```

This crate depends only on shared-types and provides configuration to all other crates.

## Key Components

### ConfigManager (`src/lib.rs`)
The main configuration management interface:

```rust
pub struct ConfigManager {
    config: GpuChartsConfig,
    presets: HashMap<QualityPreset, PresetConfig>,
}

impl ConfigManager {
    pub fn new() -> Self;
    pub fn get_config(&self) -> &GpuChartsConfig;
    pub fn set_quality_preset(&mut self, preset: QualityPreset);
    pub fn update_config<F>(&mut self, updater: F);
}
```

### Quality Presets

Four predefined quality levels optimized for different hardware:

1. **Low**: For integrated graphics and mobile devices
   - 30 FPS target
   - 1K data points
   - Basic rendering features
   - Minimal GPU memory usage

2. **Medium**: For entry-level discrete GPUs
   - 60 FPS target
   - 10K data points
   - Standard rendering quality
   - Balanced performance

3. **High**: For mid-range gaming GPUs
   - 60 FPS target
   - 100K data points
   - Enhanced visual quality
   - MSAA 4x

4. **Ultra**: For high-end GPUs
   - 144 FPS target
   - 1M data points
   - Maximum quality
   - All features enabled

## Configuration Structure

```rust
pub struct GpuChartsConfig {
    // Performance settings
    pub target_fps: u32,
    pub max_data_points: usize,
    pub buffer_size_multiplier: f32,
    
    // Visual settings
    pub line_width: f32,
    pub point_size: f32,
    pub grid_divisions: u32,
    pub enable_antialiasing: bool,
    pub msaa_samples: u32,
    
    // Chart settings
    pub enable_animations: bool,
    pub animation_duration: f32,
    pub enable_hover_effects: bool,
    pub enable_zoom_limits: bool,
    pub min_zoom: f32,
    pub max_zoom: f32,
    
    // Data settings
    pub cache_size: usize,
    pub prefetch_factor: f32,
    pub compression_level: u8,
    
    // Debug settings
    pub show_fps: bool,
    pub show_debug_info: bool,
    pub enable_profiling: bool,
}
```

## Usage Patterns

### Basic Configuration

```rust
// Create with defaults
let mut config_manager = ConfigManager::new();

// Set quality preset
config_manager.set_quality_preset(QualityPreset::High);

// Get current config
let config = config_manager.get_config();
```

### Custom Configuration

```rust
// Update specific settings
config_manager.update_config(|config| {
    config.target_fps = 120;
    config.enable_antialiasing = true;
    config.line_width = 2.0;
});
```

### Dynamic Quality Adjustment

```rust
// Adjust quality based on performance
fn adjust_quality_for_performance(
    config_manager: &mut ConfigManager,
    current_fps: f32,
) {
    let target_fps = config_manager.get_config().target_fps as f32;
    
    if current_fps < target_fps * 0.8 {
        // Downgrade quality
        config_manager.downgrade_quality();
    } else if current_fps > target_fps * 1.2 {
        // Consider upgrading
        config_manager.upgrade_quality();
    }
}
```

## Best Practices

1. **Use Presets First**: Start with a quality preset, then customize
2. **Validate Ranges**: Ensure configuration values are within valid ranges
3. **Profile Impact**: Test performance impact of configuration changes
4. **Provide Defaults**: Always have sensible default values
5. **Document Limits**: Clearly document min/max values for each setting

## Configuration Guidelines

### Performance Tuning

```rust
// For high-frequency data
config.update_config(|c| {
    c.buffer_size_multiplier = 2.0;  // More buffer space
    c.prefetch_factor = 1.5;         // Aggressive prefetching
    c.cache_size = 1024 * 1024;      // 1MB cache
});

// For battery-saving mode
config.update_config(|c| {
    c.target_fps = 30;
    c.enable_animations = false;
    c.enable_hover_effects = false;
});
```

### Visual Quality

```rust
// Maximum quality
config.update_config(|c| {
    c.enable_antialiasing = true;
    c.msaa_samples = 8;
    c.line_width = 3.0;
    c.grid_divisions = 20;
});

// Performance mode
config.update_config(|c| {
    c.enable_antialiasing = false;
    c.msaa_samples = 1;
    c.line_width = 1.0;
    c.grid_divisions = 10;
});
```

## Adding New Configuration Options

1. **Add to GpuChartsConfig struct** in shared-types
2. **Set default value** in Default implementation
3. **Update presets** with appropriate values
4. **Document the option** with ranges and impact
5. **Add validation** if needed

Example:
```rust
// In shared-types
pub struct GpuChartsConfig {
    // ... existing fields
    pub new_feature_enabled: bool,
    pub new_feature_intensity: f32, // 0.0 to 1.0
}

// In config-system
impl ConfigManager {
    fn create_presets() -> HashMap<QualityPreset, PresetConfig> {
        // Update each preset with appropriate values
        presets.insert(QualityPreset::Low, PresetConfig {
            new_feature_enabled: false,
            new_feature_intensity: 0.0,
            // ...
        });
    }
}
```

## Performance Considerations

- Configuration changes may require GPU resource reallocation
- Some changes need full re-initialization (e.g., MSAA samples)
- Cache configuration to avoid repeated lookups
- Use const values for compile-time optimizations where possible

## Testing

Test configuration scenarios:
```rust
#[test]
fn test_preset_performance_characteristics() {
    let config = ConfigManager::new()
        .with_preset(QualityPreset::Low)
        .get_config();
    
    assert!(config.max_data_points <= 10_000);
    assert_eq!(config.target_fps, 30);
}

#[test]
fn test_config_validation() {
    let mut config = ConfigManager::new();
    config.update_config(|c| {
        c.min_zoom = 2.0;
        c.max_zoom = 1.0; // Invalid!
    });
    
    assert!(config.validate().is_err());
}
```

## Future Enhancements

- Hot-reload configuration from files
- A/B testing framework for settings
- Automatic quality detection based on GPU
- Configuration profiles for different use cases
- Network-based configuration updates