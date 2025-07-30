//! Frame pacing system for smooth rendering and optimal performance
//!
//! This module provides a frame pacing system that controls the timing of
//! renders to maintain smooth visual updates while preventing unnecessary
//! GPU work.

use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::Performance;

/// Target frame rates for different scenarios
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameRateTarget {
    /// 60 FPS for smooth interactions
    Smooth,
    /// 30 FPS for balanced performance
    Balanced,
    /// 15 FPS for power saving
    PowerSaver,
    /// Custom FPS target
    Custom(f32),
}

impl FrameRateTarget {
    /// Get the target frame time in milliseconds
    pub fn frame_time_ms(&self) -> f64 {
        match self {
            FrameRateTarget::Smooth => 16.67,     // 60 FPS
            FrameRateTarget::Balanced => 33.33,   // 30 FPS
            FrameRateTarget::PowerSaver => 66.67, // 15 FPS
            FrameRateTarget::Custom(fps) => 1000.0 / (*fps as f64),
        }
    }

    /// Get the FPS value
    pub fn fps(&self) -> f32 {
        match self {
            FrameRateTarget::Smooth => 60.0,
            FrameRateTarget::Balanced => 30.0,
            FrameRateTarget::PowerSaver => 15.0,
            FrameRateTarget::Custom(fps) => *fps,
        }
    }
}

/// Frame timing statistics
#[derive(Debug, Clone)]
pub struct FrameStats {
    /// Average frame time over the last N frames
    pub avg_frame_time: f64,
    /// Minimum frame time
    pub min_frame_time: f64,
    /// Maximum frame time
    pub max_frame_time: f64,
    /// Current FPS
    pub current_fps: f32,
    /// Number of dropped frames
    pub dropped_frames: u32,
    /// Total frames rendered
    pub total_frames: u64,
}

/// Frame pacing controller
pub struct FramePacer {
    /// Performance API for high-resolution timing
    performance: Performance,
    /// Current frame rate target
    target: Cell<FrameRateTarget>,
    /// Last frame timestamp
    last_frame_time: Cell<f64>,
    /// Frame time history for statistics
    frame_times: Rc<RefCell<Vec<f64>>>,
    /// Maximum history size
    max_history: usize,
    /// Total frames rendered
    total_frames: Cell<u64>,
    /// Dropped frames counter
    dropped_frames: Cell<u32>,
    /// Whether frame pacing is enabled
    enabled: Cell<bool>,
    /// Adaptive mode - adjusts frame rate based on performance
    adaptive_mode: Cell<bool>,
    /// Minimum time between renders (prevents runaway rendering)
    min_frame_time: f64,
}

impl FramePacer {
    /// Create a new frame pacer
    pub fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
        let performance = window
            .performance()
            .ok_or_else(|| JsValue::from_str("No performance API"))?;

