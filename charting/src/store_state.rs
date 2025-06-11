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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub plan: UserPlan,
}

/// User plan enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl ChartConfig {
    /// Validate the chart configuration
    pub fn validate(&self) -> StoreValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Validate symbol
        if self.symbol.is_empty() {
            errors.push("Symbol cannot be empty".to_string());
        } else if !self.symbol.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
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
        assert!(validation.is_valid, "Validation errors: {:?}", validation.errors);
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
        assert!(validation.errors.iter().any(|e| e.contains("Start time must be less than end time")));
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
        assert!(validation.errors.iter().any(|e| e.contains("Invalid timeframe")));
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
                map.insert("BTC-USD".to_string(), MarketData {
                    symbol: "BTC-USD".to_string(),
                    price: 50000.0,
                    change: 1000.0,
                    change_percent: 2.0,
                    volume: 1000000.0,
                    timestamp: 1234567890,
                });
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
        assert_eq!(store_state.chart_config.symbol, deserialized.chart_config.symbol);
        assert_eq!(store_state.is_connected, deserialized.is_connected);
        assert_eq!(store_state.market_data.len(), deserialized.market_data.len());
        assert!(store_state.user.is_some());
        assert!(deserialized.user.is_some());
    }
}