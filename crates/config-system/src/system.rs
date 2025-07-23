//! Main configuration system that integrates all components

use crate::{
    auto_tuning::{AutoTuner, PerformanceMetrics},
    hot_reload::{ConfigUpdateEvent, HotReloadManager},
    parser::{ConfigFormat, ConfigParser},
    presets::PresetManager,
    schema::SchemaValidator,
    validation::ConfigValidator,
    ConfigError, GpuChartsConfig, Result,
};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock as TokioRwLock};

/// Main configuration system
pub struct ConfigurationSystem {
    /// Current configuration manager with hot-reload
    hot_reload: Arc<HotReloadManager>,

    /// Auto-tuner
    auto_tuner: Arc<AutoTuner>,

    /// Preset manager
    preset_manager: Arc<RwLock<PresetManager>>,

    /// Schema validator
    schema_validator: Arc<SchemaValidator>,

    /// Configuration file paths
    config_paths: Arc<TokioRwLock<Vec<PathBuf>>>,

    /// Event channel for configuration updates
    update_tx: mpsc::UnboundedSender<SystemEvent>,
}

/// System-wide configuration events
#[derive(Debug, Clone)]
pub enum SystemEvent {
    /// Configuration updated
    ConfigUpdated(ConfigUpdateEvent),

    /// Auto-tuning suggestion
    AutoTuneSuggestion(GpuChartsConfig),

    /// Validation error
    ValidationError(String),

    /// Parse error
    ParseError(String),

    /// Update error
    UpdateError(String),

    /// System error
    SystemError(String),
}

impl ConfigurationSystem {
    /// Create a new configuration system
    pub async fn new(
        initial_config: Option<GpuChartsConfig>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<SystemEvent>)> {
        let (update_tx, update_rx) = mpsc::unbounded_channel();

        // Initialize components
        let config = initial_config.unwrap_or_default();
        let schema_validator = Arc::new(SchemaValidator::new()?);

        // Validate initial config
        schema_validator.validate(&config)?;
        ConfigValidator::validate(&config)?;

        // Create hot-reload manager
        let hot_reload = Arc::new(HotReloadManager::new(config, move |cfg| {
            ConfigValidator::validate(cfg)?;
            Ok(())
        }));

        // Create other components
        let auto_tuner = Arc::new(AutoTuner::new());
        let preset_manager = Arc::new(RwLock::new(PresetManager::new()));

        Ok((
            Self {
                hot_reload,
                auto_tuner,
                preset_manager,
                schema_validator,
                config_paths: Arc::new(TokioRwLock::new(Vec::new())),
                update_tx,
            },
            update_rx,
        ))
    }

    /// Get current configuration
    pub fn current(&self) -> Arc<GpuChartsConfig> {
        self.hot_reload.current()
    }

    /// Load configuration from file
    pub async fn load_from_file(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let config = ConfigParser::parse_file(path)?;

        // Validate
        self.schema_validator.validate(&config)?;
        ConfigValidator::validate(&config)?;

        // Update
        self.hot_reload.update(config).await?;

        // Add to loaded paths
        let mut paths = self.config_paths.write().await;
        if !paths.contains(&path.to_path_buf()) {
            paths.push(path.to_path_buf());
        }

        Ok(())
    }

    /// Load configuration from string
    pub async fn load_from_string(&self, content: &str, format: ConfigFormat) -> Result<()> {
        let config = ConfigParser::parse_string(content, format)?;

        // Validate
        self.schema_validator.validate(&config)?;
        ConfigValidator::validate(&config)?;

        // Update
        self.hot_reload.update(config).await?;

        Ok(())
    }

    /// Apply a preset with optional overrides
    pub async fn apply_preset(
        &self,
        preset_name: &str,
        overrides: Option<serde_json::Value>,
    ) -> Result<()> {
        let config = self
            .preset_manager
            .read()
            .apply_preset(preset_name, overrides)?;

        // Validate
        self.schema_validator.validate(&config)?;
        ConfigValidator::validate(&config)?;

        // Update
        self.hot_reload.update(config).await?;

        Ok(())
    }

    /// Update configuration with validation
    pub async fn update(&self, new_config: GpuChartsConfig) -> Result<()> {
        // Validate
        self.schema_validator.validate(&new_config)?;
        ConfigValidator::validate(&new_config)?;

        // Update
        self.hot_reload.update(new_config).await?;

        Ok(())
    }

    /// Rollback configuration
    pub async fn rollback(&self, steps: usize) -> Result<()> {
        self.hot_reload.rollback(steps).await
    }

    /// Process performance metrics for auto-tuning
    pub async fn process_performance_metrics(&self, metrics: PerformanceMetrics) -> Result<()> {
        let current_config = self.current();

        if let Some(suggested_config) =
            self.auto_tuner.analyze_and_tune(&current_config, metrics)?
        {
            // Send suggestion event
            let _ = self
                .update_tx
                .send(SystemEvent::AutoTuneSuggestion(suggested_config.clone()));

            // Apply if auto-tuning is enabled
            if current_config.performance.auto_tuning.enabled {
                self.update(suggested_config).await?;
            }
        }

        Ok(())
    }

    /// Get configuration history
    pub fn get_history(&self) -> Vec<(std::time::Instant, Arc<GpuChartsConfig>)> {
        self.hot_reload.get_history()
    }

    /// Subscribe to configuration updates
    pub fn subscribe_updates(&self) -> tokio::sync::broadcast::Receiver<ConfigUpdateEvent> {
        self.hot_reload.subscribe()
    }

    /// List available presets
    pub fn list_presets(&self) -> Vec<crate::presets::PresetInfo> {
        self.preset_manager.read().list_presets()
    }

    /// Add user preset
    pub fn add_user_preset(&self, name: String, config: GpuChartsConfig) -> Result<()> {
        self.preset_manager.write().add_user_preset(name, config)
    }

    /// Export current configuration
    pub fn export(&self, format: ConfigFormat) -> Result<String> {
        let config = self.current();
        crate::parser::ConfigSerializer::serialize_string(&config, format)
    }

    /// Validate configuration for specific hardware
    pub fn validate_for_hardware(
        &self,
        hardware: &crate::auto_tuning::HardwareCapabilities,
    ) -> Result<Vec<String>> {
        let config = self.current();
        ConfigValidator::validate_for_hardware(&config, hardware)
    }
}

/// Builder for configuration system
pub struct ConfigSystemBuilder {
    initial_config: Option<GpuChartsConfig>,
    config_paths: Vec<PathBuf>,
    enable_auto_tuning: bool,
}

impl ConfigSystemBuilder {
    pub fn new() -> Self {
        Self {
            initial_config: None,
            config_paths: Vec::new(),
            enable_auto_tuning: true,
        }
    }

    pub fn with_config(mut self, config: GpuChartsConfig) -> Self {
        self.initial_config = Some(config);
        self
    }

    pub fn with_config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_paths.push(path.into());
        self
    }

    pub fn with_auto_tuning(mut self, enabled: bool) -> Self {
        self.enable_auto_tuning = enabled;
        self
    }

    pub async fn build(
        self,
    ) -> Result<(ConfigurationSystem, mpsc::UnboundedReceiver<SystemEvent>)> {
        let (mut system, event_rx) = ConfigurationSystem::new(self.initial_config).await?;

        // Load config files
        for path in self.config_paths {
            system.load_from_file(path).await?;
        }

        // Configure auto-tuning
        if !self.enable_auto_tuning {
            let mut config = (*system.current()).clone();
            config.performance.auto_tuning.enabled = false;
            system.update(config).await?;
        }

        Ok((system, event_rx))
    }
}
