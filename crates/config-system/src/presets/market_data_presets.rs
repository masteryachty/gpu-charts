//! Market data preset configurations
//!
//! Preset for market data visualization with bid/ask lines and trade triangles

use crate::{ChartPreset, ComputeOp, RenderPreset, RenderStyle, RenderType};

/// Create all market data presets
pub fn create_market_data_presets() -> ChartPreset {
    market_data_preset()
}

/// Combined market data preset with bid, ask, trades, and mid price
fn market_data_preset() -> ChartPreset {
    ChartPreset {
        name: "Market Data".to_string(),
        description: "Market data visualization with bid/ask lines and trade markers".to_string(),
        chart_types: vec![
            // Bid line
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("md".to_string(), "best_bid".to_string())],
                additional_data_columns: None,
                visible: true,
                label: "Bid".to_string(),
                style: RenderStyle {
                    color: Some([0.0, 0.8, 0.0, 1.0]), // Green
                    color_options: None,
                    size: 1.0,
                },
                compute_op: None,
            },
            // Ask line
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![("md".to_string(), "best_ask".to_string())],
                additional_data_columns: None,
                visible: true,
                label: "Ask".to_string(),
                style: RenderStyle {
                    color: Some([0.8, 0.0, 0.0, 1.0]), // Red
                    color_options: None,
                    size: 1.0,
                },
                compute_op: None,
            },
            // Trade triangles
            RenderPreset {
                render_type: RenderType::Triangle,
                data_columns: vec![("trades".to_string(), "price".to_string())],
                additional_data_columns: Some(vec![("trades".to_string(), "side".to_string())]),
                visible: true,
                label: "Trades".to_string(),
                style: RenderStyle {
                    color: None, // Use color_options instead
                    color_options: Some(vec![
                        [0.0, 0.6, 0.0, 1.0], // Green for buy
                        [0.6, 0.0, 0.0, 1.0], // Red for sell
                    ]),
                    size: 8.0, // Triangle size in pixels
                },
                compute_op: None,
            },
            // Mid price (calculated from bid/ask)
            RenderPreset {
                render_type: RenderType::Line,
                data_columns: vec![
                    ("md".to_string(), "best_ask".to_string()),
                    ("md".to_string(), "best_bid".to_string()),
                ],
                additional_data_columns: None,
                visible: true, // Now visible by default
                label: "Mid".to_string(),
                style: RenderStyle {
                    color: Some([0.7, 0.7, 1.0, 1.0]), // Light blue
                    color_options: None,
                    size: 1.5,
                },
                compute_op: Some(ComputeOp::Average), // (ask + bid) / 2
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_data_preset_creation() {
        let preset = create_market_data_presets();
        assert_eq!(preset.name, "Market Data");
    }

    #[test]
    fn test_market_data_preset_structure() {
        let preset = market_data_preset();
        assert_eq!(preset.chart_types.len(), 4);

        // Check each component
        assert_eq!(preset.chart_types[0].label, "Bid");
        assert_eq!(preset.chart_types[1].label, "Ask");
        assert_eq!(preset.chart_types[2].label, "Trades");
        assert_eq!(preset.chart_types[3].label, "Mid");

        // Check visibility defaults
        assert!(preset.chart_types[0].visible); // Bid visible
        assert!(preset.chart_types[1].visible); // Ask visible
        assert!(preset.chart_types[2].visible); // Trades visible by default
        assert!(preset.chart_types[3].visible); // Mid visible by default
    }

    #[test]
    fn test_trades_component_has_triangles() {
        let preset = market_data_preset();
        let trades = &preset.chart_types[2];
        assert_eq!(trades.render_type, RenderType::Triangle);
        assert!(trades.style.color_options.is_some());
        assert_eq!(trades.style.color_options.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_mid_component_has_compute_op() {
        let preset = market_data_preset();
        let mid = &preset.chart_types[3];
        assert!(mid.compute_op.is_some());
        match mid.compute_op.as_ref().unwrap() {
            ComputeOp::Average => (),
            _ => panic!("Expected Average compute op for mid price"),
        }
    }
}
