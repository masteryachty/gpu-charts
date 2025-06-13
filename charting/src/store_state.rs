// Store state structures for React-Rust WASM integration
// These structs mirror the TypeScript interfaces in web/src/types/store.ts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum time range in seconds (30 days)
pub const MAX_TIME_RANGE_SECONDS: u64 = 86400 * 30;

/// Minimum time range in seconds (1 minute)
pub const MIN_TIME_RANGE_SECONDS: u64 = 60;

/// Valid timeframe values
pub const VALID_TIMEFRAMES: &[&str] = &["1m", "5m", "15m", "1h", "4h", "1d"];

/// Valid data columns
pub const VALID_COLUMNS: &[&str] = &["time", "best_bid", "best_ask", "price", "volume", "side"];

/// Complete store state from React Zustand store
/// This matches the TypeScript StoreState interface exactly
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreState {
    /// Current active trading symbol
    pub current_symbol: String,

    /// Chart configuration containing all rendering parameters  
    pub chart_config: ChartConfig,

    /// Market data keyed by symbol
    pub market_data: HashMap<String, MarketData>,

    /// Connection status to data server
    pub is_connected: bool,

    /// Optional user information
    pub user: Option<User>,
}

/// Chart configuration with validation constraints
/// Matches the TypeScript ValidatedChartConfig interface
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartConfig {
    /// Trading symbol (must be non-empty)
    pub symbol: String,

    /// Timeframe (must be valid timeframe)
    pub timeframe: String,

    /// Start time as Unix timestamp (must be < end_time)
    pub start_time: u64,

    /// End time as Unix timestamp (must be > start_time)
    pub end_time: u64,

    /// Array of indicator names
    pub indicators: Vec<String>,
}

/// Market data structure
/// Matches the TypeScript MarketData interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    pub change: f64,
    #[serde(rename = "changePercent")]
    pub change_percent: f64,
    pub volume: f64,
    pub timestamp: u64,
}

/// User information structure
/// Matches the TypeScript User interface
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub plan: UserPlan,
}

/// User plan enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserPlan {
    Free,
    Pro,
    Enterprise,
}

/// Data fetching parameters extracted from store state
/// Used to determine if new data needs to be fetched
#[derive(Debug, Clone, PartialEq)]
pub struct DataFetchParams {
    pub symbol: String,
    pub start_time: u64,
    pub end_time: u64,
    pub columns: Vec<String>,
}

/// Store validation result
/// Used to communicate validation errors
#[derive(Debug, Clone)]
pub struct StoreValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Detailed change detection result
/// Provides fine-grained information about what changed in the store state
#[derive(Debug, Clone, PartialEq)]
pub struct StateChangeDetection {
    pub has_changes: bool,
    pub symbol_changed: bool,
    pub time_range_changed: bool,
    pub timeframe_changed: bool,
    pub indicators_changed: bool,
    pub connection_changed: bool,
    pub user_changed: bool,
    pub market_data_changed: bool,
    pub requires_data_fetch: bool,
    pub requires_render: bool,
    pub change_summary: Vec<String>,
}

/// Types of changes that can occur
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    SymbolChange {
        from: String,
        to: String,
    },
    TimeRangeChange {
        from: (u64, u64),
        to: (u64, u64),
    },
    TimeframeChange {
        from: String,
        to: String,
    },
    IndicatorsChange {
        added: Vec<String>,
        removed: Vec<String>,
    },
    ConnectionStatusChange {
        from: bool,
        to: bool,
    },
    UserChange {
        added: bool,
        removed: bool,
    },
    MarketDataChange {
        symbols_updated: Vec<String>,
    },
}

/// Change detection configuration
/// Allows customizing what changes trigger what actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeDetectionConfig {
    pub enable_symbol_change_detection: bool,
    pub enable_time_range_change_detection: bool,
    pub enable_timeframe_change_detection: bool,
    pub enable_indicator_change_detection: bool,
    pub symbol_change_triggers_fetch: bool,
    pub time_range_change_triggers_fetch: bool,
    pub timeframe_change_triggers_render: bool,
    pub indicator_change_triggers_render: bool,
    pub minimum_time_range_change_seconds: u64,
}

