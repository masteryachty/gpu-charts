//! Error recovery and graceful degradation system

use crate::{IntegrationError, Result};
use gpu_charts_shared::Error as SharedError;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Error recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retry the operation
    Retry,
    /// Fallback to a simpler implementation
    Fallback,
    /// Degrade quality/performance
    Degrade,
    /// Skip the operation
    Skip,
    /// Restart the subsystem
    Restart,
    /// Shutdown gracefully
    Shutdown,
}

/// Error context for recovery decisions
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error_type: ErrorType,
    pub subsystem: Subsystem,
    pub severity: ErrorSeverity,
    pub occurrence_count: u32,
    pub first_occurrence: Instant,
    pub last_occurrence: Instant,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    GpuOutOfMemory,
    GpuDeviceLost,
    GpuValidationError,
    DataLoadFailure,
    NetworkTimeout,
    ConfigInvalid,
    RenderFailure,
    BufferCreationFailure,
    ShaderCompilationFailure,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subsystem {
    DataManager,
    Renderer,
    Configuration,
    Network,
    GPU,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Error recovery system
pub struct ErrorRecoverySystem {
    /// Error history
    error_history: Arc<RwLock<ErrorHistory>>,

    /// Recovery strategies
    strategies: Arc<RwLock<HashMap<ErrorType, RecoveryStrategy>>>,

    /// Fallback implementations
    fallbacks: Arc<RwLock<FallbackRegistry>>,

    /// Circuit breakers
    circuit_breakers: Arc<RwLock<HashMap<Subsystem, CircuitBreaker>>>,

    /// Statistics
    stats: Arc<RwLock<RecoveryStats>>,
}

impl ErrorRecoverySystem {
    /// Create a new error recovery system
    pub fn new() -> Self {
        let mut strategies = HashMap::new();

        // Default strategies
        strategies.insert(ErrorType::GpuOutOfMemory, RecoveryStrategy::Degrade);
        strategies.insert(ErrorType::GpuDeviceLost, RecoveryStrategy::Restart);
        strategies.insert(ErrorType::DataLoadFailure, RecoveryStrategy::Retry);
        strategies.insert(ErrorType::NetworkTimeout, RecoveryStrategy::Retry);
        strategies.insert(ErrorType::ConfigInvalid, RecoveryStrategy::Fallback);
        strategies.insert(ErrorType::RenderFailure, RecoveryStrategy::Skip);
        strategies.insert(ErrorType::BufferCreationFailure, RecoveryStrategy::Degrade);
        strategies.insert(
            ErrorType::ShaderCompilationFailure,
            RecoveryStrategy::Fallback,
        );

        let mut circuit_breakers = HashMap::new();
        circuit_breakers.insert(Subsystem::DataManager, CircuitBreaker::new());
        circuit_breakers.insert(Subsystem::Renderer, CircuitBreaker::new());
        circuit_breakers.insert(Subsystem::Configuration, CircuitBreaker::new());
        circuit_breakers.insert(Subsystem::Network, CircuitBreaker::new());
        circuit_breakers.insert(Subsystem::GPU, CircuitBreaker::new());

        Self {
            error_history: Arc::new(RwLock::new(ErrorHistory::new())),
            strategies: Arc::new(RwLock::new(strategies)),
            fallbacks: Arc::new(RwLock::new(FallbackRegistry::new())),
            circuit_breakers: Arc::new(RwLock::new(circuit_breakers)),
            stats: Arc::new(RwLock::new(RecoveryStats::default())),
        }
    }

    /// Handle an error and determine recovery strategy
    pub fn handle_error(&self, error: &IntegrationError) -> RecoveryStrategy {
        let context = self.analyze_error(error);

        // Record error
        self.error_history.write().record(context.clone());
        self.stats.write().errors_handled += 1;

        // Check circuit breaker
        if let Some(breaker) = self.circuit_breakers.write().get_mut(&context.subsystem) {
            if breaker.is_open() {
                log::warn!(
                    "Circuit breaker open for {:?}, using fallback",
                    context.subsystem
                );
                return RecoveryStrategy::Fallback;
            }

            breaker.record_error();
        }

        // Determine strategy based on error pattern
        let strategy = self.determine_strategy(&context);

        log::info!(
            "Error recovery strategy for {:?}: {:?}",
            context.error_type,
            strategy
        );

        // Update stats
        match strategy {
            RecoveryStrategy::Retry => self.stats.write().retries += 1,
            RecoveryStrategy::Fallback => self.stats.write().fallbacks += 1,
            RecoveryStrategy::Degrade => self.stats.write().degradations += 1,
            _ => {}
        }

        strategy
    }

    /// Execute recovery strategy
    pub async fn execute_recovery<F, T>(
        &self,
        strategy: RecoveryStrategy,
        operation: F,
        context: &ErrorContext,
    ) -> Result<T>
    where
        F: Fn() -> Result<T> + Clone,
    {
        match strategy {
            RecoveryStrategy::Retry => self.execute_with_retry(operation, context).await,
            RecoveryStrategy::Fallback => self.execute_with_fallback(operation, context).await,
            RecoveryStrategy::Degrade => self.execute_with_degradation(operation, context).await,
            RecoveryStrategy::Skip => {
                Err(IntegrationError::Recovery("Operation skipped".to_string()))
            }
            RecoveryStrategy::Restart => {
                self.restart_subsystem(context.subsystem).await?;
                operation()
            }
            RecoveryStrategy::Shutdown => Err(IntegrationError::Recovery(
                "System shutdown required".to_string(),
            )),
        }
    }

    /// Analyze error to create context
    fn analyze_error(&self, error: &IntegrationError) -> ErrorContext {
        let (error_type, subsystem, severity) = match error {
            IntegrationError::DataManager(msg) => {
                let error_type = if msg.contains("memory") {
                    ErrorType::GpuOutOfMemory
                } else if msg.contains("load") {
                    ErrorType::DataLoadFailure
                } else {
                    ErrorType::Unknown
                };
                (error_type, Subsystem::DataManager, ErrorSeverity::Medium)
            }
            IntegrationError::Renderer(msg) => {
                let error_type = if msg.contains("device lost") {
                    ErrorType::GpuDeviceLost
                } else if msg.contains("shader") {
                    ErrorType::ShaderCompilationFailure
                } else {
                    ErrorType::RenderFailure
                };
                (error_type, Subsystem::Renderer, ErrorSeverity::High)
            }
            IntegrationError::Configuration(_) => (
                ErrorType::ConfigInvalid,
                Subsystem::Configuration,
                ErrorSeverity::Low,
            ),
            _ => (
                ErrorType::Unknown,
                Subsystem::DataManager,
                ErrorSeverity::Medium,
            ),
        };

        let now = Instant::now();
        let history = self.error_history.read();
        let occurrence_count = history.count_occurrences(error_type, subsystem);

        ErrorContext {
            error_type,
            subsystem,
            severity,
            occurrence_count: occurrence_count as u32,
            first_occurrence: now,
            last_occurrence: now,
            details: error.to_string(),
        }
    }

    /// Determine recovery strategy based on context
    fn determine_strategy(&self, context: &ErrorContext) -> RecoveryStrategy {
        // Check predefined strategies
        if let Some(&strategy) = self.strategies.read().get(&context.error_type) {
            // Escalate if error is recurring
            if context.occurrence_count > 5 {
                match strategy {
                    RecoveryStrategy::Retry => RecoveryStrategy::Fallback,
                    RecoveryStrategy::Fallback => RecoveryStrategy::Degrade,
                    RecoveryStrategy::Degrade => RecoveryStrategy::Shutdown,
                    _ => strategy,
                }
            } else {
                strategy
            }
        } else {
            // Default strategy based on severity
            match context.severity {
                ErrorSeverity::Low => RecoveryStrategy::Skip,
                ErrorSeverity::Medium => RecoveryStrategy::Retry,
                ErrorSeverity::High => RecoveryStrategy::Fallback,
                ErrorSeverity::Critical => RecoveryStrategy::Shutdown,
            }
        }
    }

    /// Execute operation with retry
    async fn execute_with_retry<F, T>(&self, operation: F, context: &ErrorContext) -> Result<T>
    where
        F: Fn() -> Result<T> + Clone,
    {
        let max_retries = 3;
        let mut retry_delay = Duration::from_millis(100);

        for attempt in 0..max_retries {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) if attempt < max_retries - 1 => {
                    log::warn!("Retry attempt {} failed: {}", attempt + 1, e);
                    tokio::time::sleep(retry_delay).await;
                    retry_delay *= 2; // Exponential backoff
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    /// Execute operation with fallback
    async fn execute_with_fallback<F, T>(&self, operation: F, context: &ErrorContext) -> Result<T>
    where
        F: Fn() -> Result<T>,
    {
        // Try original operation first
        match operation() {
            Ok(result) => Ok(result),
            Err(_) => {
                // Use fallback
                log::info!("Using fallback for {:?}", context.error_type);

                // This would use registered fallback implementations
                Err(IntegrationError::Recovery(
                    "No fallback available".to_string(),
                ))
            }
        }
    }

    /// Execute operation with quality degradation
    async fn execute_with_degradation<F, T>(
        &self,
        operation: F,
        context: &ErrorContext,
    ) -> Result<T>
    where
        F: Fn() -> Result<T>,
    {
        log::info!("Degrading quality for {:?}", context.error_type);

        // This would adjust quality settings before retrying
        operation()
    }

    /// Restart a subsystem
    async fn restart_subsystem(&self, subsystem: Subsystem) -> Result<()> {
        log::info!("Restarting subsystem: {:?}", subsystem);

        // Reset circuit breaker
        if let Some(breaker) = self.circuit_breakers.write().get_mut(&subsystem) {
            breaker.reset();
        }

        // TODO: Implement actual subsystem restart

        Ok(())
    }

    /// Get recovery statistics
    pub fn get_stats(&self) -> RecoveryStats {
        self.stats.read().clone()
    }

    /// Register a fallback implementation
    pub fn register_fallback(&self, error_type: ErrorType, fallback: Box<dyn Fallback>) {
        self.fallbacks.write().register(error_type, fallback);
    }
}

/// Error history tracking
struct ErrorHistory {
    entries: VecDeque<ErrorContext>,
    max_entries: usize,
}

impl ErrorHistory {
    fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: 1000,
        }
    }

    fn record(&mut self, context: ErrorContext) {
        self.entries.push_back(context);

        if self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    fn count_occurrences(&self, error_type: ErrorType, subsystem: Subsystem) -> usize {
        let since = Instant::now() - Duration::from_secs(300); // Last 5 minutes

        self.entries
            .iter()
            .filter(|e| {
                e.error_type == error_type && e.subsystem == subsystem && e.last_occurrence > since
            })
            .count()
    }
}

/// Circuit breaker for preventing cascading failures
struct CircuitBreaker {
    failure_count: u32,
    last_failure: Option<Instant>,
    state: CircuitBreakerState,
    threshold: u32,
    timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            failure_count: 0,
            last_failure: None,
            state: CircuitBreakerState::Closed,
            threshold: 5,
            timeout: Duration::from_secs(30),
        }
    }

    fn is_open(&self) -> bool {
        match self.state {
            CircuitBreakerState::Open => {
                // Check if timeout has passed
                if let Some(last) = self.last_failure {
                    if Instant::now().duration_since(last) > self.timeout {
                        return false; // Allow half-open state
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn record_error(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());

        if self.failure_count >= self.threshold {
            self.state = CircuitBreakerState::Open;
            log::warn!(
                "Circuit breaker opened after {} failures",
                self.failure_count
            );
        }
    }

    fn reset(&mut self) {
        self.failure_count = 0;
        self.last_failure = None;
        self.state = CircuitBreakerState::Closed;
    }
}

/// Fallback implementation trait
pub trait Fallback: Send + Sync {
    fn execute(&self) -> Result<Box<dyn std::any::Any>>;
}

/// Registry for fallback implementations
struct FallbackRegistry {
    fallbacks: HashMap<ErrorType, Box<dyn Fallback>>,
}

impl FallbackRegistry {
    fn new() -> Self {
        Self {
            fallbacks: HashMap::new(),
        }
    }

    fn register(&mut self, error_type: ErrorType, fallback: Box<dyn Fallback>) {
        self.fallbacks.insert(error_type, fallback);
    }
}

/// Recovery statistics
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RecoveryStats {
    pub errors_handled: u64,
    pub retries: u64,
    pub fallbacks: u64,
    pub degradations: u64,
    pub circuit_breaks: u64,
    pub successful_recoveries: u64,
}
