//! Simplified public API for GPU Charts
//!
//! This module provides a clean, intuitive API that hides the complexity
//! of the underlying render pipeline architecture.

use wasm_bindgen::prelude::*;

/// Simplified chart configuration
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct ChartConfig {
    /// Chart type: "line", "candlestick", "bar", "area"
    #[wasm_bindgen(skip)]
    pub chart_type: String,
    /// Symbol to display (e.g., "BTC-USD")
    #[wasm_bindgen(skip)]
    pub symbol: String,
    /// Time range start (Unix timestamp)
    pub start_time: i64,
    /// Time range end (Unix timestamp)  
    pub end_time: i64,
    /// Canvas width
    pub width: u32,
    /// Canvas height
    pub height: u32,
}

#[wasm_bindgen]
impl ChartConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(
        chart_type: String,
        symbol: String,
        start_time: i64,
        end_time: i64,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            chart_type,
            symbol,
            start_time,
            end_time,
            width,
            height,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn chart_type(&self) -> String {
        self.chart_type.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_chart_type(&mut self, chart_type: String) {
        self.chart_type = chart_type;
    }

    #[wasm_bindgen(getter)]
    pub fn symbol(&self) -> String {
        self.symbol.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_symbol(&mut self, symbol: String) {
        self.symbol = symbol;
    }
}

/// Simplified chart instance
#[wasm_bindgen]
pub struct SimpleChart {
    /// Internal chart instance
    chart: crate::Chart,
    /// Current configuration
    config: ChartConfig,
}

#[wasm_bindgen]
impl SimpleChart {
    /// Create a new chart with configuration
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_id: String, config: ChartConfig) -> Result<SimpleChart, JsValue> {
        // Create internal chart
        let mut chart = crate::Chart::new();

        // Initialize with canvas
        chart
            .init(
                &canvas_id,
                config.width,
                config.height,
                config.start_time as u32,
                config.end_time as u32,
            )
            .await?;

        // Apply configuration
        let preset = match config.chart_type.as_str() {
            "line" => "price",
            "candlestick" => "candles",
            "bar" => "volume",
            _ => "price",
        };

        chart.apply_preset_and_symbol(preset, &config.symbol)?;

        Ok(Self { chart, config })
    }

    /// Update chart data
    #[wasm_bindgen]
    pub fn update(&mut self, config: ChartConfig) -> Result<(), JsValue> {
        // Update time range if changed
        if config.start_time != self.config.start_time || config.end_time != self.config.end_time {
            // Update state
            let state = serde_json::json!({
                "ChartStateConfig": {
                    "symbol": config.symbol,
                    "startTime": config.start_time,
                    "endTime": config.end_time,
                }
            });

            self.chart.update_unified_state(&state.to_string())?;
        }

        // Update size if changed
        if config.width != self.config.width || config.height != self.config.height {
            self.chart.resize(config.width, config.height)?;
        }

        self.config = config;
        Ok(())
    }

    /// Render the chart
    #[wasm_bindgen]
    pub async fn render(&self) -> Result<(), JsValue> {
        self.chart.render().await
    }

    /// Enable auto-rendering on data changes
    #[wasm_bindgen]
    pub fn enable_auto_render(&self, _enabled: bool) -> Result<(), JsValue> {
        // This would enable automatic rendering when data changes
        Ok(())
    }

    /// Set quality preset
    #[wasm_bindgen]
    pub fn set_quality(&self, quality: &str) -> Result<(), JsValue> {
        let fps = match quality {
            "high" => 60,
            "medium" => 30,
            "low" => 15,
            _ => 30,
        };

        self.chart.set_frame_rate(fps)
    }

    /// Get performance metrics
    #[wasm_bindgen]
    pub fn get_performance(&self) -> Result<String, JsValue> {
        self.chart.get_frame_stats()
    }

    /// Handle mouse wheel
    #[wasm_bindgen]
    pub fn on_wheel(&self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
        self.chart.handle_mouse_wheel(delta_y, x, y)
    }

    /// Handle mouse move
    #[wasm_bindgen]
    pub fn on_mouse_move(&self, x: f64, y: f64) -> Result<(), JsValue> {
        self.chart.handle_mouse_move(x, y)
    }

    /// Handle mouse click
    #[wasm_bindgen]
    pub fn on_click(&self, x: f64, y: f64, pressed: bool) -> Result<(), JsValue> {
        self.chart.handle_mouse_click(x, y, pressed)
    }
}

/// Factory for creating charts with presets
#[wasm_bindgen]
pub struct ChartFactory;

#[wasm_bindgen]
impl ChartFactory {
    /// Create a line chart
    #[wasm_bindgen]
    pub async fn create_line_chart(
        canvas_id: String,
        symbol: String,
        start_time: i64,
        end_time: i64,
        width: u32,
        height: u32,
    ) -> Result<SimpleChart, JsValue> {
        let config = ChartConfig::new(
            "line".to_string(),
            symbol,
            start_time,
            end_time,
            width,
            height,
        );

        SimpleChart::new(canvas_id, config).await
    }

    /// Create a candlestick chart
    #[wasm_bindgen]
    pub async fn create_candlestick_chart(
        canvas_id: String,
        symbol: String,
        start_time: i64,
        end_time: i64,
        width: u32,
        height: u32,
    ) -> Result<SimpleChart, JsValue> {
        let config = ChartConfig::new(
            "candlestick".to_string(),
            symbol,
            start_time,
            end_time,
            width,
            height,
        );

        SimpleChart::new(canvas_id, config).await
    }
}

/// One-line chart creation
#[wasm_bindgen]
pub async fn create_chart(
    canvas_id: String,
    chart_type: String,
    symbol: String,
    hours_back: u32,
) -> Result<SimpleChart, JsValue> {
    let now = js_sys::Date::now() as i64 / 1000;
    let start = now - (hours_back as i64 * 3600);

    let config = ChartConfig::new(
        chart_type, symbol, start, now, 800, // Default width
        600, // Default height
    );

    SimpleChart::new(canvas_id, config).await
}

/// Batch chart operations
#[wasm_bindgen]
pub struct ChartBatch {
    charts: Vec<SimpleChart>,
}

#[wasm_bindgen]
impl ChartBatch {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { charts: Vec::new() }
    }

    /// Add a chart to the batch
    pub fn add(&mut self, chart: SimpleChart) {
        self.charts.push(chart);
    }

    /// Render all charts
    pub async fn render_all(&self) -> Result<(), JsValue> {
        for chart in &self.charts {
            chart.render().await?;
        }
        Ok(())
    }

    /// Update all charts with new time range
    pub fn update_time_range(&mut self, start_time: i64, end_time: i64) -> Result<(), JsValue> {
        for chart in &mut self.charts {
            let mut config = chart.config.clone();
            config.start_time = start_time;
            config.end_time = end_time;
            chart.update(config)?;
        }
        Ok(())
    }
}

/// Global chart registry for managing multiple charts
#[wasm_bindgen]
pub struct ChartRegistry {
    charts: std::collections::HashMap<String, SimpleChart>,
}

#[wasm_bindgen]
impl ChartRegistry {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            charts: std::collections::HashMap::new(),
        }
    }

    /// Register a chart with an ID
    pub fn register(&mut self, id: String, chart: SimpleChart) {
        self.charts.insert(id, chart);
    }

    /// Check if a chart exists
    pub fn has(&self, id: &str) -> bool {
        self.charts.contains_key(id)
    }

    /// Remove a chart
    pub fn remove(&mut self, id: &str) -> Option<SimpleChart> {
        self.charts.remove(id)
    }

    /// Render all registered charts
    pub async fn render_all(&self) -> Result<(), JsValue> {
        for chart in self.charts.values() {
            chart.render().await?;
        }
        Ok(())
    }
}
