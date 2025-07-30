//! Unified state management system for GPU Charts
//!
//! This module provides a centralized state management system that tracks
//! all application state and efficiently detects changes.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Unique identifier for state sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StateSection {
    /// Data-related state (time range, symbol, data values)
    Data,
    /// View state (zoom, pan, viewport)
    View,
    /// Configuration (presets, quality settings)
    Config,
    /// GPU resources state
    GPU,
    /// UI state (selected metrics, visibility)
    UI,
}

/// Represents a change in the state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDiff {
    /// Which sections changed
    pub changed_sections: HashSet<StateSection>,
    /// Generation delta (how many updates happened)
    pub generation_delta: u64,
    /// Detailed changes per section
    pub section_changes: HashMap<StateSection, SectionChange>,
}

/// Detailed change information for a state section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SectionChange {
    /// Data section changes
    Data {
        symbol_changed: bool,
        time_range_changed: bool,
        data_updated: bool,
    },
    /// View section changes
    View {
        zoom_changed: bool,
        pan_changed: bool,
        viewport_resized: bool,
    },
    /// Config section changes
    Config {
        preset_changed: bool,
        quality_changed: bool,
        chart_type_changed: bool,
    },
    /// GPU section changes
    GPU {
        buffers_updated: bool,
        pipelines_rebuilt: bool,
        surface_reconfigured: bool,
    },
    /// UI section changes
    UI {
        metrics_toggled: bool,
        theme_changed: bool,
        layout_changed: bool,
    },
}

/// The unified state container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedState {
    /// Current generation number (increments with each change)
    pub generation: u64,
    /// State sections
    pub sections: HashMap<StateSection, SectionState>,
    /// History of recent changes for debugging
    #[serde(skip)]
    pub change_history: Vec<StateDiff>,
    /// Maximum history size
    #[serde(skip)]
    pub max_history_size: usize,
}

/// State for a specific section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionState {
    /// Section-specific generation
    pub generation: u64,
    /// Actual state data
    pub data: StateData,
}

/// Actual state data for each section
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StateData {
    /// Data state
    Data {
        symbol: String,
        start_time: i64,
        end_time: i64,
        timeframe: u32,
        data_version: u64,
    },
    /// View state
    View {
        zoom_level: f32,
        pan_offset: f32,
        viewport_width: u32,
        viewport_height: u32,
    },
    /// Config state
    Config {
        preset_name: String,
        quality_level: QualityLevel,
        chart_type: String,
        show_grid: bool,
    },
    /// GPU state
    GPU {
        buffers_valid: bool,
        pipelines_valid: bool,
        surface_valid: bool,
        last_render_time: u64,
    },
    /// UI state
    UI {
        visible_metrics: Vec<String>,
        theme: String,
        layout_mode: String,
    },
}

/// Quality level settings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityLevel {
    Low,
    Medium,
    High,
    Ultra,
}

impl Default for UnifiedState {
    fn default() -> Self {
        let mut sections = HashMap::new();

        // Initialize default sections
        sections.insert(
            StateSection::Data,
            SectionState {
                generation: 0,
                data: StateData::Data {
                    symbol: "BTC-USD".to_string(),
                    start_time: 0,
                    end_time: 0,
                    timeframe: 60,
                    data_version: 0,
                },
            },
        );

        sections.insert(
            StateSection::View,
            SectionState {
                generation: 0,
                data: StateData::View {
                    zoom_level: 1.0,
                    pan_offset: 0.0,
                    viewport_width: 800,
                    viewport_height: 600,
                },
            },
        );

        sections.insert(
            StateSection::Config,
            SectionState {
                generation: 0,
                data: StateData::Config {
                    preset_name: "default".to_string(),
                    quality_level: QualityLevel::Medium,
                    chart_type: "line".to_string(),
                    show_grid: true,
                },
            },
        );

        sections.insert(
            StateSection::GPU,
            SectionState {
                generation: 0,
                data: StateData::GPU {
                    buffers_valid: false,
                    pipelines_valid: false,
                    surface_valid: false,
                    last_render_time: 0,
                },
            },
        );

        sections.insert(
            StateSection::UI,
            SectionState {
                generation: 0,
                data: StateData::UI {
                    visible_metrics: vec!["price".to_string()],
                    theme: "dark".to_string(),
                    layout_mode: "default".to_string(),
                },
            },
        );

        Self {
            generation: 0,
            sections,
            change_history: Vec::new(),
            max_history_size: 100,
        }
    }
}

