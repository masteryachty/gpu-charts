//! Immediate mode update system for simplified state management
//!
//! This module provides a simpler alternative to the complex 3-state render loop.
//! Updates are processed immediately without complex async state transitions.

use std::cell::Cell;
use std::rc::Rc;

/// Events that can trigger updates in the chart
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateEvent {
    /// Data has changed (requires preprocessing)
    DataChanged,

    /// View has changed (pan/zoom)
    ViewChanged { zoom: bool, pan: bool },

    /// Configuration changed (requires pipeline rebuild)
    ConfigChanged,

    /// Window resized
    Resized(u32, u32),

    /// Metric visibility toggled
    MetricVisibilityChanged,
}

/// Current state of the renderer
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderState {
    /// Ready to accept updates
    Ready,

    /// Currently busy with an operation
    Busy(BusyReason),
}

/// Reason why the renderer is busy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BusyReason {
    /// Fetching data from server
    FetchingData,

    /// Processing data (calculating bounds, etc.)
    Preprocessing,

    /// Currently rendering a frame
    Rendering,
}

/// Immediate mode updater - processes updates synchronously
pub struct ImmediateUpdater {
    state: Rc<Cell<RenderState>>,

    /// Track if we need to render
    needs_render: Rc<Cell<bool>>,

    /// Track if we need to fetch data
    needs_data_fetch: Rc<Cell<bool>>,

    /// Track if we need to rebuild pipelines
    needs_pipeline_rebuild: Rc<Cell<bool>>,
}

impl ImmediateUpdater {
    /// Create a new immediate updater
    pub fn new() -> Self {
        Self {
            state: Rc::new(Cell::new(RenderState::Ready)),
            needs_render: Rc::new(Cell::new(false)),
            needs_data_fetch: Rc::new(Cell::new(false)),
            needs_pipeline_rebuild: Rc::new(Cell::new(false)),
        }
    }

    /// Get current state
    pub fn get_state(&self) -> RenderState {
        self.state.get()
    }

    /// Check if ready for updates
    pub fn is_ready(&self) -> bool {
        matches!(self.state.get(), RenderState::Ready)
    }

    /// Set busy state
    pub fn set_busy(&self, reason: BusyReason) {
        self.state.set(RenderState::Busy(reason));
    }

    /// Set ready state
    pub fn set_ready(&self) {
        self.state.set(RenderState::Ready);
    }

    /// Process an update event and determine required actions
    pub fn process_update(&self, event: UpdateEvent) -> UpdateAction {
        match event {
            UpdateEvent::DataChanged => {
                self.needs_data_fetch.set(true);
                self.needs_render.set(true);
                UpdateAction::FetchAndRender
            }

            UpdateEvent::ViewChanged { .. } => {
                self.needs_render.set(true);
                UpdateAction::RenderOnly
            }

            UpdateEvent::ConfigChanged => {
                self.needs_pipeline_rebuild.set(true);
                self.needs_render.set(true);
                UpdateAction::RebuildAndRender
            }

            UpdateEvent::Resized(_, _) => {
                self.needs_pipeline_rebuild.set(true);
                self.needs_render.set(true);
                UpdateAction::RebuildAndRender
            }

            UpdateEvent::MetricVisibilityChanged => {
                self.needs_render.set(true);
                UpdateAction::RenderOnly
            }
        }
    }

    /// Check if rendering is needed
    pub fn needs_render(&self) -> bool {
        self.needs_render.get()
    }

    /// Clear render flag
    pub fn clear_render_flag(&self) {
        self.needs_render.set(false);
    }

    /// Check if data fetch is needed
    pub fn needs_data_fetch(&self) -> bool {
        self.needs_data_fetch.get()
    }

    /// Clear data fetch flag
    pub fn clear_data_fetch_flag(&self) {
        self.needs_data_fetch.set(false);
    }

    /// Check if pipeline rebuild is needed
    pub fn needs_pipeline_rebuild(&self) -> bool {
        self.needs_pipeline_rebuild.get()
    }

    /// Clear pipeline rebuild flag
    pub fn clear_pipeline_rebuild_flag(&self) {
        self.needs_pipeline_rebuild.set(false);
    }
}

/// Action to take based on update event
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateAction {
    /// Only render
    RenderOnly,

    /// Fetch data then render
    FetchAndRender,

    /// Rebuild pipelines then render
    RebuildAndRender,
}

impl Default for ImmediateUpdater {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_processing() {
        let updater = ImmediateUpdater::new();

        // Test data change
        let action = updater.process_update(UpdateEvent::DataChanged);
        assert_eq!(action, UpdateAction::FetchAndRender);
        assert!(updater.needs_data_fetch());
        assert!(updater.needs_render());

        // Clear flags
        updater.clear_data_fetch_flag();
        updater.clear_render_flag();

        // Test view change
        let action = updater.process_update(UpdateEvent::ViewChanged {
            zoom: true,
            pan: false,
        });
        assert_eq!(action, UpdateAction::RenderOnly);
        assert!(!updater.needs_data_fetch());
        assert!(updater.needs_render());
    }

    #[test]
    fn test_state_transitions() {
        let updater = ImmediateUpdater::new();

        assert_eq!(updater.get_state(), RenderState::Ready);
        assert!(updater.is_ready());

        updater.set_busy(BusyReason::Rendering);
        assert_eq!(
            updater.get_state(),
            RenderState::Busy(BusyReason::Rendering)
        );
        assert!(!updater.is_ready());

        updater.set_ready();
        assert_eq!(updater.get_state(), RenderState::Ready);
        assert!(updater.is_ready());
    }
}
