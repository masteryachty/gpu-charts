//! Candlestick chart preset configurations
//!
//! Presets for candlestick charts with optional volume panels

use crate::{ChartPreset, RenderPreset, RenderStyle, RenderType};

/// Create the candlestick preset
pub fn create_candle_presets() -> ChartPreset {
    candlestick_preset()
}

/// Candlestick chart preset
fn candlestick_preset() -> ChartPreset {
    ChartPreset {
        name: "Candlestick".to_string(),
        description: "OHLC candlestick chart aggregated from trades".to_string(),
        chart_types: vec![RenderPreset {
            render_type: RenderType::Candlestick,
            // The candlestick renderer will aggregate trades data into OHLC
            data_columns: vec![("TRADES".to_string(), "price".to_string())],
            additional_data_columns: None,
            visible: true,
            label: "OHLC".to_string(),
            style: RenderStyle {
                color: Some([0.0, 0.8, 0.0, 1.0]), // Base color (green for up)
                color_options: None,
                size: 0.8, // Body width relative to time interval
            },
            compute_op: None, // OHLC aggregation is done by the renderer itself
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_preset_creation() {
        let preset = create_candle_presets();
        assert_eq!(preset.name, "Candlestick");
    }

    #[test]
    fn test_candlestick_structure() {
        let preset = candlestick_preset();
        assert_eq!(preset.chart_types.len(), 1);
        assert_eq!(preset.chart_types[0].render_type, RenderType::Candlestick);
        assert_eq!(preset.chart_types[0].data_columns.len(), 1); // price from TRADES
    }
}
