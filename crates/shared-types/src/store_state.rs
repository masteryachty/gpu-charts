// Store state structures for React-Rust WASM integration
// These structs mirror the TypeScript interfaces in web/src/types/store.ts

use serde::{Deserialize, Serialize};

/// Maximum time range in seconds (30 days)
pub const MAX_TIME_RANGE_SECONDS: u64 = 86400 * 30;

/// Minimum time range in seconds (1 minute)
pub const MIN_TIME_RANGE_SECONDS: u64 = 60;

/// Complete store state from React Zustand store
/// Simplified to only contain essential state for the new architecture
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreState {
    pub preset: Option<String>,     // Just the preset name
    pub current_symbol: String,
    pub start_time: u64,
    pub end_time: u64,
}



/// Store validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Detailed change detection result
#[derive(Debug, Clone, PartialEq)]
pub struct StateChangeDetection {
    pub has_changes: bool,
    pub symbol_changed: bool,
    pub time_range_changed: bool,
    pub preset_changed: bool,
    pub requires_data_fetch: bool,
    pub requires_render: bool,
    pub change_summary: Vec<String>,
}

/// Types of changes that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Symbol,
    TimeRange,
    Preset,
}

/// Change detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeDetectionConfig {
    pub enable_symbol_change_detection: bool,
    pub enable_time_range_change_detection: bool,
    pub enable_preset_change_detection: bool,
    pub symbol_change_triggers_fetch: bool,
    pub time_range_change_triggers_fetch: bool,
    pub preset_change_triggers_fetch: bool,
    pub minimum_time_range_change_seconds: u64,
}

impl Default for ChangeDetectionConfig {
    fn default() -> Self {
        Self {
            enable_symbol_change_detection: true,
            enable_time_range_change_detection: true,
            enable_preset_change_detection: true,
            symbol_change_triggers_fetch: true,
            time_range_change_triggers_fetch: true,
            preset_change_triggers_fetch: true,
            minimum_time_range_change_seconds: 60,
        }
    }
}

impl StoreState {
    /// Validate the store state structure and data
    pub fn validate(&self) -> StoreValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate time range
        if self.start_time >= self.end_time {
            errors.push(format!(
                "Invalid time range: start {} >= end {}",
                self.start_time, self.end_time
            ));
        }

        let time_range = self.end_time - self.start_time;
        if time_range > MAX_TIME_RANGE_SECONDS {
            errors.push(format!(
                "Time range too large: {} seconds (max: {} seconds)",
                time_range, MAX_TIME_RANGE_SECONDS
            ));
        }

        if time_range < MIN_TIME_RANGE_SECONDS {
            warnings.push(format!(
                "Time range very small: {} seconds (min recommended: {} seconds)",
                time_range, MIN_TIME_RANGE_SECONDS
            ));
        }

        // Validate symbol
        if self.current_symbol.is_empty() {
            errors.push("Symbol cannot be empty".to_string());
        }

        StoreValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Advanced change detection with detailed analysis
    pub fn detect_changes_from(
        &self,
        previous: &StoreState,
        config: &ChangeDetectionConfig,
    ) -> StateChangeDetection {
        let mut change_summary = Vec::new();
        let mut has_changes = false;
        let mut symbol_changed = false;
        let mut time_range_changed = false;
        let mut preset_changed = false;

        // Check symbol changes
        if config.enable_symbol_change_detection && self.current_symbol != previous.current_symbol {
            symbol_changed = true;
            has_changes = true;
            change_summary.push(format!(
                "Symbol changed: {} → {}",
                previous.current_symbol, self.current_symbol
            ));
        }

        // Check time range changes
        if config.enable_time_range_change_detection {
            let time_changed = self.start_time != previous.start_time || self.end_time != previous.end_time;
            if time_changed {
                let time_diff = (self.end_time - self.start_time) as i64 - (previous.end_time - previous.start_time) as i64;
                if time_diff.abs() >= config.minimum_time_range_change_seconds as i64 {
                    time_range_changed = true;
                    has_changes = true;
                    change_summary.push(format!("Time range changed"));
                }
            }
        }

        // Check preset changes
        if config.enable_preset_change_detection && self.preset != previous.preset {
            preset_changed = true;
            has_changes = true;
            change_summary.push(format!(
                "Preset changed: {:?} → {:?}",
                previous.preset, self.preset
            ));
        }

        StateChangeDetection {
            has_changes,
            symbol_changed,
            time_range_changed,
            preset_changed,
            requires_data_fetch: (symbol_changed && config.symbol_change_triggers_fetch)
                || (time_range_changed && config.time_range_change_triggers_fetch)
                || (preset_changed && config.preset_change_triggers_fetch),
            requires_render: has_changes,
            change_summary,
        }
    }

    /// Simple change detection
    pub fn differs_from(&self, other: &StoreState) -> bool {
        self != other
    }

    /// Get list of change types
    pub fn get_change_types_from(&self, previous: &StoreState) -> Vec<ChangeType> {
        let mut changes = Vec::new();

        if self.current_symbol != previous.current_symbol {
            changes.push(ChangeType::Symbol);
        }

        if self.start_time != previous.start_time || self.end_time != previous.end_time {
            changes.push(ChangeType::TimeRange);
        }

        if self.preset != previous.preset {
            changes.push(ChangeType::Preset);
        }

        changes
    }
}