impl Default for ChangeDetectionConfig {
    fn default() -> Self {
        Self {
            enable_symbol_change_detection: true,
            enable_time_range_change_detection: true,
            enable_timeframe_change_detection: true,
            enable_indicator_change_detection: true,
            symbol_change_triggers_fetch: true,
            time_range_change_triggers_fetch: true,
            timeframe_change_triggers_render: true,
            indicator_change_triggers_render: true,
            minimum_time_range_change_seconds: 60, // Only consider changes >= 1 minute significant
        }
    }
}

impl StoreState {
    /// Validate the store state structure and data
    pub fn validate(&self) -> StoreValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate current symbol
        if self.current_symbol.is_empty() {
            errors.push("Current symbol cannot be empty".to_string());
        }

        // Validate chart config
        let config_validation = self.chart_config.validate();
        errors.extend(config_validation.errors);
        warnings.extend(config_validation.warnings);

        // Check consistency between current_symbol and chart_config.symbol
        if self.current_symbol != self.chart_config.symbol {
            warnings.push(format!(
                "Current symbol '{}' differs from chart config symbol '{}'",
                self.current_symbol, self.chart_config.symbol
            ));
        }

        StoreValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Extract data fetching parameters from the store state
    pub fn extract_fetch_params(&self) -> DataFetchParams {
        DataFetchParams {
            symbol: self.chart_config.symbol.clone(),
            start_time: self.chart_config.start_time,
            end_time: self.chart_config.end_time,
            columns: vec!["time".to_string(), "best_bid".to_string()], // Default columns
        }
    }

    /// Advanced change detection with detailed analysis
    pub fn detect_changes_from(
        &self,
        previous: &StoreState,
        config: &ChangeDetectionConfig,
    ) -> StateChangeDetection {
        let mut detection = StateChangeDetection {
            has_changes: false,
            symbol_changed: false,
            time_range_changed: false,
            timeframe_changed: false,
            indicators_changed: false,
            connection_changed: false,
            user_changed: false,
            market_data_changed: false,
            requires_data_fetch: false,
            requires_render: false,
            change_summary: Vec::new(),
        };

        // Symbol change detection
        if config.enable_symbol_change_detection
            && self.chart_config.symbol != previous.chart_config.symbol
        {
            detection.symbol_changed = true;
            detection.has_changes = true;
            if config.symbol_change_triggers_fetch {
                detection.requires_data_fetch = true;
            }
            detection.change_summary.push(format!(
                "Symbol: {} → {}",
                previous.chart_config.symbol, self.chart_config.symbol
            ));
        }

        // Time range change detection
        if config.enable_time_range_change_detection {
            let time_diff = (self.chart_config.start_time as i64
                - previous.chart_config.start_time as i64)
                .unsigned_abs()
                + (self.chart_config.end_time as i64 - previous.chart_config.end_time as i64)
                    .unsigned_abs();

            if time_diff >= config.minimum_time_range_change_seconds {
                detection.time_range_changed = true;
                detection.has_changes = true;
                if config.time_range_change_triggers_fetch {
                    detection.requires_data_fetch = true;
                }
                detection.change_summary.push(format!(
                    "Time range: [{}, {}] → [{}, {}]",
                    previous.chart_config.start_time,
                    previous.chart_config.end_time,
                    self.chart_config.start_time,
                    self.chart_config.end_time
                ));
            }
        }

        // Timeframe change detection
        if config.enable_timeframe_change_detection
            && self.chart_config.timeframe != previous.chart_config.timeframe
        {
            detection.timeframe_changed = true;
            detection.has_changes = true;
            if config.timeframe_change_triggers_render {
                detection.requires_render = true;
            }
            detection.change_summary.push(format!(
                "Timeframe: {} → {}",
                previous.chart_config.timeframe, self.chart_config.timeframe
            ));
        }

        // Indicator change detection
        if config.enable_indicator_change_detection {
            let previous_indicators: std::collections::HashSet<_> =
                previous.chart_config.indicators.iter().collect();
            let current_indicators: std::collections::HashSet<_> =
                self.chart_config.indicators.iter().collect();

            if previous_indicators != current_indicators {
                detection.indicators_changed = true;
                detection.has_changes = true;
                if config.indicator_change_triggers_render {
                    detection.requires_render = true;
                }

                let added: Vec<_> = current_indicators
                    .difference(&previous_indicators)
                    .map(|s| s.to_string())
                    .collect();
                let removed: Vec<_> = previous_indicators
                    .difference(&current_indicators)
                    .map(|s| s.to_string())
                    .collect();

                if !added.is_empty() {
                    detection
                        .change_summary
                        .push(format!("Indicators added: {:?}", added));
                }
                if !removed.is_empty() {
                    detection
                        .change_summary
                        .push(format!("Indicators removed: {:?}", removed));
                }
            }
        }

        // Connection status change
        if self.is_connected != previous.is_connected {
            detection.connection_changed = true;
            detection.has_changes = true;
            detection.change_summary.push(format!(
                "Connection: {} → {}",
                previous.is_connected, self.is_connected
            ));
        }

        // User change detection
        let user_changed = match (&previous.user, &self.user) {
            (None, Some(_)) => {
                detection.change_summary.push("User logged in".to_string());
                true
            }
            (Some(_), None) => {
                detection.change_summary.push("User logged out".to_string());
                true
            }
            (Some(prev), Some(curr)) => {
                if prev != curr {
                    detection
                        .change_summary
                        .push("User information updated".to_string());
                    true
                } else {
                    false
                }
            }
            (None, None) => false,
        };

        if user_changed {
            detection.user_changed = true;
            detection.has_changes = true;
        }

        // Market data change detection (simplified - check if different symbols are present)
        let prev_symbols: std::collections::HashSet<_> = previous.market_data.keys().collect();
        let curr_symbols: std::collections::HashSet<_> = self.market_data.keys().collect();

        if prev_symbols != curr_symbols || !self.market_data.is_empty() {
            detection.market_data_changed = true;
            detection.has_changes = true;
            detection.requires_render = true;

            let new_symbols: Vec<_> = curr_symbols
                .difference(&prev_symbols)
                .map(|s| s.to_string())
                .collect();
            if !new_symbols.is_empty() {
                detection
                    .change_summary
                    .push(format!("Market data updated for: {:?}", new_symbols));
            }
        }

        detection
    }

