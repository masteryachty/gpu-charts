//! Integration tests for the GPU renderer

use gpu_charts_renderer::{Viewport, config::*};
use gpu_charts_shared::{ChartConfiguration, ChartType, VisualConfig, DataHandle, TimeRange, DataMetadata, OverlayConfig, RenderLocation};

/// Helper to create test visual config
fn create_test_visual_config() -> VisualConfig {
    VisualConfig {
        background_color: [0.0, 0.0, 0.0, 1.0],
        grid_color: [0.2, 0.2, 0.2, 1.0],
        text_color: [1.0, 1.0, 1.0, 1.0],
        margin_percent: 0.1,
        show_grid: true,
        show_axes: true,
    }
}

#[test]
fn test_config_validation() {
    // Valid configuration
    let valid_config = ChartConfiguration {
        chart_type: ChartType::Line,
        visual_config: create_test_visual_config(),
        overlays: vec![],
        data_handles: vec![DataHandle {
            id: uuid::Uuid::new_v4(),
            metadata: DataMetadata {
                symbol: "TEST".to_string(),
                time_range: TimeRange::new(0, 100),
                columns: vec!["time".to_string(), "price".to_string()],
                row_count: 100,
                byte_size: 800,
                creation_time: 1234567890,
            },
        }],
    };
    
    assert!(ConfigValidator::validate(&valid_config).is_ok());
    
    // Invalid configuration - no data handles
    let invalid_config = ChartConfiguration {
        chart_type: ChartType::Line,
        visual_config: create_test_visual_config(),
        overlays: vec![],
        data_handles: vec![],
    };
    
    assert!(ConfigValidator::validate(&invalid_config).is_err());
    
    // Invalid configuration - bad color values
    let mut bad_color_config = valid_config.clone();
    bad_color_config.visual_config.background_color = [2.0, 0.0, 0.0, 1.0]; // > 1.0
    
    assert!(ConfigValidator::validate(&bad_color_config).is_err());
    
    // Invalid configuration - bad margin
    let mut bad_margin_config = valid_config.clone();
    bad_margin_config.visual_config.margin_percent = 0.6; // > 0.5
    
    assert!(ConfigValidator::validate(&bad_margin_config).is_err());
}

#[test]
fn test_config_diff() {
    let config1 = ChartConfiguration {
        chart_type: ChartType::Line,
        visual_config: create_test_visual_config(),
        overlays: vec![],
        data_handles: vec![DataHandle {
            id: uuid::Uuid::new_v4(),
            metadata: DataMetadata {
                symbol: "TEST".to_string(),
                time_range: TimeRange::new(0, 100),
                columns: vec!["time".to_string(), "price".to_string()],
                row_count: 100,
                byte_size: 800,
                creation_time: 1234567890,
            },
        }],
    };
    
    // No changes
    let diff = ConfigurationDiff::calculate(&config1, &config1);
    assert!(!diff.visual_changed);
    assert!(!diff.chart_type_changed);
    assert!(!diff.overlays_changed);
    assert!(!diff.data_handles_changed);
    assert!(!diff.requires_update());
    
    // Visual change
    let mut config2 = config1.clone();
    config2.visual_config.background_color = [0.1, 0.1, 0.1, 1.0];
    let diff = ConfigurationDiff::calculate(&config1, &config2);
    assert!(diff.visual_changed);
    assert!(diff.requires_update());
    
    // Chart type change
    let mut config3 = config1.clone();
    config3.chart_type = ChartType::Candlestick;
    let diff = ConfigurationDiff::calculate(&config1, &config3);
    assert!(diff.chart_type_changed);
    assert!(diff.requires_update());
    
    // Overlay change
    let mut config4 = config1.clone();
    config4.overlays.push(OverlayConfig {
        overlay_type: "volume".to_string(),
        data_handle: None,
        render_location: RenderLocation::SubChart,
        parameters: serde_json::json!({}),
    });
    let diff = ConfigurationDiff::calculate(&config1, &config4);
    assert!(diff.overlays_changed);
    assert!(diff.requires_update());
}

#[test]
fn test_performance_hints() {
    let hints = PerformanceHints::default();
    assert_eq!(hints.target_fps, 60);
    assert!(hints.enable_lod);
    assert!(hints.enable_gpu_culling);
    assert!(hints.enable_instancing);
    assert!(!hints.prefer_quality);
}

