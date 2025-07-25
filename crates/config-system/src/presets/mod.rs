//! Preset configurations for different chart types

pub mod market_data_presets;
pub mod candle_presets;

pub use market_data_presets::*;
pub use candle_presets::*;

use crate::ChartPreset;

/// Get all preset groups
pub fn get_all_presets() -> Vec<ChartPreset> {
    vec![market_data_presets::create_market_data_presets(), candle_presets::create_candle_presets()]
}
