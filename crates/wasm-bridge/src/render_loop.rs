//! State-based render loop controller for managing the rendering pipeline
//!
//! This module implements a state machine that controls when preprocessing
//! and rendering occur, allowing for efficient updates based on what changed.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::instance_manager::InstanceManager;

// Type aliases to simplify complex types
type AnimationClosure = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;
type StateChangeListeners = Rc<RefCell<Vec<Rc<dyn Fn(RenderLoopState, RenderLoopState)>>>>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderLoopState {
    /// Initial state - render loop is not running
    Off,

    /// Data has been fetched, need to preprocess
    PreProcess,

    /// Currently running preprocessing pipeline
    PreProcessing,

    /// Preprocessing complete, ready to render
    PreProcessComplete,

    /// Currently rendering frame
    Rendering,

    /// Render complete, no changes needed
    Clean,

    /// Changes detected, need to render
    Dirty,

    /// Error occurred, render loop stopped
    Error,
}

#[derive(Debug, Clone)]
pub enum StateTransitionTrigger {
    /// New data received - requires preprocessing
    DataReceived,

    /// View changed (pan/zoom) - render only
    ViewChanged,

    /// Visual settings changed - render only
    VisualSettingsChanged,

    /// Metric visibility toggled - render only
    MetricVisibilityChanged,

    /// Configuration that requires preprocessing
    DataConfigChanged,

    /// Size changed - may need preprocessing depending on implementation
    Resized {
        requires_preprocessing: bool,
    },

    /// Animation frame
    AnimationTick,

    /// Preprocessing completed
    PreProcessingDone,

    /// Render completed
    RenderDone,

    /// Error occurred
    ErrorOccurred(String),

    /// Manual start/stop
    Start,
    Stop,
}

#[derive(Clone)]
pub enum PreprocessingTask {
    /// Calculate data bounds
    CalculateBounds,

    /// Update GPU buffers
    UpdateBuffers,

    /// Prepare render pipelines
    PreparePipelines,

    /// Custom preprocessing
    Custom(String),
}

#[derive(Clone)]
pub struct RenderLoopController {
    // Current state
    state: Rc<Cell<RenderLoopState>>,

    // Animation frame handling
    animation_frame_id: Rc<RefCell<Option<i32>>>,
    animation_closure: AnimationClosure,

    // Async coordination
    processing_in_progress: Rc<Cell<bool>>,
    rendering_in_progress: Rc<Cell<bool>>,

    // Preprocessing tasks
    preprocessing_tasks: Rc<RefCell<Vec<PreprocessingTask>>>,

    // State change callbacks
    state_change_listeners: StateChangeListeners,
}

