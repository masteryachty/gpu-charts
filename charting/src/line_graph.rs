use crate::drawables::candlestick::CandlestickRenderer;
use crate::drawables::plot::{PlotRenderer, RenderListener};
use crate::drawables::x_axis::XAxisRenderer;
use crate::drawables::y_axis::YAxisRenderer;
use crate::renderer::data_store::ChartType;
use crate::renderer::data_store::DataStore;
use crate::renderer::render_engine::RenderEngine;
use crate::renderer::culling::CullingSystem;
use crate::renderer::vertex_compression::ChartVertexCompression;
use crate::renderer::gpu_vertex_gen::ChartGpuVertexGen;
use crate::renderer::render_bundles::ChartRenderBundles;
use chrono::DateTime;

#[cfg(target_arch = "wasm32")]
use crate::renderer::data_retriever::fetch_data;
#[cfg(target_arch = "wasm32")]
use crate::wrappers::js::get_query_params;
#[cfg(target_arch = "wasm32")]
use js_sys::Error;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use web_sys::HtmlCanvasElement;

pub struct LineGraph {
    pub data_store: Rc<RefCell<DataStore>>,
    // pub line_width: f32,
    pub engine: Rc<RefCell<RenderEngine>>,
    pub culling_system: Option<Rc<RefCell<CullingSystem>>>,
    pub vertex_compression: Option<Rc<RefCell<ChartVertexCompression>>>,
    pub gpu_vertex_gen: Option<Rc<RefCell<ChartGpuVertexGen>>>,
    pub render_bundles: Option<Rc<RefCell<ChartRenderBundles>>>,
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
            // Only fetch if we have a topic (from URL params)
            if !topic.is_empty() && topic != "default_topic" {
                log::info!("Attempting initial data fetch...");
                fetch_data(&device, start as u32, end as u32, data_store.clone(), None).await;
            } else {
                log::info!("Skipping initial data fetch - no topic specified");
            }

            // Set the time range regardless of whether data fetch succeeded
            data_store
                .borrow_mut()
                .set_x_range(start as u32, end as u32);

            log::info!(
                "LineGraph initialization completed (data fetch may have failed gracefully)"
            );
        }

        // Create the LineGraph instance
        let mut line_graph = Self { 
            engine: engine.clone(), 
            data_store,
            culling_system: None,
            vertex_compression: None,
            gpu_vertex_gen: None,
            render_bundles: None,
        };

        // Initialize culling system with Phase 2 optimizations if available
        {
            let engine_b = engine.borrow();
            let device = Arc::new(engine_b.device.clone());
            let queue = Arc::new(engine_b.queue.clone());
            
            let culling = CullingSystem::new(device.clone(), queue.clone());
            line_graph.culling_system = Some(Rc::new(RefCell::new(culling)));
            
            // Initialize vertex compression if enabled
            if std::env::var("ENABLE_VERTEX_COMPRESSION").unwrap_or_default() == "1" {
                let compression = ChartVertexCompression::new(device.clone(), queue.clone());
                line_graph.vertex_compression = Some(Rc::new(RefCell::new(compression)));
                log::info!("Vertex compression enabled");
            }
            
            // Initialize GPU vertex generation if enabled
            if std::env::var("ENABLE_GPU_VERTEX_GEN").unwrap_or_default() == "1" {
                let gpu_gen = ChartGpuVertexGen::new(device.clone(), queue.clone());
                line_graph.gpu_vertex_gen = Some(Rc::new(RefCell::new(gpu_gen)));
                log::info!("GPU vertex generation enabled");
            }
            
            // Initialize render bundles if enabled
            if std::env::var("ENABLE_RENDER_BUNDLES").unwrap_or_default() == "1" {
                let render_bundles = ChartRenderBundles::new(device.clone());
                line_graph.render_bundles = Some(Rc::new(RefCell::new(render_bundles)));
                log::info!("Render bundles enabled");
            }
            
            // Try to initialize GPU culling if feature is enabled
            #[cfg(feature = "phase2-optimizations")]
            {
                if let Some(culling_ref) = &line_graph.culling_system {
                    let canvas_id = "gpu-chart"; // You might want to get this from the canvas element
                    match culling_ref.borrow_mut().init_gpu_culling(canvas_id).await {
                        Ok(_) => log::info!("GPU binary search culling initialized successfully!"),
                        Err(e) => log::warn!("Failed to initialize GPU culling: {}", e),
                    }
                }
            }
        }

        // Set up initial renderers
        line_graph.setup_renderers();

        Ok(line_graph)
    }

    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn render(&self) -> Result<(), wgpu::SurfaceError> {
        // Check if rendering is needed
        if !self.data_store.borrow().is_dirty() {
            return Ok(());
        }

        // We need to be careful with RefCell borrows across await points
        // Check if we can borrow first, then clone the future
        match self.engine.try_borrow_mut() {
            Ok(mut engine) => {
                let result = engine.render().await;
                if result.is_ok() {
                    // Mark as clean after successful render
                    self.data_store.borrow_mut().mark_clean();
                }
                result
            }
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

        log::info!("Setting up renderers for chart type: {chart_type:?}");

        // Create all renderers before mutably borrowing engine
        let plot_renderer: Box<dyn RenderListener> = match chart_type {
            ChartType::Line => {
                let mut plot = PlotRenderer::new(
                    self.engine.clone(),
                    format,
                    self.data_store.clone(),
                    self.culling_system.clone(),
                );
                // Set vertex compression if available
                plot.set_vertex_compression(self.vertex_compression.clone());
                // Set GPU vertex generation if available
                plot.set_gpu_vertex_gen(self.gpu_vertex_gen.clone());
                // Set render bundles if available
                plot.set_render_bundles(self.render_bundles.clone());
                Box::new(plot)
            },
            ChartType::Candlestick => Box::new(CandlestickRenderer::new(
                self.engine.clone(),
                format,
                self.data_store.clone(),
            )),
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
            match self.engine.try_borrow_mut() {
                Ok(mut engine_mut) => {
                    engine_mut.clear_render_listeners();
                    engine_mut.add_render_listener(plot_renderer);
                    engine_mut.add_render_listener(x_axis_renderer);
                    engine_mut.add_render_listener(y_axis_renderer);
                }
                Err(e) => {
                    log::error!("Failed to borrow engine mutably: {e:?}");
                }
            }
        }
    }

    // Switch chart type
    pub fn set_chart_type(&mut self, chart_type: &str) {
        let new_type = match chart_type {
            "candlestick" => ChartType::Candlestick,
            _ => ChartType::Line,
        };

        log::info!("Setting chart type to {new_type:?}");
        self.data_store.borrow_mut().set_chart_type(new_type);
        self.setup_renderers();
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