impl UnifiedState {
    /// Create a new unified state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update a section with new data
    pub fn update_section(&mut self, section: StateSection, data: StateData) -> StateDiff {
        let old_generation = self.generation;
        self.generation += 1;

        // Determine what changed
        let section_change = self.detect_section_changes(section, &data);

        // Update the section
        if let Some(section_state) = self.sections.get_mut(&section) {
            section_state.generation = self.generation;
            section_state.data = data;
        }

        // Create diff
        let mut changed_sections = HashSet::new();
        changed_sections.insert(section);

        let mut section_changes = HashMap::new();
        section_changes.insert(section, section_change);

        let diff = StateDiff {
            changed_sections,
            generation_delta: self.generation - old_generation,
            section_changes,
        };

        // Add to history
        self.add_to_history(diff.clone());

        diff
    }

    /// Batch update multiple sections
    pub fn batch_update(&mut self, updates: Vec<(StateSection, StateData)>) -> StateDiff {
        let old_generation = self.generation;
        self.generation += 1;

        let mut changed_sections = HashSet::new();
        let mut section_changes = HashMap::new();

        for (section, data) in updates {
            let section_change = self.detect_section_changes(section, &data);

            if let Some(section_state) = self.sections.get_mut(&section) {
                section_state.generation = self.generation;
                section_state.data = data;
            }

            changed_sections.insert(section);
            section_changes.insert(section, section_change);
        }

        let diff = StateDiff {
            changed_sections,
            generation_delta: self.generation - old_generation,
            section_changes,
        };

        self.add_to_history(diff.clone());

        diff
    }

    /// Get a section's current state
    pub fn get_section(&self, section: StateSection) -> Option<&SectionState> {
        self.sections.get(&section)
    }

    /// Check if a section has changed since a given generation
    pub fn has_changed_since(&self, section: StateSection, generation: u64) -> bool {
        self.sections
            .get(&section)
            .map(|s| s.generation > generation)
            .unwrap_or(false)
    }

    /// Get all sections that changed since a given generation
    pub fn get_changes_since(&self, generation: u64) -> Vec<StateSection> {
        self.sections
            .iter()
            .filter(|(_, state)| state.generation > generation)
            .map(|(section, _)| *section)
            .collect()
    }

