//! Safe instance management for Chart instances
//! Replaces unsafe global state with a thread-local storage pattern

use std::cell::RefCell;
use std::collections::HashMap;
use uuid::Uuid;

use crate::controls::canvas_controller::CanvasController;
use crate::line_graph::LineGraph;
use shared_types::store_state::{ChangeDetectionConfig, StoreState};

/// Data requirements for a preset
#[derive(Clone, Debug)]
pub struct PresetDataRequirements {
    /// Map of data_type to set of columns needed
    pub columns_by_type: std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// Map of metric_id to visibility state
    pub visibility_states: std::collections::HashMap<String, bool>,
    /// Map of metric_id to (data_type, column) for quick lookup
    pub metric_mappings: std::collections::HashMap<String, (String, String)>,
}

/// Represents a single chart instance with all its associated state
pub struct ChartInstance {
    pub line_graph: LineGraph,
    pub canvas_controller: CanvasController,
    pub current_store_state: Option<StoreState>,
    pub change_detection_config: ChangeDetectionConfig,
    pub active_preset: Option<String>,
    pub preset_data_requirements: Option<PresetDataRequirements>,
}

// Thread-local storage for chart instances
thread_local! {
    static CHART_INSTANCES: RefCell<HashMap<Uuid, ChartInstance>> = RefCell::new(HashMap::new());
}

/// Manages chart instances safely without global mutable state
pub struct InstanceManager;

impl InstanceManager {
    /// Create a new chart instance and return its ID
    pub fn create_instance(line_graph: LineGraph, canvas_controller: CanvasController) -> Uuid {
        let id = Uuid::new_v4();
        let instance = ChartInstance {
            line_graph,
            canvas_controller,
            current_store_state: None,
            change_detection_config: ChangeDetectionConfig::default(),
            active_preset: None,
            preset_data_requirements: None,
        };

        CHART_INSTANCES.with(|instances| {
            instances.borrow_mut().insert(id, instance);
        });

        id
    }

    /// Get a reference to a chart instance
    pub fn with_instance<F, R>(id: &Uuid, f: F) -> Option<R>
    where
        F: FnOnce(&ChartInstance) -> R,
    {
        CHART_INSTANCES.with(|instances| instances.borrow().get(id).map(f))
    }

    /// Get a mutable reference to a chart instance
    pub fn with_instance_mut<F, R>(id: &Uuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut ChartInstance) -> R,
    {
        CHART_INSTANCES.with(|instances| instances.borrow_mut().get_mut(id).map(f))
    }

    /// Check if an instance exists
    pub fn instance_exists(id: &Uuid) -> bool {
        CHART_INSTANCES.with(|instances| instances.borrow().contains_key(id))
    }

    /// Remove an instance
    pub fn remove_instance(id: &Uuid) -> Option<ChartInstance> {
        CHART_INSTANCES.with(|instances| instances.borrow_mut().remove(id))
    }

    /// Get the number of active instances
    pub fn instance_count() -> usize {
        CHART_INSTANCES.with(|instances| instances.borrow().len())
    }

    /// Clear all instances (useful for cleanup)
    pub fn clear_all() {
        CHART_INSTANCES.with(|instances| {
            instances.borrow_mut().clear();
        });
    }

    /// Temporarily take an instance for async operations
    /// WARNING: You MUST call put_instance after you're done!
    pub fn take_instance(id: &Uuid) -> Option<ChartInstance> {
        CHART_INSTANCES.with(|instances| instances.borrow_mut().remove(id))
    }

    /// Put an instance back after async operations
    pub fn put_instance(id: Uuid, instance: ChartInstance) {
        CHART_INSTANCES.with(|instances| {
            instances.borrow_mut().insert(id, instance);
        });
    }
}
