//! Safe instance management for Chart instances
//! Replaces unsafe global state with a thread-local storage pattern
use crate::chart_engine::ChartEngine;
use std::cell::RefCell;
use std::collections::HashMap;
use uuid::Uuid;

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
    pub chart_engine: ChartEngine,
}

// Thread-local storage for chart instances
thread_local! {
    static CHART_INSTANCES: RefCell<HashMap<Uuid, ChartInstance>> = RefCell::new(HashMap::new());
}

/// Manages chart instances safely without global mutable state
pub struct InstanceManager;

impl InstanceManager {
    /// Create a new chart instance and return its ID
    pub async fn create_instance(
        canvas_id: &str,
        width: u32,
        height: u32,
        start_x: u32,
        end_x: u32,
    ) -> Result<Uuid, String> {
        let id = Uuid::new_v4();

        // Initialize the line graph directly with canvas
        let mut chart_engine = ChartEngine::new(width, height, canvas_id, start_x, end_x)
            .await
            .map_err(|e| format!("Failed to create LineGraph: {e:?}"))?;

        // Set the instance ID in the chart engine
        chart_engine.set_instance_id(id);

        let instance = ChartInstance { chart_engine };

        CHART_INSTANCES.with(|instances| {
            instances.borrow_mut().insert(id, instance);
        });

        Ok(id)
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
