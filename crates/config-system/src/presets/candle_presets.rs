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

/// Create the candlestick with RSI preset
pub fn create_candlestick_with_rsi_presets() -> ChartPreset {
    candlestick_with_rsi_preset()
}

/// Candlestick chart with RSI indicator preset
fn candlestick_with_rsi_preset() -> ChartPreset {
    ChartPreset {
        name: "Candlestick with RSI".to_string(),
        description: "OHLC candlestick chart with RSI(14) technical indicator".to_string(),
        chart_types: vec![
            // Main candlestick chart
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
            // RSI indicator
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("TRADES".to_string(), "price".to_string())],
                additional_data_columns: None,
                visible: true,
                label: "RSI (14)".to_string(),
                style: RenderStyle {
                    color: Some([0.8, 0.4, 1.0, 1.0]), // Purple color for RSI line
                    color_options: None,
                    size: 2.0, // Line thickness
                },
                compute_op: Some(ComputeOp::Rsi { period: 14 }), // 14-period RSI
            },
        ],
    }
}

/// Create multiple RSI period variants
pub fn create_candlestick_rsi_variants() -> Vec<ChartPreset> {
    vec![
        candlestick_with_rsi_period(9),
        candlestick_with_rsi_period(14), 
        candlestick_with_rsi_period(21),
    ]
}

/// Candlestick with configurable RSI period
fn candlestick_with_rsi_period(period: u32) -> ChartPreset {
    ChartPreset {
        name: format!("Candlestick with RSI({})", period),
        description: format!("OHLC candlestick chart with RSI({}) technical indicator", period),
        chart_types: vec![
            // Main candlestick chart
            RenderPreset {
                render_type: RenderType::Candlestick,
                data_columns: vec![("TRADES".to_string(), "price".to_string())],
                additional_data_columns: None,
                visible: true,
                label: "OHLC".to_string(),
                style: RenderStyle {
                    color: Some([0.0, 0.8, 0.0, 1.0]), 
                    color_options: None,
                    size: 0.8,
                },
                compute_op: None,
            },
            // RSI indicator with custom period
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("TRADES".to_string(), "price".to_string())],
                additional_data_columns: None,
                visible: true,
                label: format!("RSI ({})", period),
                style: RenderStyle {
                    color: Some([0.8, 0.4, 1.0, 1.0]), // Purple color for RSI
                    color_options: None,
                    size: 2.0,
                },
                compute_op: Some(ComputeOp::Rsi { period }),
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
        assert_eq!(preset.chart_types.len(), 1);
        assert_eq!(preset.chart_types[0].render_type, RenderType::Candlestick);
        assert_eq!(preset.chart_types[0].data_columns.len(), 1); // price from TRADES
    }

    #[test]
    fn test_candlestick_with_rsi_preset() {
        let preset = create_candlestick_with_rsi_presets();
        assert_eq!(preset.name, "Candlestick with RSI");
        assert_eq!(preset.chart_types.len(), 2); // Candlestick + RSI
        
        // Check candlestick part
        assert_eq!(preset.chart_types[0].render_type, RenderType::Candlestick);
        assert_eq!(preset.chart_types[0].label, "OHLC");
        assert_eq!(preset.chart_types[0].compute_op, None);
        
        // Check RSI part
        assert_eq!(preset.chart_types[1].render_type, RenderType::Line);
        assert_eq!(preset.chart_types[1].label, "RSI (14)");
        assert_eq!(preset.chart_types[1].compute_op, Some(ComputeOp::Rsi { period: 14 }));
    }

    #[test]
    fn test_candlestick_rsi_variants() {
        let variants = create_candlestick_rsi_variants();
        assert_eq!(variants.len(), 3); // RSI(9), RSI(14), RSI(21)
        
        // Test RSI(9) variant
        assert_eq!(variants[0].name, "Candlestick with RSI(9)");
        assert_eq!(variants[0].chart_types[1].compute_op, Some(ComputeOp::Rsi { period: 9 }));
        
        // Test RSI(14) variant  
        assert_eq!(variants[1].name, "Candlestick with RSI(14)");
        assert_eq!(variants[1].chart_types[1].compute_op, Some(ComputeOp::Rsi { period: 14 }));
        
        // Test RSI(21) variant
        assert_eq!(variants[2].name, "Candlestick with RSI(21)");
        assert_eq!(variants[2].chart_types[1].compute_op, Some(ComputeOp::Rsi { period: 21 }));
    }
}