impl Default for RenderLoopController {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderLoopController {
    pub fn new() -> Self {
        Self {
            state: Rc::new(Cell::new(RenderLoopState::Off)),
            animation_frame_id: Rc::new(RefCell::new(None)),
            animation_closure: Rc::new(RefCell::new(None)),
            processing_in_progress: Rc::new(Cell::new(false)),
            rendering_in_progress: Rc::new(Cell::new(false)),
            preprocessing_tasks: Rc::new(RefCell::new(vec![
                PreprocessingTask::CalculateBounds,
                PreprocessingTask::UpdateBuffers,
                PreprocessingTask::PreparePipelines,
            ])),
            state_change_listeners: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Get current state
    pub fn get_state(&self) -> RenderLoopState {
        self.state.get()
    }

    /// Add a state change listener
    pub fn add_state_listener(&self, listener: Rc<dyn Fn(RenderLoopState, RenderLoopState)>) {
        self.state_change_listeners.borrow_mut().push(listener);
    }

    /// Trigger a state transition
    pub fn trigger_transition(&self, trigger: StateTransitionTrigger, instance_id: Uuid) {
        let current_state = self.state.get();
        let new_state = self.calculate_next_state(current_state, &trigger);
        log::info!("99999, current_state: {current_state:?}, {new_state:?}, {trigger:?}");
        if new_state != current_state {
            log::info!(
                "State transition: {current_state:?} -> {new_state:?} (trigger: {trigger:?})"
            );

            // Update state
            let old_state = current_state;
            self.state.set(new_state);

            // Notify listeners
            for listener in self.state_change_listeners.borrow().iter() {
                listener(old_state, new_state);
            }

            // Handle state entry actions
            self.on_state_enter(new_state, instance_id);
        }
    }

    /// Calculate next state based on current state and trigger
    fn calculate_next_state(
        &self,
        current: RenderLoopState,
        trigger: &StateTransitionTrigger,
    ) -> RenderLoopState {
        match (current, trigger) {
            // Starting the render loop
            (RenderLoopState::Off, StateTransitionTrigger::Start) => RenderLoopState::Clean,

            // === Data changes that REQUIRE preprocessing ===
            (
                RenderLoopState::Clean | RenderLoopState::Dirty,
                StateTransitionTrigger::DataReceived,
            ) => RenderLoopState::PreProcess,
            (
                RenderLoopState::Clean | RenderLoopState::Dirty,
                StateTransitionTrigger::DataConfigChanged,
            ) => RenderLoopState::PreProcess,

            // === Changes that only need rendering (skip preprocessing) ===
            (RenderLoopState::Clean, StateTransitionTrigger::ViewChanged) => RenderLoopState::Dirty,
            (RenderLoopState::Clean, StateTransitionTrigger::VisualSettingsChanged) => {
                RenderLoopState::Dirty
            }
            (RenderLoopState::Clean, StateTransitionTrigger::MetricVisibilityChanged) => {
                RenderLoopState::Dirty
            }

            // Resized - typically just needs re-render unless explicitly requires preprocessing
            (
                RenderLoopState::Clean,
                StateTransitionTrigger::Resized {
                    requires_preprocessing,
                },
            ) => {
                if *requires_preprocessing {
                    RenderLoopState::PreProcess
                } else {
                    // Resize is just a view change - render only
                    RenderLoopState::Dirty
                }
            }

            // PreProcess automatically transitions to PreProcessing
            (RenderLoopState::PreProcess, _) => RenderLoopState::PreProcessing,

            // PreProcessing completes
            (RenderLoopState::PreProcessing, StateTransitionTrigger::PreProcessingDone) => {
                RenderLoopState::PreProcessComplete
            }

            // Both PreProcessComplete and Dirty go to Rendering
            (RenderLoopState::PreProcessComplete | RenderLoopState::Dirty, _) => {
                RenderLoopState::Rendering
            }

            // Rendering completes
            (RenderLoopState::Rendering, StateTransitionTrigger::RenderDone) => {
                RenderLoopState::Clean
            }

            // Stop from any state goes to Off
            (_, StateTransitionTrigger::Stop) => RenderLoopState::Off,

            // Error from any state
            (_, StateTransitionTrigger::ErrorOccurred(_)) => RenderLoopState::Error,

            // Default: stay in current state
            _ => current,
        }
    }

    /// Handle state entry actions
    fn on_state_enter(&self, state: RenderLoopState, instance_id: Uuid) {
        match state {
            RenderLoopState::Off => {
                self.stop_animation_loop();
            }

            RenderLoopState::Clean => {
                // Start animation loop if not running
                if self.animation_frame_id.borrow().is_none() {
                    self.start_animation_loop(instance_id);
                }
            }

            RenderLoopState::PreProcessing => {
                // Start preprocessing pipeline
                self.start_preprocessing(instance_id);
            }

            RenderLoopState::Rendering => {
                // Start rendering
                self.start_rendering(instance_id);
            }

            _ => {
                // Other states don't have entry actions
            }
        }
    }

    /// Start the animation frame loop
    fn start_animation_loop(&self, instance_id: Uuid) {
        let window = web_sys::window().unwrap();
        let controller = self.clone();

        let closure = Closure::wrap(Box::new(move |timestamp: f64| {
            controller.animation_frame_callback(instance_id, timestamp);
        }) as Box<dyn FnMut(f64)>);

        let handle = window
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .unwrap();

        *self.animation_frame_id.borrow_mut() = Some(handle);
        *self.animation_closure.borrow_mut() = Some(closure);
    }

    /// Stop the animation frame loop
    fn stop_animation_loop(&self) {
        if let Some(id) = self.animation_frame_id.borrow_mut().take() {
            web_sys::window().unwrap().cancel_animation_frame(id).ok();
        }
        self.animation_closure.borrow_mut().take();
    }

    /// Animation frame callback
    fn animation_frame_callback(&self, instance_id: Uuid, _timestamp: f64) {
        let current_state = self.state.get();

        // Determine what to do based on current state
        match current_state {
            RenderLoopState::PreProcess => {
                // Automatically transition to preprocessing
                self.trigger_transition(StateTransitionTrigger::AnimationTick, instance_id);
            }

            RenderLoopState::PreProcessComplete | RenderLoopState::Dirty => {
                // These states should render
                self.trigger_transition(StateTransitionTrigger::AnimationTick, instance_id);
            }

            RenderLoopState::Clean => {
                // Nothing to do, just keep the loop running
            }

            _ => {
                // Other states are handled by their async operations
            }
        }

        // Request next frame if still running
        if self.state.get() != RenderLoopState::Off {
            self.request_next_frame(instance_id);
        }
    }

    /// Request next animation frame
    fn request_next_frame(&self, instance_id: Uuid) {
        let window = web_sys::window().unwrap();
        let controller = self.clone();

        let closure = Closure::once(move |timestamp: f64| {
            controller.animation_frame_callback(instance_id, timestamp);
        });

        let handle = window
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .unwrap();

        closure.forget(); // Let JS manage the closure
        *self.animation_frame_id.borrow_mut() = Some(handle);
    }

    /// Start preprocessing pipeline
    fn start_preprocessing(&self, instance_id: Uuid) {
        if self.processing_in_progress.get() {
            return;
        }

        self.processing_in_progress.set(true);
        let controller = self.clone();

        wasm_bindgen_futures::spawn_local(async move {
            // Execute preprocessing tasks
            match controller.execute_preprocessing(instance_id).await {
                Ok(_) => {
                    controller
                        .trigger_transition(StateTransitionTrigger::PreProcessingDone, instance_id);
                }
                Err(e) => {
                    controller.trigger_transition(
                        StateTransitionTrigger::ErrorOccurred(format!("{e:?}")),
                        instance_id,
                    );
                }
            }

            controller.processing_in_progress.set(false);
        });
    }

    /// Start rendering
    fn start_rendering(&self, instance_id: Uuid) {
        if self.rendering_in_progress.get() {
            return;
        }

        self.rendering_in_progress.set(true);
        let controller = self.clone();

        wasm_bindgen_futures::spawn_local(async move {
            // Execute rendering
            match controller.execute_rendering(instance_id).await {
                Ok(_) => {
                    controller.trigger_transition(StateTransitionTrigger::RenderDone, instance_id);
                }
                Err(e) => {
                    controller.trigger_transition(
                        StateTransitionTrigger::ErrorOccurred(format!("{e:?}")),
                        instance_id,
                    );
                }
            }

            controller.rendering_in_progress.set(false);
        });
    }

    /// Execute preprocessing tasks
    async fn execute_preprocessing(&self, instance_id: Uuid) -> Result<(), JsValue> {
        let tasks = self.preprocessing_tasks.borrow().clone();

        for task in tasks {
            match task {
                PreprocessingTask::CalculateBounds => {
                    self.calculate_bounds(instance_id).await?;
                }
                PreprocessingTask::UpdateBuffers => {
                    self.update_buffers(instance_id).await?;
                }
                PreprocessingTask::PreparePipelines => {
                    self.prepare_pipelines(instance_id).await?;
                }
                PreprocessingTask::Custom(name) => {
                    log::info!("Executing custom preprocessing task: {name}");
                    // Custom task implementation would go here
                }
            }
        }

        Ok(())
    }

    /// Calculate bounds from data
    async fn calculate_bounds(&self, instance_id: Uuid) -> Result<(), JsValue> {
        log::info!("Preprocessing: Calculating bounds");

        // Take the instance temporarily for GPU operations
        let instance_opt = InstanceManager::take_instance(&instance_id);
        match instance_opt {
            Some(mut instance) => {
                {
                    // Clear existing bounds first
                    let data_store = instance.chart_engine.renderer.data_store_mut();
                    data_store.min_max_buffer = None;
                    data_store.min_max_staging_buffer = None;
                    data_store.gpu_min_y = None;
                    data_store.gpu_max_y = None;
                    log::info!("Cleared existing bounds for recalculation");
                }

                // Create a temporary encoder for the compute pass
                let encoder = instance
                    .chart_engine
                    .renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Bounds Calculation Encoder"),
                    });

                // Now perform the actual GPU bounds calculation
                // This will trigger the calculate_min_max_y function in the renderer
                let result = instance
                    .chart_engine
                    .renderer
                    .calculate_bounds(encoder)
                    .await;

                // Put the instance back
                InstanceManager::put_instance(instance_id, instance);

                result.map_err(|e| JsValue::from_str(&format!("Bounds calculation error: {e:?}")))
            }
            None => Err(JsValue::from_str(
                "Failed to take instance for bounds calculation",
            )),
        }
    }

