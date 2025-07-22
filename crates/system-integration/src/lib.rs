//! Phase 3 System Integration
//!
//! This module provides seamless integration between all GPU Charts subsystems,
//! including DataManager, Renderer, and Configuration System.

pub mod api;
pub mod bridge;
pub mod error_recovery;
pub mod lifecycle;
pub mod unified_api;

use gpu_charts_shared::{Error as SharedError, Result as SharedResult};
use thiserror::Error;

/// System integration errors
#[derive(Error, Debug)]
pub enum IntegrationError {
    #[error("Data manager error: {0}")]
    DataManager(String),

    #[error("Renderer error: {0}")]
    Renderer(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Lifecycle error: {0}")]
    Lifecycle(String),

    #[error("Bridge error: {0}")]
    Bridge(String),

    #[error("Recovery error: {0}")]
    Recovery(String),

    #[error("Shared error: {0}")]
    Shared(#[from] SharedError),
}

pub type Result<T> = std::result::Result<T, IntegrationError>;

/// Main system integration hub
pub struct SystemIntegration {
    /// Data manager bridge
    data_bridge: bridge::DataManagerBridge,

    /// Renderer bridge
    renderer_bridge: bridge::RendererBridge,

    /// Lifecycle coordinator
    lifecycle: lifecycle::LifecycleCoordinator,

    /// Error recovery system
    recovery: error_recovery::ErrorRecoverySystem,

    /// Unified API
    api: unified_api::UnifiedApi,
}

impl SystemIntegration {
    /// Create a new system integration
    pub async fn new(
        device: std::sync::Arc<wgpu::Device>,
        queue: std::sync::Arc<wgpu::Queue>,
        config: gpu_charts_config::GpuChartsConfig,
    ) -> Result<Self> {
        // Initialize bridges
        let data_bridge =
            bridge::DataManagerBridge::new(device.clone(), queue.clone(), &config).await?;

        let renderer_bridge =
            bridge::RendererBridge::new(device.clone(), queue.clone(), &config).await?;

        // Initialize lifecycle coordinator
        let lifecycle = lifecycle::LifecycleCoordinator::new();

        // Initialize error recovery
        let recovery = error_recovery::ErrorRecoverySystem::new();

        // Initialize unified API
        let api = unified_api::UnifiedApi::new(
            data_bridge.clone(),
            renderer_bridge.clone(),
            lifecycle.clone(),
        );

        Ok(Self {
            data_bridge,
            renderer_bridge,
            lifecycle,
            recovery,
            api,
        })
    }

    /// Get the unified API
    pub fn api(&self) -> &unified_api::UnifiedApi {
        &self.api
    }

    /// Update configuration
    pub async fn update_config(
        &mut self,
        config: gpu_charts_config::GpuChartsConfig,
    ) -> Result<()> {
        self.data_bridge.update_config(&config).await?;
        self.renderer_bridge.update_config(&config).await?;
        Ok(())
    }

    /// Get system statistics
    pub fn get_stats(&self) -> SystemStats {
        SystemStats {
            data_manager: self.data_bridge.get_stats(),
            renderer: self.renderer_bridge.get_stats(),
            lifecycle: self.lifecycle.get_stats(),
            recovery: self.recovery.get_stats(),
        }
    }
}

/// System-wide statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemStats {
    pub data_manager: bridge::DataManagerStats,
    pub renderer: bridge::RendererStats,
    pub lifecycle: lifecycle::LifecycleStats,
    pub recovery: error_recovery::RecoveryStats,
}