    /// Simple change detection (backward compatibility)
    pub fn differs_from(&self, other: &StoreState) -> bool {
        let config = ChangeDetectionConfig::default();
        let detection = self.detect_changes_from(other, &config);
        detection.has_changes
    }

    /// Get list of change types with detailed information
    pub fn get_change_types_from(&self, previous: &StoreState) -> Vec<ChangeType> {
        let mut changes = Vec::new();

        // Symbol change
        if self.chart_config.symbol != previous.chart_config.symbol {
            changes.push(ChangeType::SymbolChange {
                from: previous.chart_config.symbol.clone(),
                to: self.chart_config.symbol.clone(),
            });
        }

        // Time range change
        if self.chart_config.start_time != previous.chart_config.start_time
            || self.chart_config.end_time != previous.chart_config.end_time
        {
            changes.push(ChangeType::TimeRangeChange {
                from: (
                    previous.chart_config.start_time,
                    previous.chart_config.end_time,
                ),
                to: (self.chart_config.start_time, self.chart_config.end_time),
            });
        }

        // Timeframe change
        if self.chart_config.timeframe != previous.chart_config.timeframe {
            changes.push(ChangeType::TimeframeChange {
                from: previous.chart_config.timeframe.clone(),
                to: self.chart_config.timeframe.clone(),
            });
        }

        // Indicator changes
        let prev_indicators: std::collections::HashSet<_> =
            previous.chart_config.indicators.iter().collect();
        let curr_indicators: std::collections::HashSet<_> =
            self.chart_config.indicators.iter().collect();

        if prev_indicators != curr_indicators {
            let added: Vec<String> = curr_indicators
                .difference(&prev_indicators)
                .map(|s| s.to_string())
                .collect();
            let removed: Vec<String> = prev_indicators
                .difference(&curr_indicators)
                .map(|s| s.to_string())
                .collect();

            changes.push(ChangeType::IndicatorsChange { added, removed });
        }

        // Connection status change
        if self.is_connected != previous.is_connected {
            changes.push(ChangeType::ConnectionStatusChange {
                from: previous.is_connected,
                to: self.is_connected,
            });
        }

        // User change
        match (&previous.user, &self.user) {
            (None, Some(_)) => changes.push(ChangeType::UserChange {
                added: true,
                removed: false,
            }),
            (Some(_), None) => changes.push(ChangeType::UserChange {
                added: false,
                removed: true,
            }),
            (Some(prev), Some(curr)) => {
                if prev != curr {
                    changes.push(ChangeType::UserChange {
                        added: false,
                        removed: false,
                    });
                }
            }
            _ => {}
        }

        // Market data changes
        let prev_symbols: std::collections::HashSet<_> = previous.market_data.keys().collect();
        let curr_symbols: std::collections::HashSet<_> = self.market_data.keys().collect();

        if prev_symbols != curr_symbols {
            let updated_symbols: Vec<String> = curr_symbols
                .union(&prev_symbols)
                .map(|s| s.to_string())
                .collect();
            changes.push(ChangeType::MarketDataChange {
                symbols_updated: updated_symbols,
            });
        }

        changes
    }
}