    /// Update GPU buffers with new data
    async fn update_buffers(&self, instance_id: Uuid) -> Result<(), JsValue> {
        log::info!("Preprocessing: Updating GPU buffers");

        // This is where we'd update vertex buffers, uniform buffers, etc.
        // For now, just mark that buffers need updating
        InstanceManager::with_instance_mut(&instance_id, |instance| {
            let data_store = instance.chart_engine.renderer.data_store_mut();
            data_store.mark_dirty();
        })
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;

        Ok(())
    }

    /// Prepare render pipelines
    async fn prepare_pipelines(&self, instance_id: Uuid) -> Result<(), JsValue> {
        log::info!("Preprocessing: Preparing render pipelines");

        // Ensure pipelines are created/updated as needed
        InstanceManager::with_instance_mut(&instance_id, |instance| {
            if let Some(ref mut _multi_renderer) = instance.chart_engine.multi_renderer {
                // Multi-renderer will handle pipeline preparation
                log::info!("Pipelines prepared");
            }
        })
        .ok_or_else(|| JsValue::from_str("Instance not found"))?;

        Ok(())
    }

    /// Execute rendering
    async fn execute_rendering(&self, instance_id: Uuid) -> Result<(), JsValue> {
        log::info!("Executing render");

        // Take the instance temporarily for async rendering
        let instance_opt = InstanceManager::take_instance(&instance_id);
        match instance_opt {
            Some(mut instance) => {
                // Perform the render
                let result = instance.chart_engine.render().await;

                // Put the instance back
                InstanceManager::put_instance(instance_id, instance);

                result.map_err(|e| JsValue::from_str(&format!("Render error: {e:?}")))
            }
            None => Err(JsValue::from_str("Failed to take instance for rendering")),
        }
    }
}
