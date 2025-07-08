use std::{cell::RefCell, rc::Rc};

use crate::{
    events::{ElementState, MouseScrollDelta, WindowEvent},
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
    position: Position,
    start_drag_pos: Option<Position>,
    data_store: Rc<RefCell<DataStore>>,
    engine: Rc<RefCell<RenderEngine>>,
}

impl CanvasController {
    pub fn new(data_store: Rc<RefCell<DataStore>>, engine: Rc<RefCell<RenderEngine>>) -> Self {
        CanvasController {
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
        }
    }

    fn handle_cursor_moved(&mut self, position: crate::events::PhysicalPosition) {
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
        state: crate::events::ElementState,
        button: crate::events::MouseButton,
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

        // Ensure start is less than end
        let (new_start, new_end) = if start_ts.0 < end_ts.0 {
            (start_ts.0 as u32, end_ts.0 as u32)
        } else {
            (end_ts.0 as u32, start_ts.0 as u32)
        };

        log::info!(
            "Drag zoom: {} to {} ({} to {})",
            new_start,
            new_end,
            unix_timestamp_to_string(new_start as i64),
            unix_timestamp_to_string(new_end as i64)
        );

        // Fetch data for the new range and update
        let data_store = self.data_store.clone();
        let engine = self.engine.clone();

        spawn_local(async move {
            let device = {
                let engine_borrow = engine.try_borrow();
                if let Ok(engine_ref) = engine_borrow {
                    engine_ref.device.clone()
                } else {
                    log::warn!("Engine is borrowed, skipping data fetch");
                    return;
                }
            };

            fetch_data(&device, new_start, new_end, data_store.clone(), None).await;

            // Use try_borrow_mut to prevent panic
            if let Ok(mut data_store_mut) = data_store.try_borrow_mut() {
                data_store_mut.set_x_range(new_start, new_end);
            }

            log::info!("Drag zoom completed: {} to {}", new_start, new_end);
        });
    }

    fn handle_cursor_wheel(
        &self,
        delta: crate::events::MouseScrollDelta,
        phase: crate::events::TouchPhase,
    ) {
        log::info!("handle_cursor_wheel type: {:?} {:?}", delta, phase);

        let MouseScrollDelta::PixelDelta(position) = delta;
        let data_store = self.data_store.clone();
        let engine = self.engine.clone();

        spawn_local(async move {
            let start_x = data_store.borrow().start_x;
            let end_x = data_store.borrow().end_x;
            let range = end_x - start_x;

            let (new_start, new_end) = if position.y < 0. {
                // Scrolling up = zoom out (expand range)
                let new_start = start_x - (range / 2);
                let new_end = end_x + (range / 2);
                (new_start, new_end)
            } else if position.y > 0. {
                // Scrolling down = zoom in (shrink range)
                let new_start = start_x + (range / 4);
                let new_end = end_x - (range / 4);
                // Ensure we don't zoom in too much
                if new_end > new_start {
                    (new_start, new_end)
                } else {
                    (start_x, end_x) // Keep current range if too zoomed in
                }
            } else {
                (start_x, end_x) // No change
            };

            // Only update if range actually changed
            if new_start != start_x || new_end != end_x {
                let device = {
                    let engine_borrow = engine.try_borrow();
                    if let Ok(engine_ref) = engine_borrow {
                        engine_ref.device.clone()
                    } else {
                        log::warn!("Engine is borrowed, skipping data fetch");
                        return;
                    }
                };

                fetch_data(&device, new_start, new_end, data_store.clone(), None).await;

                // Use try_borrow_mut to prevent panic
                if let Ok(mut data_store_mut) = data_store.try_borrow_mut() {
                    data_store_mut.set_x_range(new_start, new_end);
                }

                log::info!(
                    "Zoom: new_start = {}, new_end = {} (delta_y = {})",
                    new_start,
                    new_end,
                    position.y
                );
            }
        });
    }
}
