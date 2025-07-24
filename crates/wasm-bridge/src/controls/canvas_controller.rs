use crate::line_graph::unix_timestamp_to_string;
use renderer::Renderer;
use shared_types::events::{ElementState, MouseScrollDelta, WindowEvent};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    x: f64,
    y: f64,
}

pub struct CanvasController {
    position: Position,
    start_drag_pos: Option<Position>,
}

impl CanvasController {
    pub fn new() -> Self {
        CanvasController {
            position: Position { x: -1., y: -1. },
            start_drag_pos: None,
        }
    }

    pub fn handle_cursor_event(&mut self, event: WindowEvent, renderer: &mut Renderer) {
        match event {
            WindowEvent::MouseWheel { delta, phase, .. } => {
                self.handle_cursor_wheel(delta, phase, renderer);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_cursor_input(state, button, renderer);
            }
        }
    }

    fn handle_cursor_moved(&mut self, position: shared_types::events::PhysicalPosition) {
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
        state: shared_types::events::ElementState,
        button: shared_types::events::MouseButton,
        renderer: &mut Renderer,
    ) {
        match state {
            ElementState::Pressed => {
                self.start_drag_pos = Some(self.position);
            }
            ElementState::Released => {
                if let Some(start_pos) = self.start_drag_pos {
                    if start_pos != self.position {
                        self.apply_drag_zoom(start_pos, self.position, renderer);
                    }
                }
                // Always clear the drag position after release
                self.start_drag_pos = None;
            }
        }
        log::info!("MouseInput type: {button:?} {state:?}");
    }

    fn apply_drag_zoom(
        &self,
        start_pos: Position,
        end_position: Position,
        renderer: &mut Renderer,
    ) {
        let start_ts = renderer
            .data_store()
            .screen_to_world_with_margin(start_pos.x as f32, start_pos.y as f32);
        let end_ts = renderer
            .data_store()
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

        // Update the data store range
        // Note: Data fetching should be handled by the parent component using DataManager
        renderer.data_store_mut().set_x_range(new_start, new_end);
        log::info!("Drag zoom completed: {new_start} to {new_end}");
    }

    fn handle_cursor_wheel(
        &self,
        delta: shared_types::events::MouseScrollDelta,
        phase: shared_types::events::TouchPhase,
        renderer: &mut Renderer,
    ) {
        log::info!("handle_cursor_wheel type: {delta:?} {phase:?}");

        let MouseScrollDelta::PixelDelta(position) = delta;

        let start_x = renderer.data_store().start_x;
        let end_x = renderer.data_store().end_x;
        let range = end_x - start_x;

        let (new_start, new_end) = if position.y < 0. {
            // Scrolling up = zoom in (shrink range)
            let new_start = start_x + (range / 4);
            let new_end = end_x - (range / 4);
            // Ensure we don't zoom in too much
            if new_end > new_start {
                (new_start, new_end)
            } else {
                (start_x, end_x) // Keep current range if too zoomed in
            }
        } else if position.y > 0. {
            // Scrolling down = zoom out (expand range)
            let new_start = start_x - (range / 2);
            let new_end = end_x + (range / 2);
            (new_start, new_end)
        } else {
            (start_x, end_x) // No change
        };

        // Only update if range actually changed
        if new_start != start_x || new_end != end_x {
            // Update the data store range
            // Note: Data fetching should be handled by the parent component using DataManager
            renderer.data_store_mut().set_x_range(new_start, new_end);

            log::info!(
                "Zoom: new_start = {}, new_end = {} (delta_y = {})",
                new_start,
                new_end,
                position.y
            );
        }
    }
}