    /// Detect what changed in a section
    fn detect_section_changes(&self, section: StateSection, new_data: &StateData) -> SectionChange {
        let current = self.sections.get(&section).map(|s| &s.data);

        match (section, current, new_data) {
            (
                StateSection::Data,
                Some(StateData::Data {
                    symbol: old_symbol,
                    start_time: old_start,
                    end_time: old_end,
                    ..
                }),
                StateData::Data {
                    symbol: new_symbol,
                    start_time: new_start,
                    end_time: new_end,
                    ..
                },
            ) => SectionChange::Data {
                symbol_changed: old_symbol != new_symbol,
                time_range_changed: old_start != new_start || old_end != new_end,
                data_updated: true,
            },
            (
                StateSection::View,
                Some(StateData::View {
                    zoom_level: old_zoom,
                    pan_offset: old_pan,
                    viewport_width: old_w,
                    viewport_height: old_h,
                }),
                StateData::View {
                    zoom_level: new_zoom,
                    pan_offset: new_pan,
                    viewport_width: new_w,
                    viewport_height: new_h,
                },
            ) => SectionChange::View {
                zoom_changed: (old_zoom - new_zoom).abs() > f32::EPSILON,
                pan_changed: (old_pan - new_pan).abs() > f32::EPSILON,
                viewport_resized: old_w != new_w || old_h != new_h,
            },
            (
                StateSection::Config,
                Some(StateData::Config {
                    preset_name: old_preset,
                    quality_level: old_quality,
                    chart_type: old_type,
                    ..
                }),
                StateData::Config {
                    preset_name: new_preset,
                    quality_level: new_quality,
                    chart_type: new_type,
                    ..
                },
            ) => SectionChange::Config {
                preset_changed: old_preset != new_preset,
                quality_changed: old_quality != new_quality,
                chart_type_changed: old_type != new_type,
            },
            _ => {
                // Default changes for other cases
                match section {
                    StateSection::Data => SectionChange::Data {
                        symbol_changed: true,
                        time_range_changed: true,
                        data_updated: true,
                    },
                    StateSection::View => SectionChange::View {
                        zoom_changed: true,
                        pan_changed: true,
                        viewport_resized: true,
                    },
                    StateSection::Config => SectionChange::Config {
                        preset_changed: true,
                        quality_changed: true,
                        chart_type_changed: true,
                    },
                    StateSection::GPU => SectionChange::GPU {
                        buffers_updated: true,
                        pipelines_rebuilt: true,
                        surface_reconfigured: true,
                    },
                    StateSection::UI => SectionChange::UI {
                        metrics_toggled: true,
                        theme_changed: true,
                        layout_changed: true,
                    },
                }
            }
        }
    }

    /// Add a diff to history
    fn add_to_history(&mut self, diff: StateDiff) {
        self.change_history.push(diff);

        // Trim history if too large
        if self.change_history.len() > self.max_history_size {
            self.change_history.remove(0);
        }
    }

    /// Get recent change history
    pub fn get_history(&self, count: usize) -> &[StateDiff] {
        let start = self.change_history.len().saturating_sub(count);
        &self.change_history[start..]
    }

    /// Clear change history
    pub fn clear_history(&mut self) {
        self.change_history.clear();
    }
}

/// Helper to determine what actions are needed based on state changes
#[derive(Debug, Clone)]
pub struct StateChangeActions {
    pub needs_data_fetch: bool,
    pub needs_preprocessing: bool,
    pub needs_render: bool,
    pub needs_pipeline_rebuild: bool,
}

impl StateDiff {
    /// Determine what actions are needed based on the changes
    pub fn get_required_actions(&self) -> StateChangeActions {
        let mut actions = StateChangeActions {
            needs_data_fetch: false,
            needs_preprocessing: false,
            needs_render: false,
            needs_pipeline_rebuild: false,
        };

        for (section, change) in &self.section_changes {
            match (section, change) {
                (
                    StateSection::Data,
                    SectionChange::Data {
                        symbol_changed,
                        time_range_changed,
                        ..
                    },
                ) => {
                    if *symbol_changed || *time_range_changed {
                        actions.needs_data_fetch = true;
                        actions.needs_preprocessing = true;
                        actions.needs_render = true;
                    }
                }
                (
                    StateSection::View,
                    SectionChange::View {
                        zoom_changed,
                        pan_changed,
                        viewport_resized,
                    },
                ) => {
                    if *zoom_changed || *pan_changed || *viewport_resized {
                        actions.needs_render = true;
                    }
                    if *viewport_resized {
                        actions.needs_pipeline_rebuild = true;
                    }
                }
                (
                    StateSection::Config,
                    SectionChange::Config {
                        preset_changed,
                        quality_changed,
                        chart_type_changed,
                    },
                ) => {
                    if *preset_changed || *chart_type_changed {
                        actions.needs_pipeline_rebuild = true;
                        actions.needs_render = true;
                    }
                    if *quality_changed {
                        actions.needs_render = true;
                    }
                }
                (
                    StateSection::UI,
                    SectionChange::UI {
                        metrics_toggled, ..
                    },
                ) => {
                    if *metrics_toggled {
                        actions.needs_render = true;
                    }
                }
                _ => {}
            }
        }

        actions
    }
}
