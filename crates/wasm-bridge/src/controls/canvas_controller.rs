use shared_types::events::PhysicalPosition;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

impl From<PhysicalPosition> for Position {
    fn from(pos: PhysicalPosition) -> Self {
        Position { x: pos.x, y: pos.y }
    }
}

/// Pure state holder for canvas interaction state
/// All event processing logic is handled in ChartEngine to avoid duplication
pub struct CanvasController {
    pub position: Position,
    pub start_drag_pos: Option<Position>,
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

    /// Update cursor position
    pub fn update_position(&mut self, position: Position) {
        self.position = position;
    }

    /// Start drag operation
    pub fn start_drag(&mut self) {
        self.start_drag_pos = Some(self.position);
    }

    /// End drag operation and return drag positions if it was a real drag
    pub fn end_drag(&mut self) -> Option<(Position, Position)> {
        if let Some(start_pos) = self.start_drag_pos {
            let end_pos = self.position;
            self.start_drag_pos = None;
            
            // Only return positions if there was actual movement
            if start_pos != end_pos {
                Some((start_pos, end_pos))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Clear drag state
    pub fn clear_drag(&mut self) {
        self.start_drag_pos = None;
    }
}