//! Lifecycle coordination between subsystems

use crate::{IntegrationError, Result};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

/// Lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Uninitialized,
    Initializing,
    Ready,
    Running,
    Paused,
    ShuttingDown,
    Terminated,
    Error,
}

/// Lifecycle events
#[derive(Debug, Clone)]
pub enum LifecycleEvent {
    StateChanged(LifecycleState, LifecycleState), // old, new
    InitComplete,
    ShutdownRequested,
    ErrorOccurred(String),
    ResourceCreated(ResourceType, String),
    ResourceDestroyed(ResourceType, String),
}

#[derive(Debug, Clone)]
pub enum ResourceType {
    DataBuffer,
    RenderPipeline,
    Surface,
    ConfigFile,
}

/// Lifecycle coordinator manages system-wide lifecycle
#[derive(Clone)]
pub struct LifecycleCoordinator {
    /// Current state
    state: Arc<RwLock<LifecycleState>>,

    /// State history
    state_history: Arc<RwLock<Vec<(Instant, LifecycleState)>>>,

    /// Event sender
    event_tx: mpsc::UnboundedSender<LifecycleEvent>,

    /// Resource tracking
    resources: Arc<RwLock<ResourceTracker>>,

    /// Statistics
    stats: Arc<RwLock<LifecycleStats>>,
}

impl LifecycleCoordinator {
    /// Create a new lifecycle coordinator
    pub fn new() -> Self {
        let (event_tx, _) = mpsc::unbounded_channel();

        Self {
            state: Arc::new(RwLock::new(LifecycleState::Uninitialized)),
            state_history: Arc::new(RwLock::new(vec![(
                Instant::now(),
                LifecycleState::Uninitialized,
            )])),
            event_tx,
            resources: Arc::new(RwLock::new(ResourceTracker::new())),
            stats: Arc::new(RwLock::new(LifecycleStats::default())),
        }
    }

