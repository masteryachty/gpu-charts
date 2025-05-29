use crate::drawables::plot::PlotRenderer;
use crate::drawables::x_axis::XAxisRenderer;
use crate::renderer::data_retriever::{
    create_chunked_gpu_buffer_from_arraybuffer, fetch_binary, fetch_data,
};
use crate::renderer::data_store::{self, DataStore};
use crate::renderer::render_engine::{GraphicsError, RenderEngine};
use crate::wrappers::js::get_query_params;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
// use crate::renderer::web_socket::WebSocketConnnection;
// use crate::AppOrchestrator;

use futures::join;
use js_sys::{Int32Array, Uint8Array};
use winit::window::Window;

pub struct LineGraph {
    pub data_store: Rc<RefCell<DataStore>>,
    pub line_width: f32,
    pub engine: Rc<RefCell<RenderEngine>>,
    // web_socket: WebSocketConnnection,
}

fn unix_timestamp_to_string(timestamp: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp, 0);
    // let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);
    datetime.unwrap().to_rfc3339()
}

impl LineGraph {
    pub async fn new(
        width: u32,
        height: u32,
        window: std::rc::Rc<Window>,
    ) -> Result<LineGraph, GraphicsError> {
        let params = get_query_params();
        let topic = params.get("topic").unwrap();
        let start = params.get("start").unwrap().parse().unwrap();
        let end = params.get("end").unwrap().parse().unwrap();
        log::info!("topic: {:?}", topic);
        log::info!("start: {:?}", unix_timestamp_to_string(start));
        log::info!("end: {:?}", unix_timestamp_to_string(end));

        let mut ds = DataStore::new();

        // log::info!("0");
        let data_store = Rc::new(RefCell::new(ds));

        let engine: Rc<RefCell<RenderEngine>>;
        {
            // let performance = web_sys::window().unwrap().performance().unwrap();
            let enginePromise =
                RenderEngine::new(width, height, window.clone(), data_store.clone()).await;

            engine = Rc::new(RefCell::new(enginePromise.unwrap()));
            let engine_b = engine.borrow();
            let device = &engine_b.device;
            fetch_data(
                &device,
                topic,
                start as u32,
                end as u32,
                data_store.clone(),
            )
            .await;
            // let start_u32: u32 = start.parse().unwrap();
            // let end_u32: u32 = end.parse().unwrap();
            data_store
                .borrow_mut()
                .set_x_range(start as u32, end as u32, &device);
        }

        let plot_renderer = Box::new(PlotRenderer::new(
            engine.clone(),
            engine.borrow().config.format,
            data_store.clone(),
        ));

        log::info!("wxh: {:?} {:?}", width, height);

        let x_axis_renderer = Box::new(XAxisRenderer::new(
            engine.clone(),
            engine.borrow().config.format,
            width,
            height,
            data_store.clone(),
        ));
        {
            let mut engine_b = engine.borrow_mut();
            engine_b.add_render_listener(plot_renderer);
            engine_b.add_render_listener(x_axis_renderer);
        }
        Ok(Self {
            engine,
            line_width: 1.0,
            data_store: data_store,
            // web_socket,
        })
    }

pub async fn render(&self) -> Result<(), wgpu::SurfaceError> {
    self.engine.borrow_mut().render().await
}

    pub fn resized(&mut self, width: u32, height: u32) {
        self.engine.borrow_mut().resized(width, height);
    }
}
