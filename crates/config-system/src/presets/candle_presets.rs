//! Candlestick chart preset configurations
//!
//! Presets for candlestick charts with optional volume panels

use crate::{ChartPreset, ComputeOp, RenderPreset, RenderStyle, RenderType};

/// Create the candlestick preset
pub fn create_candle_presets() -> ChartPreset {
    candlestick_preset()
}

/// Candlestick chart preset
fn candlestick_preset() -> ChartPreset {
    ChartPreset {
        name: "Candlestick".to_string(),
        description: "OHLC candlestick chart aggregated from trades".to_string(),
        chart_types: vec![
            RenderPreset {
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
            },
            // EMA 9
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("COMPUTED".to_string(), "ema_9".to_string())],
                additional_data_columns: Some(vec![("TRADES".to_string(), "price".to_string())]),
                visible: false,
                label: "EMA 9".to_string(),
                style: RenderStyle {
                    color: Some([1.0, 0.4, 0.4, 1.0]), // Light red
                    color_options: None,
                    size: 2.0,
                },
                compute_op: Some(ComputeOp::WeightedAverage { weights: vec![] }),
            },
            // EMA 20
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("COMPUTED".to_string(), "ema_20".to_string())],
                additional_data_columns: Some(vec![("TRADES".to_string(), "price".to_string())]),
                visible: false,
                label: "EMA 20".to_string(),
                style: RenderStyle {
                    color: Some([0.4, 1.0, 1.0, 1.0]), // Light teal
                    color_options: None,
                    size: 2.0,
                },
                compute_op: Some(ComputeOp::WeightedAverage { weights: vec![] }),
            },
            // EMA 50
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("COMPUTED".to_string(), "ema_50".to_string())],
                additional_data_columns: Some(vec![("TRADES".to_string(), "price".to_string())]),
                visible: false,
                label: "EMA 50".to_string(),
                style: RenderStyle {
                    color: Some([0.4, 0.4, 1.0, 1.0]), // Light blue
                    color_options: None,
                    size: 2.0,
                },
                compute_op: Some(ComputeOp::WeightedAverage { weights: vec![] }),
            },
            // EMA 100
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("COMPUTED".to_string(), "ema_100".to_string())],
                additional_data_columns: Some(vec![("TRADES".to_string(), "price".to_string())]),
                visible: false,
                label: "EMA 100".to_string(),
                style: RenderStyle {
                    color: Some([0.4, 1.0, 0.4, 1.0]), // Light green
                    color_options: None,
                    size: 2.0,
                },
                compute_op: Some(ComputeOp::WeightedAverage { weights: vec![] }),
            },
            // EMA 200
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("COMPUTED".to_string(), "ema_200".to_string())],
                additional_data_columns: Some(vec![("TRADES".to_string(), "price".to_string())]),
                visible: false,
                label: "EMA 200".to_string(),
                style: RenderStyle {
                    color: Some([1.0, 1.0, 0.4, 1.0]), // Light yellow
                    color_options: None,
                    size: 2.0,
                },
                compute_op: Some(ComputeOp::WeightedAverage { weights: vec![] }),
            },
        ],
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
        assert_eq!(preset.chart_types.len(), 6); // 1 candlestick + 5 EMAs
        assert_eq!(preset.chart_types[0].render_type, RenderType::Candlestick);
        assert_eq!(preset.chart_types[0].data_columns.len(), 1); // price from TRADES
        
        // Check EMA configurations
        for i in 1..6 {
            assert_eq!(preset.chart_types[i].render_type, RenderType::Line);
            assert!(preset.chart_types[i].label.starts_with("EMA"));
            assert_eq!(preset.chart_types[i].visible, false); // All EMAs start hidden
        }
    }
}