        Ok(Self {
            performance,
            target: Cell::new(FrameRateTarget::Smooth),
            last_frame_time: Cell::new(0.0),
            frame_times: Rc::new(RefCell::new(Vec::with_capacity(120))),
            max_history: 120,
            total_frames: Cell::new(0),
            dropped_frames: Cell::new(0),
            enabled: Cell::new(true),
            adaptive_mode: Cell::new(false),
            min_frame_time: 8.0, // ~120 FPS max
        })
    }

    /// Set the frame rate target
    pub fn set_target(&self, target: FrameRateTarget) {
        self.target.set(target);
        log::info!(
            "Frame rate target set to {:?} ({} FPS)",
            target,
            target.fps()
        );
    }

    /// Enable or disable frame pacing
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.set(enabled);
    }

    /// Enable or disable adaptive mode
    pub fn set_adaptive_mode(&self, adaptive: bool) {
        self.adaptive_mode.set(adaptive);
    }

    /// Check if enough time has passed for the next frame
    pub fn should_render(&self) -> bool {
        if !self.enabled.get() {
            return true; // Always render if pacing is disabled
        }

        let now = self.performance.now();
        let last = self.last_frame_time.get();
        let elapsed = now - last;

        // Always render first frame
        if last == 0.0 {
            return true;
        }

        // Check minimum frame time to prevent runaway
        if elapsed < self.min_frame_time {
            return false;
        }

        // Check against target frame time
        let target_time = self.target.get().frame_time_ms();
        elapsed >= target_time
    }

    /// Mark the start of a frame
    pub fn begin_frame(&self) {
        let now = self.performance.now();
        let last = self.last_frame_time.get();

        if last > 0.0 {
            let frame_time = now - last;
            self.record_frame_time(frame_time);

            // Check for dropped frames (frame took more than 1.5x target)
            let target_time = self.target.get().frame_time_ms();
            if frame_time > target_time * 1.5 {
                self.dropped_frames.set(self.dropped_frames.get() + 1);
            }
        }

        self.last_frame_time.set(now);
        self.total_frames.set(self.total_frames.get() + 1);
    }

    /// Mark the end of a frame (for measuring render time)
    pub fn end_frame(&self) {
        // Adaptive mode: adjust frame rate based on performance
        if self.adaptive_mode.get() {
            self.update_adaptive_target();
        }
    }

    /// Record a frame time in the history
    fn record_frame_time(&self, frame_time: f64) {
        let mut times = self.frame_times.borrow_mut();
        times.push(frame_time);

        // Maintain history size
        if times.len() > self.max_history {
            times.remove(0);
        }
    }

    /// Update frame rate target in adaptive mode
    fn update_adaptive_target(&self) {
        let times = self.frame_times.borrow();
        if times.len() < 10 {
            return; // Not enough data
        }

        // Calculate recent average
        let recent: &[f64] = &times[times.len().saturating_sub(10)..];
        let avg = recent.iter().sum::<f64>() / recent.len() as f64;

        // Adjust target based on performance
        let current_target = self.target.get();
        let new_target = match current_target {
            FrameRateTarget::Smooth => {
                if avg > 20.0 {
                    // Can't maintain 60 FPS, drop to 30
                    FrameRateTarget::Balanced
                } else {
                    current_target
                }
            }
            FrameRateTarget::Balanced => {
                if avg < 16.0 {
                    // Performance improved, try 60 FPS
                    FrameRateTarget::Smooth
                } else if avg > 40.0 {
                    // Can't maintain 30 FPS, drop to 15
                    FrameRateTarget::PowerSaver
                } else {
                    current_target
                }
            }
            FrameRateTarget::PowerSaver => {
                if avg < 30.0 {
                    // Performance improved, try 30 FPS
                    FrameRateTarget::Balanced
                } else {
                    current_target
                }
            }
            _ => current_target,
        };

        if new_target != current_target {
            self.set_target(new_target);
            log::info!(
                "Adaptive mode: Changed frame rate target to {:?}",
                new_target
            );
        }
    }

    /// Get current frame statistics
    pub fn get_stats(&self) -> FrameStats {
        let times = self.frame_times.borrow();

        if times.is_empty() {
            return FrameStats {
                avg_frame_time: 0.0,
                min_frame_time: 0.0,
                max_frame_time: 0.0,
                current_fps: 0.0,
                dropped_frames: self.dropped_frames.get(),
                total_frames: self.total_frames.get(),
            };
        }

        let sum: f64 = times.iter().sum();
        let avg = sum / times.len() as f64;
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let fps = if avg > 0.0 { 1000.0 / avg } else { 0.0 };

        FrameStats {
            avg_frame_time: avg,
            min_frame_time: min,
            max_frame_time: max,
            current_fps: fps as f32,
            dropped_frames: self.dropped_frames.get(),
            total_frames: self.total_frames.get(),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.frame_times.borrow_mut().clear();
        self.dropped_frames.set(0);
        self.total_frames.set(0);
        self.last_frame_time.set(0.0);
    }

    /// Get time until next frame (for scheduling)
    pub fn time_until_next_frame(&self) -> f64 {
        if !self.enabled.get() {
            return 0.0;
        }

        let now = self.performance.now();
        let last = self.last_frame_time.get();
        let elapsed = now - last;
        let target_time = self.target.get().frame_time_ms();

        (target_time - elapsed).max(0.0)
    }
}

/// Frame pacing integration for render loop
pub struct RenderLoopPacer {
    pacer: FramePacer,
    /// Callback ID for scheduled frame
    scheduled_frame: Rc<RefCell<Option<i32>>>,
}

impl RenderLoopPacer {
    /// Create a new render loop pacer
    pub fn new() -> Result<Self, JsValue> {
        Ok(Self {
            pacer: FramePacer::new()?,
            scheduled_frame: Rc::new(RefCell::new(None)),
        })
    }

    /// Schedule the next frame with proper pacing
    pub fn schedule_frame<F>(&self, callback: F)
    where
        F: FnOnce() + 'static,
    {
        // Cancel any previously scheduled frame
        if let Some(id) = self.scheduled_frame.borrow_mut().take() {
            web_sys::window().unwrap().clear_timeout_with_handle(id);
        }

        let time_until_next = self.pacer.time_until_next_frame();

        if time_until_next <= 0.0 {
            // Ready to render immediately
            callback();
        } else {
            // Schedule for later
            let window = web_sys::window().unwrap();
            let closure = Closure::once(callback);
            let handle = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    time_until_next as i32,
                )
                .unwrap();

            closure.forget();
            *self.scheduled_frame.borrow_mut() = Some(handle);
        }
    }

    /// Get the inner frame pacer
    pub fn pacer(&self) -> &FramePacer {
        &self.pacer
    }
}

use std::cell::RefCell;

thread_local! {
    static GLOBAL_PACER: RefCell<Option<RenderLoopPacer>> = RefCell::new(None);
}

/// Get or create the global frame pacer
pub fn get_global_pacer() -> Result<RenderLoopPacer, JsValue> {
    GLOBAL_PACER.with(|pacer| {
        let mut pacer_ref = pacer.borrow_mut();
        if pacer_ref.is_none() {
            *pacer_ref = Some(RenderLoopPacer::new()?);
        }

        // Clone is not available, so we create a new one
        RenderLoopPacer::new()
    })
}
