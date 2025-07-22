use serde::{Deserialize, Serialize};
use wasm_storage::simple::SimpleStorage;
use std::cell::RefCell;
use std::rc::Rc;

/// WASM-compatible chart configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    /// Rendering settings
    pub rendering: RenderingConfig,
    
    /// Performance settings
    pub performance: PerformanceConfig,
    
    /// Feature flags
    pub features: FeatureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingConfig {
    /// Target FPS
    pub target_fps: u32,
    
    /// Line width for charts
    pub line_width: f32,
    
    /// Enable antialiasing
    pub antialiasing: bool,
    
    /// Chart colors
    pub colors: ChartColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartColors {
    pub background: [f32; 4],
    pub grid: [f32; 4],
    pub axis: [f32; 4],
    pub plot: [f32; 3],
}

impl Default for RenderingConfig {
    fn default() -> Self {
        Self {
            target_fps: 60,
            line_width: 2.0,
            antialiasing: true,
            colors: ChartColors {
                background: [0.0, 0.0, 0.0, 1.0],
                grid: [0.3, 0.3, 0.3, 0.5],
                axis: [0.7, 0.7, 0.7, 1.0],
                plot: [0.0, 0.5, 1.0],
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum FPS
    pub max_fps: u32,
    
    /// GPU buffer chunk size
    pub chunk_size: usize,
    
    /// Enable GPU memory optimization
    pub optimize_memory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable binary search culling
    pub binary_culling: bool,
    
    /// Enable vertex compression
    pub vertex_compression: bool,
    
    /// Enable GPU vertex generation
    pub gpu_vertex_generation: bool,
    
    /// Enable render bundles
    pub render_bundles: bool,
}

impl Default for ChartConfig {
    fn default() -> Self {
        Self {
            rendering: RenderingConfig::default(),
            performance: PerformanceConfig {
                max_fps: 60,
                chunk_size: 128 * 1024 * 1024, // 128MB chunks
                optimize_memory: true,
            },
            features: FeatureFlags {
                binary_culling: true,
                vertex_compression: true,
                gpu_vertex_generation: true,
                render_bundles: true, // Enable all features
            },
        }
    }
}

/// Configuration manager for the chart
pub struct ConfigManager {
    config: Rc<RefCell<ChartConfig>>,
    storage: SimpleStorage,
    config_key: String,
    on_change_callbacks: Vec<Box<dyn Fn(&ChartConfig)>>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        let storage = SimpleStorage::local();
        let config_key = "gpu_charts_config".to_string();
        
        // Load config from storage or use default
        let config = storage
            .get_json::<ChartConfig>(&config_key)
            .ok()
            .flatten()
            .unwrap_or_default();
        
        Self {
            config: Rc::new(RefCell::new(config)),
            storage,
            config_key,
            on_change_callbacks: Vec::new(),
        }
    }
    
    /// Get the current configuration
    pub fn get(&self) -> ChartConfig {
        self.config.borrow().clone()
    }
    
    /// Update configuration
    pub fn update<F>(&mut self, updater: F) -> Result<(), wasm_bindgen::JsValue>
    where
        F: FnOnce(&mut ChartConfig),
    {
        let old_config = self.config.borrow().clone();
        
        {
            let mut config = self.config.borrow_mut();
            updater(&mut config);
        }
        
        let new_config = self.config.borrow().clone();
        
        // Only save and notify if config actually changed
        if !configs_equal(&old_config, &new_config) {
            // Save to storage
            self.storage.set_json(&self.config_key, &new_config)?;
            
            // Notify callbacks for hot-reload
            for callback in &self.on_change_callbacks {
                callback(&new_config);
            }
            
            log::info!("Configuration updated and hot-reloaded");
        }
        
        Ok(())
    }
    
    /// Set the entire configuration
    pub fn set(&mut self, config: ChartConfig) -> Result<(), wasm_bindgen::JsValue> {
        *self.config.borrow_mut() = config.clone();
        
        // Save to storage
        self.storage.set_json(&self.config_key, &config)?;
        
        // Notify callbacks
        for callback in &self.on_change_callbacks {
            callback(&config);
        }
        
        Ok(())
    }
    
    /// Register a callback for configuration changes
    pub fn on_change<F>(&mut self, callback: F)
    where
        F: Fn(&ChartConfig) + 'static,
    {
        self.on_change_callbacks.push(Box::new(callback));
    }
    
    /// Load a preset configuration
    pub fn load_preset(&mut self, preset: ConfigPreset) -> Result<(), wasm_bindgen::JsValue> {
        let config = match preset {
            ConfigPreset::Performance => Self::performance_preset(),
            ConfigPreset::Quality => Self::quality_preset(),
            ConfigPreset::Balanced => Self::balanced_preset(),
            ConfigPreset::LowPower => Self::low_power_preset(),
        };
        
        self.set(config)
    }
    
    // Preset configurations
    
    fn performance_preset() -> ChartConfig {
        ChartConfig {
            rendering: RenderingConfig {
                target_fps: 144,
                line_width: 1.0,
                antialiasing: false,
                ..Default::default()
            },
            performance: PerformanceConfig {
                max_fps: 144,
                chunk_size: 256 * 1024 * 1024, // 256MB chunks
                optimize_memory: true,
            },
            features: FeatureFlags {
                binary_culling: true,
                vertex_compression: true,
                gpu_vertex_generation: true,
                render_bundles: true, // All features enabled
            },
        }
    }
    
    fn quality_preset() -> ChartConfig {
        ChartConfig {
            rendering: RenderingConfig {
                target_fps: 60,
                line_width: 3.0,
                antialiasing: true,
                ..Default::default()
            },
            performance: PerformanceConfig {
                max_fps: 60,
                chunk_size: 64 * 1024 * 1024, // 64MB chunks
                optimize_memory: false,
            },
            features: FeatureFlags {
                binary_culling: true,
                vertex_compression: true,
                gpu_vertex_generation: true,
                render_bundles: true,
            },
        }
    }
    
    fn balanced_preset() -> ChartConfig {
        ChartConfig::default()
    }
    
    fn low_power_preset() -> ChartConfig {
        ChartConfig {
            rendering: RenderingConfig {
                target_fps: 30,
                line_width: 2.0,
                antialiasing: false,
                ..Default::default()
            },
            performance: PerformanceConfig {
                max_fps: 30,
                chunk_size: 32 * 1024 * 1024, // 32MB chunks
                optimize_memory: true,
            },
            features: FeatureFlags {
                binary_culling: true,
                vertex_compression: true,
                gpu_vertex_generation: true,
                render_bundles: true,
            },
        }
    }
}

/// Configuration presets
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigPreset {
    Performance,
    Quality,
    Balanced,
    LowPower,
}

/// Helper function to compare configurations
fn configs_equal(a: &ChartConfig, b: &ChartConfig) -> bool {
    // Compare all fields
    a.rendering.target_fps == b.rendering.target_fps &&
    a.rendering.line_width == b.rendering.line_width &&
    a.rendering.antialiasing == b.rendering.antialiasing &&
    a.rendering.colors.background == b.rendering.colors.background &&
    a.rendering.colors.grid == b.rendering.colors.grid &&
    a.rendering.colors.axis == b.rendering.colors.axis &&
    a.rendering.colors.plot == b.rendering.colors.plot &&
    a.performance.max_fps == b.performance.max_fps &&
    a.performance.chunk_size == b.performance.chunk_size &&
    a.performance.optimize_memory == b.performance.optimize_memory &&
    a.features.binary_culling == b.features.binary_culling &&
    a.features.vertex_compression == b.features.vertex_compression &&
    a.features.gpu_vertex_generation == b.features.gpu_vertex_generation &&
    a.features.render_bundles == b.features.render_bundles
}