    /// Subscribe to lifecycle events
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<LifecycleEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        // TODO: Implement proper subscription management
        rx
    }

    /// Get current state
    pub fn get_state(&self) -> LifecycleState {
        *self.state.read()
    }

    /// Transition to a new state
    pub fn transition_to(&self, new_state: LifecycleState) -> Result<()> {
        let old_state = {
            let mut state = self.state.write();
            let old = *state;

            // Validate transition
            if !self.is_valid_transition(old, new_state) {
                return Err(IntegrationError::Lifecycle(format!(
                    "Invalid transition from {:?} to {:?}",
                    old, new_state
                )));
            }

            *state = new_state;
            old
        };

        // Record history
        self.state_history.write().push((Instant::now(), new_state));

        // Send event
        let _ = self
            .event_tx
            .send(LifecycleEvent::StateChanged(old_state, new_state));

        // Update stats
        self.stats.write().state_transitions += 1;

        Ok(())
    }

    /// Initialize the system
    pub async fn initialize(&self) -> Result<()> {
        self.transition_to(LifecycleState::Initializing)?;

        // Perform initialization steps
        log::info!("Initializing GPU Charts system...");

        // Simulate initialization work
        tokio::time::sleep(Duration::from_millis(100)).await;

        self.transition_to(LifecycleState::Ready)?;
        let _ = self.event_tx.send(LifecycleEvent::InitComplete);

        Ok(())
    }

    /// Start the system
    pub async fn start(&self) -> Result<()> {
        if self.get_state() != LifecycleState::Ready {
            return Err(IntegrationError::Lifecycle(
                "System must be in Ready state to start".to_string(),
            ));
        }

        self.transition_to(LifecycleState::Running)?;
        Ok(())
    }

    /// Pause the system
    pub async fn pause(&self) -> Result<()> {
        if self.get_state() != LifecycleState::Running {
            return Err(IntegrationError::Lifecycle(
                "System must be Running to pause".to_string(),
            ));
        }

        self.transition_to(LifecycleState::Paused)?;
        Ok(())
    }

    /// Resume the system
    pub async fn resume(&self) -> Result<()> {
        if self.get_state() != LifecycleState::Paused {
            return Err(IntegrationError::Lifecycle(
                "System must be Paused to resume".to_string(),
            ));
        }

        self.transition_to(LifecycleState::Running)?;
        Ok(())
    }

    /// Shutdown the system
    pub async fn shutdown(&self) -> Result<()> {
        let _ = self.event_tx.send(LifecycleEvent::ShutdownRequested);
        self.transition_to(LifecycleState::ShuttingDown)?;

        // Perform cleanup
        log::info!("Shutting down GPU Charts system...");

        // Wait for resources to be released
        let resources_released = self.wait_for_resource_release().await?;
        log::info!("Released {} resources", resources_released);

        self.transition_to(LifecycleState::Terminated)?;
        Ok(())
    }

    /// Report an error
    pub fn report_error(&self, error: String) {
        let _ = self
            .event_tx
            .send(LifecycleEvent::ErrorOccurred(error.clone()));
        let _ = self.transition_to(LifecycleState::Error);
        self.stats.write().errors_reported += 1;
    }

    /// Register a resource
    pub fn register_resource(&self, resource_type: ResourceType, id: String) {
        self.resources
            .write()
            .register(resource_type.clone(), id.clone());
        let _ = self
            .event_tx
            .send(LifecycleEvent::ResourceCreated(resource_type, id));
    }

    /// Unregister a resource
    pub fn unregister_resource(&self, resource_type: ResourceType, id: String) {
        self.resources.write().unregister(&resource_type, &id);
        let _ = self
            .event_tx
            .send(LifecycleEvent::ResourceDestroyed(resource_type, id));
    }

    /// Get statistics
    pub fn get_stats(&self) -> LifecycleStats {
        self.stats.read().clone()
    }

    /// Check if a state transition is valid
    fn is_valid_transition(&self, from: LifecycleState, to: LifecycleState) -> bool {
        use LifecycleState::*;

        match (from, to) {
            // Initialization flow
            (Uninitialized, Initializing) => true,
            (Initializing, Ready) => true,
            (Initializing, Error) => true,

            // Normal operation
            (Ready, Running) => true,
            (Running, Paused) => true,
            (Paused, Running) => true,

            // Shutdown flow
            (Ready | Running | Paused | Error, ShuttingDown) => true,
            (ShuttingDown, Terminated) => true,

            // Error handling
            (_, Error) => true,
            (Error, Initializing) => true,

            // Invalid transitions
            _ => false,
        }
    }

    /// Wait for all resources to be released
    async fn wait_for_resource_release(&self) -> Result<usize> {
        let start = Instant::now();
        let timeout = Duration::from_secs(10);

        loop {
            let count = self.resources.read().total_count();
            if count == 0 {
                return Ok(0);
            }

            if start.elapsed() > timeout {
                return Err(IntegrationError::Lifecycle(format!(
                    "Timeout waiting for {} resources to release",
                    count
                )));
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Resource tracker
struct ResourceTracker {
    resources: std::collections::HashMap<ResourceType, std::collections::HashSet<String>>,
}

impl ResourceTracker {
    fn new() -> Self {
        Self {
            resources: std::collections::HashMap::new(),
        }
    }

    fn register(&mut self, resource_type: ResourceType, id: String) {
        self.resources
            .entry(resource_type)
            .or_insert_with(std::collections::HashSet::new)
            .insert(id);
    }

    fn unregister(&mut self, resource_type: &ResourceType, id: &str) {
        if let Some(set) = self.resources.get_mut(resource_type) {
            set.remove(id);
            if set.is_empty() {
                self.resources.remove(resource_type);
            }
        }
    }

    fn total_count(&self) -> usize {
        self.resources.values().map(|set| set.len()).sum()
    }
}

/// Lifecycle statistics
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct LifecycleStats {
    pub state_transitions: u64,
    pub errors_reported: u64,
    pub resources_created: u64,
    pub resources_destroyed: u64,
    pub uptime_seconds: u64,
}

/// Startup sequence coordinator
pub struct StartupSequence {
    steps: Vec<StartupStep>,
}

struct StartupStep {
    name: String,
    action: Box<dyn Fn() -> oneshot::Receiver<Result<()>> + Send + Sync>,
    timeout: Duration,
}

impl StartupSequence {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Add a startup step
    pub fn add_step<F>(&mut self, name: String, timeout: Duration, action: F)
    where
        F: Fn() -> oneshot::Receiver<Result<()>> + Send + Sync + 'static,
    {
        self.steps.push(StartupStep {
            name,
            action: Box::new(action),
            timeout,
        });
    }

    /// Execute the startup sequence
    pub async fn execute(&self) -> Result<()> {
        for (i, step) in self.steps.iter().enumerate() {
            log::info!("Startup step {}/{}: {}", i + 1, self.steps.len(), step.name);

            let rx = (step.action)();

            match tokio::time::timeout(step.timeout, rx).await {
                Ok(Ok(Ok(()))) => {
                    log::info!("Step '{}' completed successfully", step.name);
                }
                Ok(Ok(Err(e))) => {
                    return Err(IntegrationError::Lifecycle(format!(
                        "Step '{}' failed: {}",
                        step.name, e
                    )));
                }
                Ok(Err(_)) => {
                    return Err(IntegrationError::Lifecycle(format!(
                        "Step '{}' channel error",
                        step.name
                    )));
                }
                Err(_) => {
                    return Err(IntegrationError::Lifecycle(format!(
                        "Step '{}' timed out after {:?}",
                        step.name, step.timeout
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Shutdown sequence coordinator
pub struct ShutdownSequence {
    steps: Vec<ShutdownStep>,
}

struct ShutdownStep {
    name: String,
    action: Box<dyn Fn() -> oneshot::Receiver<()> + Send + Sync>,
    timeout: Duration,
}

impl ShutdownSequence {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Add a shutdown step
    pub fn add_step<F>(&mut self, name: String, timeout: Duration, action: F)
    where
        F: Fn() -> oneshot::Receiver<()> + Send + Sync + 'static,
    {
        self.steps.push(ShutdownStep {
            name,
            action: Box::new(action),
            timeout,
        });
    }

    /// Execute the shutdown sequence
    pub async fn execute(&self) -> Result<()> {
        // Execute in reverse order
        for (i, step) in self.steps.iter().rev().enumerate() {
            log::info!(
                "Shutdown step {}/{}: {}",
                i + 1,
                self.steps.len(),
                step.name
            );

            let rx = (step.action)();

            match tokio::time::timeout(step.timeout, rx).await {
                Ok(Ok(())) => {
                    log::info!("Step '{}' completed", step.name);
                }
                Ok(Err(_)) => {
                    log::warn!("Step '{}' channel error", step.name);
                }
                Err(_) => {
                    log::warn!("Step '{}' timed out, continuing shutdown", step.name);
                }
            }
        }

        Ok(())
    }
}