impl ChartConfig {
    /// Validate the chart configuration
    pub fn validate(&self) -> StoreValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate symbol
        if self.symbol.is_empty() {
            errors.push("Symbol cannot be empty".to_string());
        } else if !self
            .symbol
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            errors.push("Symbol contains invalid characters".to_string());
        }

        // Validate timeframe
        if !VALID_TIMEFRAMES.contains(&self.timeframe.as_str()) {
            errors.push(format!(
                "Invalid timeframe '{}'. Must be one of: {}",
                self.timeframe,
                VALID_TIMEFRAMES.join(", ")
            ));
        }

        // Validate time range
        if self.start_time >= self.end_time {
            errors.push("Start time must be less than end time".to_string());
        } else {
            let time_range = self.end_time - self.start_time;

            if time_range < MIN_TIME_RANGE_SECONDS {
                errors.push(format!(
                    "Time range too small: {} seconds (minimum: {} seconds)",
                    time_range, MIN_TIME_RANGE_SECONDS
                ));
            }

            if time_range > MAX_TIME_RANGE_SECONDS {
                warnings.push(format!(
                    "Time range very large: {} seconds (maximum recommended: {} seconds)",
                    time_range, MAX_TIME_RANGE_SECONDS
                ));
            }
        }

        // Validate indicators
        for indicator in &self.indicators {
            if indicator.is_empty() {
                warnings.push("Empty indicator name found".to_string());
            }
        }

        StoreValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

impl DataFetchParams {
    /// Check if these fetch parameters differ from another set
    /// Used to determine if new data fetching is needed
    pub fn differs_from(&self, other: &DataFetchParams) -> bool {
        self.symbol != other.symbol
            || self.start_time != other.start_time
            || self.end_time != other.end_time
            || self.columns != other.columns
    }

    /// Validate the fetch parameters
    pub fn validate(&self) -> StoreValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate symbol
        if self.symbol.is_empty() {
            errors.push("Symbol cannot be empty for data fetching".to_string());
        }

        // Validate time range
        if self.start_time >= self.end_time {
            errors.push("Start time must be less than end time for data fetching".to_string());
        }

        // Validate columns
        if self.columns.is_empty() {
            errors.push("At least one column must be specified for data fetching".to_string());
        } else {
            for column in &self.columns {
                if !VALID_COLUMNS.contains(&column.as_str()) {
                    warnings.push(format!(
                        "Unknown column '{}'. Valid columns: {}",
                        column,
                        VALID_COLUMNS.join(", ")
                    ));
                }
            }
        }

        StoreValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_valid_store_state() {
        let store_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec!["RSI".to_string()],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let validation = store_state.validate();
        assert!(
            validation.is_valid,
            "Validation errors: {:?}",
            validation.errors
        );
    }

