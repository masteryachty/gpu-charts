use crate::chart_engine::unix_timestamp_to_string;
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

impl Default for CanvasController {
    fn default() -> Self {
        CanvasController {
            position: Position { x: -1., y: -1. },
            start_drag_pos: None,
        }
    }
}

impl CanvasController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_cursor_event(&mut self, event: WindowEvent, renderer: &mut Renderer) {
        // log::info!(
        //     "[CanvasController] handle_cursor_event called with event: {:?}",
        //     event
        // );

        match event {
            WindowEvent::MouseWheel { delta, phase, .. } => {
                log::info!(
                    "[CanvasController] MouseWheel event detected, calling handle_cursor_wheel"
                );
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

        // IMPORTANT: After changing x range, we need to recalculate Y bounds
        // and update the shared bind group for axis rendering
        renderer.data_store_mut().mark_dirty();
        log::info!("Drag zoom completed: {new_start} to {new_end}");
    }

    fn handle_cursor_wheel(
        &self,
        delta: shared_types::events::MouseScrollDelta,
        phase: shared_types::events::TouchPhase,
        renderer: &mut Renderer,
    ) {
        log::info!("[handle_cursor_wheel] START - delta: {delta:?}, phase: {phase:?}");

        let MouseScrollDelta::PixelDelta(position) = delta;
        log::info!(
            "[handle_cursor_wheel] PixelDelta position: x={}, y={}",
            position.x,
            position.y
        );

        let start_x = renderer.data_store().start_x;
        let end_x = renderer.data_store().end_x;
        let range = end_x - start_x;

        log::info!(
            "[handle_cursor_wheel] Current range: start_x={}, end_x={}, range={}",
            start_x,
            end_x,
            range
        );

        // Zoom factor based on scroll amount
        let zoom_factor = 0.1; // 10% zoom per scroll
        let zoom_amount = (range as f32 * zoom_factor) as u32;

        log::info!(
            "[handle_cursor_wheel] Zoom factor={}, zoom_amount={}",
            zoom_factor,
            zoom_amount
        );

        let (new_start, new_end) = if position.y < 0. {
            // Scrolling up = zoom in (shrink range)
            log::info!("[handle_cursor_wheel] Scrolling UP detected (zoom IN)");
            let new_start = start_x + zoom_amount;
            let new_end = end_x - zoom_amount;
            // Ensure we don't zoom in too much (minimum range of 10 units)
            if new_end > new_start + 10 {
                log::info!(
                    "[handle_cursor_wheel] Zoom IN accepted: new_start={}, new_end={}",
                    new_start,
                    new_end
                );
                (new_start, new_end)
            } else {
                log::info!("[handle_cursor_wheel] Zoom IN rejected - would be too zoomed in");
                (start_x, end_x) // Keep current range if too zoomed in
            }
        } else if position.y > 0. {
            // Scrolling down = zoom out (expand range)
            log::info!("[handle_cursor_wheel] Scrolling DOWN detected (zoom OUT)");
            let new_start = start_x.saturating_sub(zoom_amount);
            let new_end = end_x + zoom_amount;
            log::info!(
                "[handle_cursor_wheel] Zoom OUT: new_start={}, new_end={}",
                new_start,
                new_end
            );
            (new_start, new_end)
        } else {
            log::info!("[handle_cursor_wheel] No Y delta - no zoom");
            (start_x, end_x) // No change
        };

        // Only update if range actually changed
        if new_start != start_x || new_end != end_x {
            log::info!("[handle_cursor_wheel] Range changed, updating data store");

            // Update the data store range
            // Note: Data fetching should be handled by the parent component using DataManager
            renderer.data_store_mut().set_x_range(new_start, new_end);

            // IMPORTANT: After changing x range, we need to recalculate Y bounds
            // and update the shared bind group for axis rendering
            renderer.data_store_mut().mark_dirty();

            log::info!(
                "[handle_cursor_wheel] COMPLETED - Zoom applied: {} to {} (was {} to {})",
                new_start,
                new_end,
                start_x,
                end_x
            );
        } else {
            log::info!("[handle_cursor_wheel] No range change - skipping update");
        }
    }
}
