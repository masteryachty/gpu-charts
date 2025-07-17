use crate::drawables::plot::{PlotRenderer, RenderListener};
use crate::drawables::x_axis::XAxisRenderer;
use crate::drawables::y_axis::YAxisRenderer;
use crate::drawables::candlestick::CandlestickRenderer;
use crate::renderer::data_store::ChartType;
use crate::renderer::data_retriever::fetch_data;
use crate::renderer::data_store::DataStore;
use crate::renderer::render_engine::RenderEngine;
use crate::wrappers::js::get_query_params;
use chrono::DateTime;
use js_sys::Error;
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

pub struct LineGraph {
    pub data_store: Rc<RefCell<DataStore>>,
    // pub line_width: f32,
    pub engine: Rc<RefCell<RenderEngine>>,
    // web_socket: WebSocketConnnection,
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

        log::info!("topic: {topic:?}");
        log::info!("start: {:?}", unix_timestamp_to_string(start));
        log::info!("end: {:?}", unix_timestamp_to_string(end));

        let ds = DataStore::new(width, height);

        // log::info!("0");
        let data_store = Rc::new(RefCell::new(ds));

        let engine: Rc<RefCell<RenderEngine>>;
        {
            // let performance = web_sys::window().unwrap().performance().unwrap();
            let engine_promise = RenderEngine::new(canvas, data_store.clone()).await;

            engine = Rc::new(RefCell::new(engine_promise.unwrap()));
            let device = {
                let engine_b = engine.borrow();
                engine_b.device.clone()
            };
            data_store.borrow_mut().topic = Some(topic.clone());

            // Try to fetch initial data, but don't fail if server is unavailable
            log::info!("Attempting initial data fetch...");
            fetch_data(&device, start as u32, end as u32, data_store.clone(), None).await;

            // Set the time range regardless of whether data fetch succeeded
            data_store
                .borrow_mut()
                .set_x_range(start as u32, end as u32);

            log::info!(
                "LineGraph initialization completed (data fetch may have failed gracefully)"
            );
        }

        log::info!("wxh: {width:?} {height:?}");

        // Create the LineGraph instance
        let mut line_graph = Self {
            engine,
            data_store,
        };
        
        // Set up initial renderers
        line_graph.setup_renderers();
        
        Ok(line_graph)
    }

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn render(&self) -> Result<(), wgpu::SurfaceError> {
        // We need to be careful with RefCell borrows across await points
        // Check if we can borrow first, then clone the future
        match self.engine.try_borrow_mut() {
            Ok(mut engine) => engine.render().await,
            Err(_) => Ok(()),
        }
    }

    pub fn resized(&mut self, width: u32, height: u32) {
        self.data_store.borrow_mut().resized(width, height);
        self.engine.borrow_mut().resized(width, height);
    }
    
    // Set up renderers based on current chart type
    fn setup_renderers(&mut self) {
        // Get all the values we need before mutably borrowing
        let format = self.engine.borrow().config.format;
        let chart_type = self.data_store.borrow().chart_type;
        
        // Create all renderers before mutably borrowing engine
        let plot_renderer: Box<dyn RenderListener> = match chart_type {
            ChartType::Line => {
                Box::new(PlotRenderer::new(
                    self.engine.clone(),
                    format,
                    self.data_store.clone(),
                ))
            }
            ChartType::Candlestick => {
                Box::new(CandlestickRenderer::new(
                    self.engine.clone(),
                    format,
                    self.data_store.clone(),
                ))
            }
        };
        
        let x_axis_renderer = Box::new(XAxisRenderer::new(
            self.engine.clone(),
            format,
            self.data_store.clone(),
        ));
        
        let y_axis_renderer = Box::new(YAxisRenderer::new(
            self.engine.clone(),
            format,
            self.data_store.clone(),
        ));
        
        // Now do all the mutations in one go
        {
            let mut engine_mut = self.engine.borrow_mut();
            engine_mut.clear_render_listeners();
            engine_mut.add_render_listener(plot_renderer);
            engine_mut.add_render_listener(x_axis_renderer);
            engine_mut.add_render_listener(y_axis_renderer);
        }
    }
    
    // Switch chart type
    pub fn set_chart_type(&mut self, chart_type: &str) {
        let new_type = match chart_type {
            "candlestick" => ChartType::Candlestick,
            _ => ChartType::Line,
        };
        
        self.data_store.borrow_mut().set_chart_type(new_type);
        self.setup_renderers();
    }
    
    // Set candle timeframe (in seconds)
    pub fn set_candle_timeframe(&mut self, timeframe_seconds: u32) {
        self.data_store.borrow_mut().set_candle_timeframe(timeframe_seconds);
        // Note: The candlestick renderer will read this value from data_store
    }
}

pub fn unix_timestamp_to_string(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    datetime.unwrap().to_rfc3339()
}
