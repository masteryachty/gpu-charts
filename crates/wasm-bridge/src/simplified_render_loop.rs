//! Simplified 3-state render loop controller
//!
//! This module implements a simplified state machine with only 3 states:
//! Idle -> Updating -> Rendering -> Idle

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::{
    frame_pacing::{FramePacer, FrameRateTarget},
    instance_manager::InstanceManager,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderState {
    /// System is idle, waiting for updates
    Idle,

    /// System is updating (data fetch, preprocessing, etc.)
    Updating(UpdateType),

    /// System is rendering
    Rendering,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateType {
    /// Data needs fetching and preprocessing
    Data,

    /// View changed (pan/zoom) - render only
    View,

    /// Configuration changed - rebuild pipeline
    Config,
}

#[derive(Debug, Clone)]
pub enum UpdateTrigger {
    /// New data requested
    DataRequested,

    /// View changed (pan/zoom)
    ViewChanged,

    /// Visual settings changed
    VisualSettingsChanged,

    /// Metric visibility toggled
    MetricVisibilityChanged,

    /// Configuration changed
    ConfigChanged,

    /// Window resized
    Resized,

    /// Update completed
    UpdateComplete,

    /// Render completed
    RenderComplete,

    /// Error occurred
    Error(String),

    /// Manual control
    Start,
    Stop,
}

type StateChangeListener = Rc<dyn Fn(RenderState, RenderState)>;

#[derive(Clone)]
pub struct SimplifiedRenderLoop {
    // Current state
    state: Rc<Cell<RenderState>>,

    // Animation frame handling
    animation_frame_id: Rc<RefCell<Option<i32>>>,
    animation_closure: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>,

    // Processing flags
    is_processing: Rc<Cell<bool>>,

    // State change listeners
    state_listeners: Rc<RefCell<Vec<StateChangeListener>>>,

    // Pending updates (can accumulate while processing)
    pending_update: Rc<RefCell<Option<UpdateType>>>,

    // Frame pacing controller
    frame_pacer: Rc<RefCell<Option<FramePacer>>>,
}

impl Default for SimplifiedRenderLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl SimplifiedRenderLoop {
    pub fn new() -> Self {
        // Try to create frame pacer
        let frame_pacer = match FramePacer::new() {
            Ok(pacer) => {
                // Set default to balanced 30 FPS for better performance
                pacer.set_target(FrameRateTarget::Balanced);
                Some(pacer)
            }
            Err(e) => {
                log::warn!("Failed to create frame pacer: {:?}", e);
                None
            }
        };

        Self {
            state: Rc::new(Cell::new(RenderState::Idle)),
            animation_frame_id: Rc::new(RefCell::new(None)),
            animation_closure: Rc::new(RefCell::new(None)),
            is_processing: Rc::new(Cell::new(false)),
            state_listeners: Rc::new(RefCell::new(Vec::new())),
            pending_update: Rc::new(RefCell::new(None)),
            frame_pacer: Rc::new(RefCell::new(frame_pacer)),
        }
    }

    /// Get current state
    pub fn get_state(&self) -> RenderState {
        self.state.get()
    }

    /// Add a state change listener
    pub fn add_state_listener(&self, listener: StateChangeListener) {
        self.state_listeners.borrow_mut().push(listener);
    }

    /// Set frame rate target
    pub fn set_frame_rate_target(&self, target: FrameRateTarget) {
        if let Some(ref pacer) = *self.frame_pacer.borrow() {
            pacer.set_target(target);
        }
    }

    /// Enable or disable frame pacing
    pub fn set_frame_pacing_enabled(&self, enabled: bool) {
        if let Some(ref pacer) = *self.frame_pacer.borrow() {
            pacer.set_enabled(enabled);
        }
    }

    /// Enable or disable adaptive frame rate
    pub fn set_adaptive_frame_rate(&self, enabled: bool) {
        if let Some(ref pacer) = *self.frame_pacer.borrow() {
            pacer.set_adaptive_mode(enabled);
        }
    }

    /// Get frame statistics
    pub fn get_frame_stats(&self) -> Option<crate::frame_pacing::FrameStats> {
        self.frame_pacer
            .borrow()
            .as_ref()
            .map(|pacer| pacer.get_stats())
    }

    /// Process an update trigger
    pub fn trigger(&self, trigger: UpdateTrigger, instance_id: Uuid) {
        match trigger {
            UpdateTrigger::Start => {
                self.start_loop(instance_id);
            }

            UpdateTrigger::Stop => {
                self.stop_loop();
            }

            UpdateTrigger::DataRequested => {
                self.request_update(UpdateType::Data, instance_id);
            }

            UpdateTrigger::ViewChanged
            | UpdateTrigger::VisualSettingsChanged
            | UpdateTrigger::MetricVisibilityChanged
            | UpdateTrigger::Resized => {
                self.request_update(UpdateType::View, instance_id);
            }

            UpdateTrigger::ConfigChanged => {
                self.request_update(UpdateType::Config, instance_id);
            }

            UpdateTrigger::UpdateComplete => {
                self.on_update_complete(instance_id);
            }

            UpdateTrigger::RenderComplete => {
                self.on_render_complete(instance_id);
            }

            UpdateTrigger::Error(msg) => {
                log::error!("Render loop error: {}", msg);
                self.transition_to(RenderState::Idle);
            }
        }
    }

    /// Request an update
    fn request_update(&self, update_type: UpdateType, instance_id: Uuid) {
        let current_state = self.state.get();

        match current_state {
            RenderState::Idle => {
                // Transition immediately to updating
                self.transition_to(RenderState::Updating(update_type));
                self.start_update(update_type, instance_id);
            }

            RenderState::Updating(current_type) => {
                // Already updating - store higher priority update
                let new_priority = self.get_update_priority(update_type);
                let current_priority = self.get_update_priority(current_type);

                if new_priority > current_priority {
                    *self.pending_update.borrow_mut() = Some(update_type);
                }
            }

            RenderState::Rendering => {
                // Store update for after rendering
                let mut pending = self.pending_update.borrow_mut();
                match &*pending {
                    Some(pending_type) => {
                        // Keep higher priority update
                        let new_priority = self.get_update_priority(update_type);
                        let pending_priority = self.get_update_priority(*pending_type);
                        if new_priority > pending_priority {
                            *pending = Some(update_type);
                        }
                    }
                    None => {
                        *pending = Some(update_type);
                    }
                }
            }
        }
    }

    /// Get update priority (higher number = higher priority)
    fn get_update_priority(&self, update_type: UpdateType) -> u8 {
        match update_type {
            UpdateType::Config => 3, // Highest - requires full rebuild
            UpdateType::Data => 2,   // Medium - requires preprocessing
            UpdateType::View => 1,   // Lowest - render only
        }
    }

    /// Transition to a new state
    fn transition_to(&self, new_state: RenderState) {
        let old_state = self.state.get();
        if old_state != new_state {
            log::info!("State transition: {:?} -> {:?}", old_state, new_state);
            self.state.set(new_state);

            // Notify listeners
            for listener in self.state_listeners.borrow().iter() {
                listener(old_state, new_state);
            }
        }
    }

    /// Start the render loop
    fn start_loop(&self, instance_id: Uuid) {
        if self.animation_frame_id.borrow().is_none() {
            self.request_animation_frame(instance_id);
        }
    }

    /// Stop the render loop
    fn stop_loop(&self) {
        if let Some(id) = self.animation_frame_id.borrow_mut().take() {
            web_sys::window().unwrap().cancel_animation_frame(id).ok();
        }
        self.animation_closure.borrow_mut().take();
        self.transition_to(RenderState::Idle);
    }

    /// Request animation frame
    fn request_animation_frame(&self, instance_id: Uuid) {
        let window = web_sys::window().unwrap();
        let controller = self.clone();

        let closure = Closure::once(move |timestamp: f64| {
            controller.animation_frame_callback(instance_id, timestamp);
        });

        let handle = window
            .request_animation_frame(closure.as_ref().unchecked_ref())
            .unwrap();

        closure.forget();
        *self.animation_frame_id.borrow_mut() = Some(handle);
    }

    /// Animation frame callback
    fn animation_frame_callback(&self, instance_id: Uuid, _timestamp: f64) {
        // Keep the loop running if not stopped
        if self.state.get() != RenderState::Idle || self.pending_update.borrow().is_some() {
            self.request_animation_frame(instance_id);
        }

        // Process any pending updates
        if let Some(update_type) = self.pending_update.borrow_mut().take() {
            if self.state.get() == RenderState::Idle {
                self.transition_to(RenderState::Updating(update_type));
                self.start_update(update_type, instance_id);
            }
        }
    }

    /// Start an update
    fn start_update(&self, update_type: UpdateType, instance_id: Uuid) {
        if self.is_processing.get() {
            return;
        }

        self.is_processing.set(true);
        let controller = self.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match update_type {
                UpdateType::Data => {
                    controller.execute_data_update(instance_id).await;
                }
                UpdateType::View => {
                    controller.execute_view_update(instance_id).await;
                }
                UpdateType::Config => {
                    controller.execute_config_update(instance_id).await;
                }
            }

            controller.trigger(UpdateTrigger::UpdateComplete, instance_id);
            controller.is_processing.set(false);
        });
    }

    /// Execute data update (fetch + preprocess)
    async fn execute_data_update(&self, instance_id: Uuid) {
        log::info!("Executing data update");

        // Take instance for GPU operations
        let instance_opt = InstanceManager::take_instance(&instance_id);
        match instance_opt {
            Some(mut instance) => {
                // Clear bounds for recalculation
                {
                    let data_store = instance.chart_engine.renderer.data_store_mut();
                    data_store.min_max_buffer = None;
                    data_store.min_max_staging_buffer = None;
                    data_store.gpu_min_y = None;
                    data_store.gpu_max_y = None;
                }

                // Calculate bounds
                let encoder = instance
                    .chart_engine
                    .renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Data Update Encoder"),
                    });

                if let Err(e) = instance
                    .chart_engine
                    .renderer
                    .calculate_bounds(encoder)
                    .await
                {
                    log::error!("Bounds calculation failed: {:?}", e);
                }

                // Mark data as dirty
                instance.chart_engine.renderer.data_store_mut().mark_dirty();

                // Return instance
                InstanceManager::put_instance(instance_id, instance);
            }
            None => {
                log::error!("Failed to take instance for data update");
            }
        }
    }

    /// Execute view update (render only)
    async fn execute_view_update(&self, _instance_id: Uuid) {
        log::info!("Executing view update");
        // View updates don't need preprocessing, just mark for render
    }

    /// Execute config update (rebuild pipelines)
    async fn execute_config_update(&self, instance_id: Uuid) {
        log::info!("Executing config update");

        InstanceManager::with_instance_mut(&instance_id, |instance| {
            if let Some(ref mut _multi_renderer) = instance.chart_engine.multi_renderer {
                // Multi-renderer will rebuild pipelines as needed
                log::info!("Config update applied");
            }
        });
    }

    /// Handle update completion
    fn on_update_complete(&self, instance_id: Uuid) {
        match self.state.get() {
            RenderState::Updating(_) => {
                // Transition to rendering
                self.transition_to(RenderState::Rendering);
                self.start_render(instance_id);
            }
            _ => {
                log::warn!(
                    "Unexpected update complete in state: {:?}",
                    self.state.get()
                );
            }
        }
    }

    /// Start rendering
    fn start_render(&self, instance_id: Uuid) {
        // Check frame pacing
        if let Some(ref pacer) = *self.frame_pacer.borrow() {
            if !pacer.should_render() {
                // Not time for next frame yet, schedule for later
                let time_until = pacer.time_until_next_frame();
                if time_until > 0.0 {
                    let controller = self.clone();
                    let window = web_sys::window().unwrap();
                    let closure = Closure::once(move || {
                        controller.start_render(instance_id);
                    });

                    window
                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                            closure.as_ref().unchecked_ref(),
                            time_until as i32,
                        )
                        .unwrap();

                    closure.forget();
                    return;
                }
            }

            // Mark frame start
            pacer.begin_frame();
        }

        let controller = self.clone();

        wasm_bindgen_futures::spawn_local(async move {
            controller.execute_render(instance_id).await;

            // Mark frame end
            if let Some(ref pacer) = *controller.frame_pacer.borrow() {
                pacer.end_frame();
            }

            controller.trigger(UpdateTrigger::RenderComplete, instance_id);
        });
    }

    /// Execute render
    async fn execute_render(&self, instance_id: Uuid) {
        log::info!("Executing render");

        let instance_opt = InstanceManager::take_instance(&instance_id);
        match instance_opt {
            Some(mut instance) => {
                if let Err(e) = instance.chart_engine.render().await {
                    log::error!("Render failed: {:?}", e);
                }

                InstanceManager::put_instance(instance_id, instance);
            }
            None => {
                log::error!("Failed to take instance for rendering");
            }
        }
    }

    /// Handle render completion
    fn on_render_complete(&self, instance_id: Uuid) {
        match self.state.get() {
            RenderState::Rendering => {
                // Check for pending updates
                if self.pending_update.borrow().is_some() {
                    // Process pending update immediately
                    self.transition_to(RenderState::Idle);
                    self.animation_frame_callback(instance_id, 0.0);
                } else {
                    // Go back to idle
                    self.transition_to(RenderState::Idle);
                }
            }
            _ => {
                log::warn!(
                    "Unexpected render complete in state: {:?}",
                    self.state.get()
                );
            }
        }
    }
}
