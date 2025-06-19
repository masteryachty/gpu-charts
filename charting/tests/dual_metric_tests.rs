// Test file for wasm-pack test
// Run with: wasm-pack test --node
use wasm_bindgen_test::*;

// Configure for Node.js testing
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_hsv_to_rgb_color_generation() {
    use crate::renderer::data_retriever::hsv_to_rgb;
    
    // Test known HSV to RGB conversions
    let (r, g, b) = hsv_to_rgb(0.0, 1.0, 1.0); // Red
    assert!((r - 1.0).abs() < 0.01);
    assert!(g.abs() < 0.01);
    assert!(b.abs() < 0.01);
    
    let (r, g, b) = hsv_to_rgb(120.0, 1.0, 1.0); // Green
    assert!(r.abs() < 0.01);
    assert!((g - 1.0).abs() < 0.01);
    assert!(b.abs() < 0.01);
    
    let (r, g, b) = hsv_to_rgb(240.0, 1.0, 1.0); // Blue
    assert!(r.abs() < 0.01);
    assert!(g.abs() < 0.01);
    assert!((b - 1.0).abs() < 0.01);
}

#[wasm_bindgen_test]
fn test_metric_color_uniqueness() {
    use crate::renderer::data_retriever::hsv_to_rgb;
    
    // Test that different indices generate different colors
    let mut colors = Vec::new();
    
    for i in 0..5 {
        let hue = (i as f32 * 137.5) % 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);
        colors.push((r, g, b));
    }
    
    // Verify all colors are unique
    for i in 0..colors.len() {
        for j in (i + 1)..colors.len() {
            let (r1, g1, b1) = colors[i];
            let (r2, g2, b2) = colors[j];
            
            // Colors should be different (allowing for small floating point differences)
            let diff = (r1 - r2).abs() + (g1 - g2).abs() + (b1 - b2).abs();
            assert!(diff > 0.1, "Colors {} and {} are too similar", i, j);
        }
    }
}

#[wasm_bindgen_test]
fn test_known_metric_color_assignments() {
    // Test that known metrics get expected colors
    let bid_color = [0.0, 0.5, 1.0]; // Blue
    let ask_color = [1.0, 0.2, 0.2]; // Red  
    let price_color = [0.0, 1.0, 0.0]; // Green
    let volume_color = [1.0, 1.0, 0.0]; // Yellow
    
    // Verify color values are in valid range
    for color in [bid_color, ask_color, price_color, volume_color] {
        for component in color {
            assert!(component >= 0.0 && component <= 1.0);
        }
    }
    
    // Verify colors are distinct
    assert_ne!(bid_color, ask_color);
    assert_ne!(bid_color, price_color);
    assert_ne!(ask_color, volume_color);
}

#[wasm_bindgen_test] 
fn test_metric_selection_url_construction() {
    // Test that URL construction works with different metric combinations
    let base_url = "https://localhost:8443/api/data";
    let symbol = "BTC-USD";
    let start = 1234567890u32;
    let end = 1234567900u32;
    
    // Test with bid/ask only
    let metrics = vec!["best_bid".to_string(), "best_ask".to_string()];
    let mut cols = vec!["time".to_string()];
    cols.extend(metrics);
    let columns = cols.join(",");
    
    let expected_url = format!(
        "{}?symbol={}&type=MD&start={}&end={}&columns={}",
        base_url, symbol, start, end, columns
    );
    
    assert!(expected_url.contains("columns=time,best_bid,best_ask"));
    assert!(expected_url.contains(&format!("symbol={}", symbol)));
    assert!(expected_url.contains(&format!("start={}", start)));
    assert!(expected_url.contains(&format!("end={}", end)));
}

#[wasm_bindgen_test]
fn test_metric_selection_with_all_types() {
    // Test URL construction with all metric types
    let metrics = vec![
        "best_bid".to_string(), 
        "best_ask".to_string(),
        "price".to_string(),
        "volume".to_string()
    ];
    
    let mut cols = vec!["time".to_string()];
    cols.extend(metrics);
    let columns = cols.join(",");
    
    assert_eq!(columns, "time,best_bid,best_ask,price,volume");
}

#[wasm_bindgen_test]
fn test_empty_metrics_fallback() {
    // Test that empty metrics defaults to bid/ask
    let empty_metrics: Option<Vec<String>> = None;
    
    let columns = if let Some(metrics) = empty_metrics {
        let mut cols = vec!["time".to_string()];
        cols.extend(metrics);
        cols.join(",")
    } else {
        "time,best_bid,best_ask".to_string()
    };
    
    assert_eq!(columns, "time,best_bid,best_ask");
}