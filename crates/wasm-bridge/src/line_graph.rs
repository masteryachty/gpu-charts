use data_manager::{ChartType, DataStore, DataManager};
use renderer::{RenderEngine, Renderer};
use config_system::GpuChartsConfig;
use chrono::DateTime;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use crate::wrappers::js::get_query_params;
#[cfg(target_arch = "wasm32")]
use js_sys::Error;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

pub struct LineGraph {
    pub data_store: Rc<RefCell<DataStore>>,
    pub renderer: Renderer,
    pub data_manager: DataManager,
}

impl LineGraph {
    #[cfg(target_arch = "wasm32")]
    pub async fn new(
        width: u32,
        height: u32,
        canvas: HtmlCanvasElement,
    ) -> Result<LineGraph, Error> {
        let params = get_query_params();

        // Handle missing query parameters gracefully (for React integration)
        let topic = params
            .get("topic")
            .unwrap_or(&"default_topic".to_string())
            .clone();
        let start = params
            .get("start")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                // Default to last hour if no start time provided
                chrono::Utc::now().timestamp() - 3600
            });
        let end = params
            .get("end")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                // Default to current time if no end time provided
                chrono::Utc::now().timestamp()
            });

        // Create DataStore
        let ds = DataStore::new(width, height);
        let data_store = Rc::new(RefCell::new(ds));
        data_store.borrow_mut().topic = Some(topic.clone());

        // Create RenderEngine
        let engine_result = RenderEngine::new(canvas, data_store.clone()).await;
        let engine = Rc::new(RefCell::new(engine_result.unwrap()));

        // Get device and queue for DataManager
        let (device, queue) = {
            let engine_b = engine.borrow();
            (Arc::new(engine_b.device.clone()), Arc::new(engine_b.queue.clone()))
        };

        // Create DataManager with modular approach
        let mut data_manager = DataManager::new(
            device.clone(),
            queue.clone(),
            "https://api.rednax.io".to_string(),
        );

        // Create config
        let config = GpuChartsConfig::default();

        // Create Renderer with modular approach
        let renderer = Renderer::new(
            engine.clone(),
            config,
            data_store.clone(),
        ).await.unwrap();

        // Try to fetch initial data using DataManager
        log::info!("Attempting initial data fetch...");
        let selected_metrics = vec!["best_bid", "best_ask"];
        let _ = data_manager.fetch_data(
            &topic,
            start as u64,
            end as u64,
            &selected_metrics,
        ).await;

        // Set the time range
        data_store
            .borrow_mut()
            .set_x_range(start as u32, end as u32);

        log::info!(
            "LineGraph initialization completed (data fetch may have failed gracefully)"
        );

        // Create the LineGraph instance
        Ok(Self {
            data_store,
            renderer,
            data_manager,
        })
    }

    pub async fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Use the new Renderer
        self.renderer.render().await.map_err(|e| match e {
            shared_types::GpuChartsError::Surface { .. } => wgpu::SurfaceError::Outdated,
            _ => wgpu::SurfaceError::Outdated,
        })
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.data_store.borrow_mut().resized(width, height);
        self.renderer.resize(width, height);
    }


    // Switch chart type
    pub fn set_chart_type(&mut self, chart_type: &str) {
        let new_type = match chart_type {
            "candlestick" => ChartType::Candlestick,
            _ => ChartType::Line,
        };

        log::info!("Setting chart type to {new_type:?}");
        self.data_store.borrow_mut().set_chart_type(new_type);
        // Renderer will automatically update based on chart type in data_store
        self.renderer.set_chart_type(new_type);
    }

    // Set candle timeframe (in seconds)
    pub fn set_candle_timeframe(&mut self, timeframe_seconds: u32) {
        self.data_store
            .borrow_mut()
            .set_candle_timeframe(timeframe_seconds);
        // Note: The candlestick renderer will read this value from data_store
    }
}

pub fn unix_timestamp_to_string(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    datetime.unwrap().to_rfc3339()
}