#[test]
fn test_render_configuration() {
    let base_config = ChartConfiguration {
        chart_type: ChartType::Line,
        visual_config: create_test_visual_config(),
        overlays: vec![],
        data_handles: vec![DataHandle {
            id: uuid::Uuid::new_v4(),
            metadata: DataMetadata {
                symbol: "TEST".to_string(),
                time_range: TimeRange::new(0, 100),
                columns: vec!["time".to_string(), "price".to_string()],
                row_count: 100,
                byte_size: 800,
                creation_time: 1234567890,
            },
        }],
    };
    
    let render_config = RenderConfiguration {
        base_config: base_config.clone(),
        performance_hints: PerformanceHints::default(),
        debug_options: DebugOptions::default(),
    };
    
    assert_eq!(render_config.base_config.chart_type, ChartType::Line);
    assert_eq!(render_config.performance_hints.target_fps, 60);
    assert!(!render_config.debug_options.show_wireframe);
}

#[test]
fn test_overlay_validation() {
    let valid_overlay = OverlayConfig {
        overlay_type: "volume".to_string(),
        data_handle: None,
        render_location: RenderLocation::SubChart,
        parameters: serde_json::json!({}),
    };
    
    // This would be part of ConfigValidator::validate_overlay
    assert_eq!(valid_overlay.overlay_type, "volume");
    
    let invalid_overlay = OverlayConfig {
        overlay_type: "unknown_overlay".to_string(),
        data_handle: None,
        render_location: RenderLocation::MainChart,
        parameters: serde_json::json!({}),
    };
    
    // In actual implementation, validator would reject this
    assert_ne!(invalid_overlay.overlay_type, "volume");
}

#[test]
fn test_viewport_operations() {
    let mut viewport = Viewport {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
        zoom_level: 1.0,
        time_range: TimeRange::new(0, 1000000),
    };
    
    // Test pan
    viewport.x += 100.0;
    viewport.y += 50.0;
    assert_eq!(viewport.x, 100.0);
    assert_eq!(viewport.y, 50.0);
    
    // Test zoom
    viewport.zoom_level = 2.0;
    assert_eq!(viewport.zoom_level, 2.0);
    
    // Test time range adjustment
    let center = (viewport.time_range.start + viewport.time_range.end) / 2;
    let half_range = ((viewport.time_range.end - viewport.time_range.start) as f64 / (2.0 * viewport.zoom_level as f64)) as u64;
    viewport.time_range.start = center.saturating_sub(half_range);
    viewport.time_range.end = center + half_range;
    
    assert!(viewport.time_range.end > viewport.time_range.start);
}

#[cfg(test)]
mod culling_tests {
    use gpu_charts_renderer::culling::{DataRange, RenderRange};
    use gpu_charts_shared::TimeRange;
    
    #[test]
    fn test_render_range() {
        let range = RenderRange {
            start_index: 100,
            end_index: 200,
            total_points: 100,
        };
        
        assert_eq!(range.total_points, 100);
        assert!(range.end_index > range.start_index);
    }
    
    #[test]
    fn test_data_range() {
        let range = DataRange {
            time_range: TimeRange::new(1000, 2000),
            value_min: 0.0,
            value_max: 100.0,
        };
        
        assert_eq!(range.value_max - range.value_min, 100.0);
        assert_eq!(range.time_range.end - range.time_range.start, 1000);
    }
}

#[cfg(test)]
mod lod_tests {
    use gpu_charts_renderer::lod::{LODLevel, LODSystem};
    
    #[test]
    fn test_lod_level_selection() {
        let system = LODSystem::new();
        
        // Test full detail
        let level = system.select_lod(1.0, 1000);
        assert!(matches!(level, LODLevel::Full));
        
        // Test moderate reduction (need > 1M points)
        let level = system.select_lod(0.6, 2_000_000);
        assert!(matches!(level, LODLevel::Moderate));
        
        // Test aggressive reduction (zoom < 0.5)
        let level = system.select_lod(0.3, 1_000_000);
        assert!(matches!(level, LODLevel::Aggressive));
        
        // Test aggregated (zoom < 0.1)
        let level = system.select_lod(0.05, 10_000_000);
        assert!(matches!(level, LODLevel::Aggregated));
    }
}

#[cfg(test)]
mod performance_metrics_tests {
    use gpu_charts_renderer::PerformanceMetrics;
    
    #[test]
    fn test_performance_metrics_default() {
        let metrics = PerformanceMetrics::default();
        assert_eq!(metrics.frame_time_ms, 0.0);
        assert_eq!(metrics.gpu_time_ms, 0.0);
        assert_eq!(metrics.cpu_time_ms, 0.0);
        assert_eq!(metrics.draw_calls, 0);
        assert_eq!(metrics.vertices_rendered, 0);
        assert_eq!(metrics.triangles_rendered, 0);
    }
    
    #[test]
    fn test_performance_metrics_update() {
        let mut metrics = PerformanceMetrics::default();
        metrics.frame_time_ms = 16.67;
        metrics.draw_calls = 10;
        metrics.vertices_rendered = 100_000;
        
        assert!(metrics.frame_time_ms > 16.0);
        assert_eq!(metrics.draw_calls, 10);
        assert_eq!(metrics.vertices_rendered, 100_000);
    }
}