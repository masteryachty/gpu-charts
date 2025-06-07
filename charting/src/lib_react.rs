use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, console};

mod calcables;
mod controls;
mod drawables;
mod renderer;
mod wrappers;
mod line_graph;

use crate::line_graph::LineGraph;
use crate::controls::canvas_controller::CanvasController;
use winit::{
    dpi::PhysicalSize,
    event::{WindowEvent, MouseScrollDelta, ElementState},
    window::{Window, WindowId},
    platform::web::WindowExtWebSys,
};

extern crate nalgebra_glm as glm;

// Global state for the chart instance
static mut CHART_INSTANCE: Option<ChartInstance> = None;

struct ChartInstance {
    line_graph: Rc<RefCell<LineGraph>>,
    canvas_controller: CanvasController,
    window: Rc<Window>,
}

#[wasm_bindgen]
pub struct Chart {
    instance_id: u32,
}

#[wasm_bindgen]
impl Chart {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Chart {
        Chart { instance_id: 0 }
    }

    #[wasm_bindgen]
    pub async fn init(&self, canvas_id: &str, width: u32, height: u32) -> Result<(), JsValue> {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Debug).expect("Couldn't initialize logger");
            }
        }

        log::info!("Initializing chart with canvas: {}, size: {}x{}", canvas_id, width, height);

        // Get the canvas element
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or("Canvas not found")?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Element is not a canvas")?;

        // Set canvas size
        canvas.set_width(width);
        canvas.set_height(height);

        // Create a mock event loop and window for winit compatibility
        let event_loop = winit::event_loop::EventLoop::new()
            .map_err(|e| format!("Failed to create event loop: {:?}", e))?;
        
        let window_attrs = Window::default_attributes()
            .with_inner_size(PhysicalSize::new(width, height));

        // For WASM, we need to associate with the canvas
        #[cfg(target_arch = "wasm32")]
        let window_attrs = {
            use winit::platform::web::WindowAttributesExtWebSys;
            window_attrs.with_canvas(Some(canvas))
        };

        let window = Rc::new(event_loop.create_window(window_attrs)
            .map_err(|e| format!("Failed to create window: {:?}", e))?);

        // Initialize the line graph
        let line_graph = LineGraph::new(width, height, window.clone())
            .await
            .map_err(|e| format!("Failed to create LineGraph: {:?}", e))?;

        let line_graph = Rc::new(RefCell::new(line_graph));

        // Create canvas controller
        let data_store = line_graph.borrow().data_store.clone();
        let engine = line_graph.borrow().engine.clone();
        let canvas_controller = CanvasController::new(window.clone(), data_store, engine);

        // Store globally (in a real app, you'd want better state management)
        let instance = ChartInstance {
            line_graph,
            canvas_controller,
            window,
        };

        unsafe {
            CHART_INSTANCE = Some(instance);
        }

        // Initial render
        self.render().await?;

        log::info!("Chart initialized successfully");
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn render(&self) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &CHART_INSTANCE {
                instance.line_graph.borrow().render()
                    .await
                    .map_err(|e| format!("Render failed: {:?}", e))?;
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn resize(&self, width: u32, height: u32) -> Result<(), JsValue> {
        log::info!("Resizing chart to: {}x{}", width, height);
        
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                instance.line_graph.borrow_mut().resized(width, height);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_wheel(&self, delta_y: f64, x: f64, y: f64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let window_event = WindowEvent::MouseWheel {
                    device_id: unsafe { std::mem::transmute(0u32) },
                    delta: MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition::new(0.0, delta_y)),
                    phase: winit::event::TouchPhase::Moved,
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_move(&self, x: f64, y: f64) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let window_event = WindowEvent::CursorMoved {
                    device_id: unsafe { std::mem::transmute(0u32) },
                    position: winit::dpi::PhysicalPosition::new(x, y),
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn handle_mouse_click(&self, x: f64, y: f64, pressed: bool) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &mut CHART_INSTANCE {
                let window_event = WindowEvent::MouseInput {
                    device_id: unsafe { std::mem::transmute(0u32) },
                    state: if pressed { ElementState::Pressed } else { ElementState::Released },
                    button: winit::event::MouseButton::Left,
                };
                instance.canvas_controller.handle_cursor_event(window_event);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_data_range(&self, start: u32, end: u32) -> Result<(), JsValue> {
        unsafe {
            if let Some(instance) = &CHART_INSTANCE {
                instance.line_graph.borrow().data_store.borrow_mut().set_x_range(start, end);
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn request_redraw(&self) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let closure = Closure::once_into_js(move || {
            wasm_bindgen_futures::spawn_local(async move {
                unsafe {
                    if let Some(instance) = &CHART_INSTANCE {
                        let _ = instance.line_graph.borrow().render().await;
                    }
                }
            });
        });
        
        window.request_animation_frame(closure.as_ref().unchecked_ref())?;
        closure.forget();
        Ok(())
    }
}

// Remove the auto-start function
// #[wasm_bindgen(start)]
// pub fn run() { ... }

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}