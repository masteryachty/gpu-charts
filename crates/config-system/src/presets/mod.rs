//! Preset configurations for different chart types

pub mod candle_presets;
pub mod market_data_presets;

pub use candle_presets::*;
pub use market_data_presets::*;

use crate::ChartPreset;

/// Get all preset groups
pub fn get_all_presets() -> Vec<ChartPreset> {
    let mut presets = vec![
        market_data_presets::create_market_data_presets(),
        candle_presets::create_candle_presets(),
        candle_presets::create_candlestick_with_rsi_presets(),
    ];
    
    // Add RSI variants for different periods
    presets.extend(candle_presets::create_candlestick_rsi_variants());
    
    presets
}