    #[wasm_bindgen_test]
    fn test_invalid_empty_symbol() {
        let store_state = StoreState {
            current_symbol: "".to_string(),
            chart_config: ChartConfig {
                symbol: "".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: false,
            user: None,
        };

        let validation = store_state.validate();
        assert!(!validation.is_valid);
        assert!(validation.errors.len() >= 2); // Both current_symbol and chart_config.symbol should error
    }

    #[wasm_bindgen_test]
    fn test_invalid_time_range() {
        let chart_config = ChartConfig {
            symbol: "BTC-USD".to_string(),
            timeframe: "1h".to_string(),
            start_time: 2000,
            end_time: 1000, // Invalid: start > end
            indicators: vec![],
        };

        let validation = chart_config.validate();
        assert!(!validation.is_valid);
        assert!(validation
            .errors
            .iter()
            .any(|e| e.contains("Start time must be less than end time")));
    }

    #[wasm_bindgen_test]
    fn test_invalid_timeframe() {
        let chart_config = ChartConfig {
            symbol: "BTC-USD".to_string(),
            timeframe: "invalid".to_string(),
            start_time: 1000,
            end_time: 2000,
            indicators: vec![],
        };

        let validation = chart_config.validate();
        assert!(!validation.is_valid);
        assert!(validation
            .errors
            .iter()
            .any(|e| e.contains("Invalid timeframe")));
    }

    #[wasm_bindgen_test]
    fn test_data_fetch_params_differs() {
        let params1 = DataFetchParams {
            symbol: "BTC-USD".to_string(),
            start_time: 1000,
            end_time: 2000,
            columns: vec!["time".to_string(), "best_bid".to_string()],
        };

        let params2 = DataFetchParams {
            symbol: "ETH-USD".to_string(), // Different symbol
            start_time: 1000,
            end_time: 2000,
            columns: vec!["time".to_string(), "best_bid".to_string()],
        };

        let params3 = DataFetchParams {
            symbol: "BTC-USD".to_string(),
            start_time: 1500, // Different time
            end_time: 2000,
            columns: vec!["time".to_string(), "best_bid".to_string()],
        };

        let params4 = params1.clone();

        assert!(params1.differs_from(&params2));
        assert!(params1.differs_from(&params3));
        assert!(!params1.differs_from(&params4));
    }

    #[wasm_bindgen_test]
    fn test_extract_fetch_params() {
        let store_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "ETH-USD".to_string(), // Different from current_symbol
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let fetch_params = store_state.extract_fetch_params();

        // Should use chart_config.symbol, not current_symbol
        assert_eq!(fetch_params.symbol, "ETH-USD");
        assert_eq!(fetch_params.start_time, 1000);
        assert_eq!(fetch_params.end_time, 2000);
        assert!(!fetch_params.columns.is_empty());
    }

    #[wasm_bindgen_test]
    fn test_serialization_round_trip() {
        let store_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec!["RSI".to_string(), "MACD".to_string()],
            },
            market_data: {
                let mut map = HashMap::new();
                map.insert(
                    "BTC-USD".to_string(),
                    MarketData {
                        symbol: "BTC-USD".to_string(),
                        price: 50000.0,
                        change: 1000.0,
                        change_percent: 2.0,
                        volume: 1000000.0,
                        timestamp: 1234567890,
                    },
                );
                map
            },
            is_connected: true,
            user: Some(User {
                id: "user123".to_string(),
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                plan: UserPlan::Pro,
            }),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&store_state).expect("Failed to serialize");
        let deserialized: StoreState = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(store_state.current_symbol, deserialized.current_symbol);
        assert_eq!(
            store_state.chart_config.symbol,
            deserialized.chart_config.symbol
        );
        assert_eq!(store_state.is_connected, deserialized.is_connected);
        assert_eq!(
            store_state.market_data.len(),
            deserialized.market_data.len()
        );
        assert!(store_state.user.is_some());
        assert!(deserialized.user.is_some());
    }

    #[wasm_bindgen_test]
    fn test_bridge_serialization_compatibility() {
        // Test that our Rust structs serialize to JSON that React can consume
        let store_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1734567890,
                end_time: 1734571490,
                indicators: vec!["RSI".to_string(), "MACD".to_string()],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: Some(User {
                id: "user123".to_string(),
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                plan: UserPlan::Pro,
            }),
        };

        // Test JSON serialization for React bridge
        let json = serde_json::to_string(&store_state).expect("Failed to serialize for bridge");

        // Verify camelCase field names for React compatibility
        assert!(json.contains("\"currentSymbol\":\"BTC-USD\""));
        assert!(json.contains("\"chartConfig\""));
        assert!(json.contains("\"startTime\":1734567890"));
        assert!(json.contains("\"endTime\":1734571490"));
        assert!(json.contains("\"marketData\""));
        assert!(json.contains("\"isConnected\":true"));

        // Test round-trip deserialization
        let deserialized: StoreState =
            serde_json::from_str(&json).expect("Failed to deserialize from bridge");
        assert_eq!(store_state.current_symbol, deserialized.current_symbol);
        assert_eq!(
            store_state.chart_config.start_time,
            deserialized.chart_config.start_time
        );
    }

    #[wasm_bindgen_test]
    fn test_bridge_error_handling() {
        // Test invalid JSON handling
        let invalid_json = "{\"invalid\": json}";
        let result: Result<StoreState, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());

        // Test valid JSON but invalid structure
        let invalid_structure = r#"{"someField": "value"}"#;
        let result: Result<StoreState, _> = serde_json::from_str(invalid_structure);
        assert!(result.is_err());

        // Test partial valid structure
        let partial_structure = r#"{"currentSymbol": "BTC-USD"}"#;
        let result: Result<StoreState, _> = serde_json::from_str(partial_structure);
        assert!(result.is_err()); // Should fail validation
    }

    #[wasm_bindgen_test]
    fn test_bridge_validation_integration() {
        // Test that validation works with serialized/deserialized data
        let invalid_store_state_json = r#"{
            "currentSymbol": "",
            "chartConfig": {
                "symbol": "",
                "timeframe": "invalid",
                "startTime": 2000,
                "endTime": 1000,
                "indicators": []
            },
            "marketData": {},
            "isConnected": false
        }"#;

        let store_state: StoreState = serde_json::from_str(invalid_store_state_json)
            .expect("Should parse JSON even if invalid");

        let validation_result = store_state.validate();
        assert!(!validation_result.is_valid);
        assert!(validation_result.errors.len() >= 3); // Empty symbols + invalid timeframe + invalid time range
    }

    #[wasm_bindgen_test]
    fn test_minimal_valid_bridge_payload() {
        // Test the minimal payload that React bridge would send
        let minimal_json = r#"{
            "currentSymbol": "BTC-USD",
            "chartConfig": {
                "symbol": "BTC-USD",
                "timeframe": "1h",
                "startTime": 1000,
                "endTime": 2000,
                "indicators": []
            },
            "marketData": {},
            "isConnected": true
        }"#;

        let store_state: StoreState =
            serde_json::from_str(minimal_json).expect("Should parse minimal JSON");

        let validation_result = store_state.validate();
        assert!(
            validation_result.is_valid,
            "Minimal payload should be valid: {:?}",
            validation_result.errors
        );

        // Verify fields
        assert_eq!(store_state.current_symbol, "BTC-USD");
        assert_eq!(store_state.chart_config.symbol, "BTC-USD");
        assert_eq!(store_state.chart_config.timeframe, "1h");
        assert_eq!(store_state.user, None);
    }

    #[wasm_bindgen_test]
    fn test_smart_change_detection_symbol_change() {
        let initial_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let new_state = StoreState {
            current_symbol: "ETH-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "ETH-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let config = ChangeDetectionConfig::default();
        let detection = new_state.detect_changes_from(&initial_state, &config);

        assert!(detection.has_changes);
        assert!(detection.symbol_changed);
        assert!(!detection.time_range_changed);
        assert!(!detection.timeframe_changed);
        assert!(detection.requires_data_fetch);
        assert!(detection
            .change_summary
            .iter()
            .any(|s| s.contains("BTC-USD") && s.contains("ETH-USD")));
    }

    #[wasm_bindgen_test]
    fn test_smart_change_detection_time_range_change() {
        let initial_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let new_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1100, // Different time range
                end_time: 2100,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let config = ChangeDetectionConfig::default();
        let detection = new_state.detect_changes_from(&initial_state, &config);

        assert!(detection.has_changes);
        assert!(!detection.symbol_changed);
        assert!(detection.time_range_changed);
        assert!(detection.requires_data_fetch);
        assert!(detection
            .change_summary
            .iter()
            .any(|s| s.contains("Time range")));
    }

    #[wasm_bindgen_test]
    fn test_smart_change_detection_indicator_changes() {
        let initial_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec!["RSI".to_string()],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let new_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec!["RSI".to_string(), "MACD".to_string()],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let config = ChangeDetectionConfig::default();
        let detection = new_state.detect_changes_from(&initial_state, &config);

        assert!(detection.has_changes);
        assert!(detection.indicators_changed);
        assert!(detection.requires_render);
        assert!(detection
            .change_summary
            .iter()
            .any(|s| s.contains("Indicators added") && s.contains("MACD")));
    }

    #[wasm_bindgen_test]
    fn test_smart_change_detection_minimal_time_threshold() {
        let initial_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let new_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1030, // Only 30 second difference
                end_time: 2030,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let config = ChangeDetectionConfig::default(); // Default minimum is 60 seconds
        let detection = new_state.detect_changes_from(&initial_state, &config);

        // Should not detect change because difference is less than minimum threshold
        assert!(!detection.has_changes);
        assert!(!detection.time_range_changed);
    }

    #[wasm_bindgen_test]
    fn test_smart_change_detection_configuration() {
        let initial_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let new_state = StoreState {
            current_symbol: "ETH-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "ETH-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec![],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        // Test with symbol change detection disabled
        let mut config = ChangeDetectionConfig::default();
        config.enable_symbol_change_detection = false;

        let detection = new_state.detect_changes_from(&initial_state, &config);

        assert!(!detection.has_changes);
        assert!(!detection.symbol_changed);
        assert!(!detection.requires_data_fetch);
    }

    #[wasm_bindgen_test]
    fn test_change_types_detailed() {
        let initial_state = StoreState {
            current_symbol: "BTC-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "BTC-USD".to_string(),
                timeframe: "1h".to_string(),
                start_time: 1000,
                end_time: 2000,
                indicators: vec!["RSI".to_string()],
            },
            market_data: HashMap::new(),
            is_connected: true,
            user: None,
        };

        let new_state = StoreState {
            current_symbol: "ETH-USD".to_string(),
            chart_config: ChartConfig {
                symbol: "ETH-USD".to_string(),
                timeframe: "4h".to_string(),
                start_time: 1500,
                end_time: 2500,
                indicators: vec!["RSI".to_string(), "MACD".to_string()],
            },
            market_data: HashMap::new(),
            is_connected: false,
            user: Some(User {
                id: "user123".to_string(),
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                plan: UserPlan::Pro,
            }),
        };

        let change_types = new_state.get_change_types_from(&initial_state);

        // Should detect multiple types of changes
        assert!(change_types.len() >= 4);

        // Check specific change types
        assert!(change_types
            .iter()
            .any(|ct| matches!(ct, ChangeType::SymbolChange { .. })));
        assert!(change_types
            .iter()
            .any(|ct| matches!(ct, ChangeType::TimeRangeChange { .. })));
        assert!(change_types
            .iter()
            .any(|ct| matches!(ct, ChangeType::TimeframeChange { .. })));
        assert!(change_types
            .iter()
            .any(|ct| matches!(ct, ChangeType::IndicatorsChange { .. })));
        assert!(change_types
            .iter()
            .any(|ct| matches!(ct, ChangeType::ConnectionStatusChange { .. })));
        assert!(change_types
            .iter()
            .any(|ct| matches!(ct, ChangeType::UserChange { .. })));
    }
}
