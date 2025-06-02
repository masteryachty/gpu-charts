use std::{cell::RefCell, rc::Rc};

use winit::{
    event::{ElementState, MouseScrollDelta, WindowEvent},
    window::Window,
};

use crate::{
    line_graph::unix_timestamp_to_string,
    renderer::{data_retriever::fetch_data, data_store::DataStore, render_engine::RenderEngine},
};

use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    x: f64,
    y: f64,
}
pub struct CanvasController {
    window: Rc<Window>,
    position: Position,
    start_drag_pos: Option<Position>,
    data_store: Rc<RefCell<DataStore>>,
    engine: Rc<RefCell<RenderEngine>>,
}

impl CanvasController {
    pub fn new(
        window: Rc<Window>,
        data_store: Rc<RefCell<DataStore>>,
        engine: Rc<RefCell<RenderEngine>>,
    ) -> Self {
        CanvasController {
            window,
            position: Position { x: -1., y: -1. },
            start_drag_pos: None,
            data_store,
            engine,
        }
    }

    pub fn handle_cursor_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::MouseWheel { delta, phase, .. } => {
                self.handle_cursor_wheel(delta, phase);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_cursor_input(state, button);
            }
            _ => {
                log::info!("Cursor Event type not handled: {:?}", event);
            }
        }
    }

    fn handle_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        // log::info!("CursorMoved type: {:?}", position);
        if position.x != self.position.x || position.y != self.position.y {
            self.position = Position {
                x: position.x,
                y: position.y,
            };
        }
    }

    fn handle_cursor_input(
        &mut self,
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) {
        match state {
            ElementState::Pressed => {
                self.start_drag_pos = Some(self.position);
            }
            ElementState::Released => {
                if let Some(start_pos) = self.start_drag_pos {
                    if start_pos != self.position {
                        self.apply_drag_zoom(start_pos, self.position);
                    }
                }
                // Always clear the drag position after release
                self.start_drag_pos = None;
            }
        }
        log::info!("MouseInput type: {:?} {:?}", button, state);
    }

    fn apply_drag_zoom(&self, start_pos: Position, end_position: Position) {
        let start_ts = self
            .data_store
            .borrow()
            .screen_to_world_with_margin(start_pos.x as f32, start_pos.y as f32);
        let end_ts = self
            .data_store
            .borrow()
            .screen_to_world_with_margin(end_position.x as f32, end_position.y as f32);
        log::info!(
            "start: {:?} {:?}",
            start_ts.0,
            unix_timestamp_to_string(start_ts.0 as i64)
        );
        log::info!(
            "end: {:?} {:?}",
            end_ts.0,
            unix_timestamp_to_string(end_ts.0 as i64)
        );

        self.data_store
            .borrow_mut()
            .set_x_range(start_ts.0 as u32, end_ts.0 as u32);

        self.window.request_redraw();
    }

    fn handle_cursor_wheel(
        &self,
        delta: winit::event::MouseScrollDelta,
        phase: winit::event::TouchPhase,
    ) {
        log::info!("handle_cursor_wheel type: {:?} {:?}", delta, phase);

        match delta {
            MouseScrollDelta::PixelDelta(position) => {
                // println!("Scrolled (lines): x = {}, y = {}", position.x, .positiony);
                if position.y < 0. {
                    let data_store = self.data_store.clone();
                    let engine = self.engine.clone();
                    let window = self.window.clone();

                    spawn_local(async move {
                        let start_x = data_store.borrow().start_x;
                        let end_x = data_store.borrow().end_x;
                        let range = end_x - start_x;
                        let new_start = start_x - (range / 2);
                        let new_end = end_x + (range / 2);

                        let device = &engine.borrow().device;

                        fetch_data(device, new_start as u32, new_end as u32, data_store.clone())
                            .await;
                        data_store
                            .borrow_mut()
                            .set_x_range(new_start as u32, new_end as u32);

                        window.request_redraw();
                        log::info!("Scrolled: new_start = {}, new_end = {}", new_start, new_end);
                    });
                }
            }
            _ => {}
        }
    }
}
