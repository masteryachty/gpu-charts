use std::{cell::RefCell, rc::Rc};

use controls::canvas_controller::CanvasController;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

mod calcables;
mod controls;
mod drawables;
mod renderer;
mod wrappers;

// use renderer::render_engine::GraphicsError;

mod line_graph;

// React bridge module for integration
#[cfg(target_arch = "wasm32")]
mod react_bridge;
use crate::line_graph::LineGraph;
extern crate nalgebra_glm as glm;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const CANVAS_ID: &str = "new-api-canvas";

type AppWindow = std::rc::Rc<Window>;
type AppGraphics = LineGraph;

type AppEvent = (AppWindow, AppGraphics);

enum Application {
    Building(Option<EventLoopProxy<AppEvent>>),
    Running {
        #[allow(unused)]
        window: AppWindow,
        graphics: Rc<RefCell<LineGraph>>,
        canvas_controller: CanvasController,
    },
}

impl Application {
    fn new(event_loop: &EventLoop<AppEvent>) -> Self {
        let loop_proxy = Some(event_loop.create_proxy());
        log::info!("Creating a new application 1");
        Self::Building(loop_proxy)
    }

    fn render(&mut self) {
        match self {
            Self::Running { graphics, .. } => {
                log::info!("draw");
                let graphics = graphics.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Err(e) = graphics.borrow().render().await {
                        log::error!("Render failed: {:?}", e);
                    }
                });
            }
            _ => {
                log::info!("Draw call rejected because graphics doesn't exist yet");
                return;
            }
        }
    }

    fn resized(&mut self, size: PhysicalSize<u32>) {
        let Self::Running { graphics, .. } = self else {
            return;
        };
        let graphics = graphics.clone();
        log::info!("Resized {:?} {:?}", size.width, size.height);
        let g = graphics.try_borrow_mut();
        if g.is_ok() {
            g.unwrap().resized(size.width, size.height);
        }
    }
}

impl ApplicationHandler<AppEvent> for Application {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => self.resized(size),
            WindowEvent::RedrawRequested => {
                log::info!("redraw");

                self.render()
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::MouseWheel { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::CursorMoved { .. } => match self {
                Self::Running {
                    canvas_controller, ..
                } => {
                    canvas_controller.handle_cursor_event(event);
                }
                _ => {}
            },
            event => {
                log::info!("Event type not handled : {:?}", event);
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("App is resumed1");

        let Self::Building(builder) = self else {
            return; // Graphics have been built.
        };
        let Some(loop_proxy) = builder.take() else {
            return; // Graphics are being built.
        };

        let mut window_attrs = Window::default_attributes();
        log::info!("App is resumed 2");

        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            log::info!("a");

            let window = web_sys::window().unwrap_throw();
            log::info!("b");

            let document = window.document().unwrap_throw();
            log::info!("c");

            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            log::info!("d");

            let html_canvas_element = canvas.unchecked_into();
            window_attrs = window_attrs.with_canvas(Some(html_canvas_element));
            log::info!("App is now up and running {:?}", window_attrs);

            let window = std::rc::Rc::new(event_loop.create_window(window_attrs).unwrap_throw());
            let size = window.inner_size();
            log::info!("App is now up and running {} {}", size.width, size.height);
            let state = LineGraph::new(size.width, size.height, window.clone());

            //log::info!("Spawning future to build the graphics context");
            wasm_bindgen_futures::spawn_local(async move {
                let state = state.await.expect_throw("To build a graphics context");
                let _ = loop_proxy.send_event((window, state));
            });
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, app_event: AppEvent) {
        let (window, graphics) = app_event;
        if matches!(self, Self::Running { .. }) {
            log::error!("Received a new graphics context when we already have one");
            return;
        }

        // The surface is not actually configured yet.
        let graphics = Rc::new(RefCell::new(graphics));
        let size = window.inner_size();
        graphics.borrow_mut().resized(size.width, size.height);
        let data_store = graphics.borrow().data_store.clone();
        let canvas_controller =
            CanvasController::new(window.clone(), data_store, graphics.borrow().engine.clone());

        //log::info!("App is now up and running");
        *self = Self::Running {
            window,
            graphics: graphics.clone(),
            canvas_controller,
        };
        self.render();
    }
}

// Auto-start function for standalone mode (COMPLETELY DISABLED for React integration)
// #[cfg(all(target_arch = "wasm32", not(feature = "react-mode")))]
// #[wasm_bindgen(start)]
// pub fn run() {
//     use winit::platform::web::EventLoopExtWebSys;
//     cfg_if::cfg_if! {
//         if #[cfg(target_arch = "wasm32")] {
//             std::panic::set_hook(Box::new(console_error_panic_hook::hook));
//             console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");

//         } else {
//             env_logger::init();
//         }
//     }
//     //log::info!("New Api Example is starting");
//     let event_loop = EventLoop::with_user_event().build().unwrap_throw();
//     event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
//     let app = Application::new(&event_loop);
//     event_loop.spawn_app(app);
// }

// Internal run function (not exported to WASM)
fn run() {
    use winit::platform::web::EventLoopExtWebSys;
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");

        } else {
            env_logger::init();
        }
    }
    log::info!("Internal run: Starting chart application");
    let event_loop = EventLoop::with_user_event().build().unwrap_throw();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let app = Application::new(&event_loop);
    event_loop.spawn_app(app);
}

// Manual run function for React integration
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn manual_run() {
    use winit::platform::web::EventLoopExtWebSys;
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
    log::info!("88889");

            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            let _ = console_log::init_with_level(log::Level::Debug);

        } else {
            env_logger::init();
        }
    }

    log::info!("Manual run: Starting chart application");
    let event_loop = EventLoop::with_user_event().build().unwrap_throw();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let app = Application::new(&event_loop);
    event_loop.spawn_app(app);
}

// macro_rules! impl_errors {
//     (@ours [$($our:path => $desc:literal),+ $(,)?]
//      @theirs [$($wrapper:path => $theirs:ty),+ $(,)?]) => {
//         $(impl From<$theirs> for GraphicsError {
//             fn from(item: $theirs) -> Self { $wrapper(Box::new(item)) }
//         })*
//         impl core::fmt::Display for GraphicsError {
//             fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//                 match self {
//                     $($our => write!(f, $desc),)+
//                     $($wrapper(nested) => nested.fmt(f)),+
//                 }
//             }
//         }
//         impl core::error::Error for GraphicsError {
//             fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//                 match self {
//                     $($wrapper(nested) => Some(nested),)+
//                     _ => None,
//                 }
//             }
//         }
//     };
// }

// impl_errors!(
//     // @ours [
//     //     // GraphicsError::NoCompatibleAdapter => "Could not find a compatible adapter",
//     //     // GraphicsError::IncompatibleAdapter => "Adapter and surface are not compatible",
//     // ]
//     // @theirs [
//     //     // GraphicsError::RequestDeviceError => wgpu::RequestDeviceError,
//     //     // GraphicsError::CreateSurfaceError => wgpu::CreateSurfaceError,
//     // ]
// );
