//! Custom event types to replace winit events for React integration

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalPosition {
    pub x: f64,
    pub y: f64,
}

impl PhysicalPosition {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseScrollDelta {
    PixelDelta(PhysicalPosition),
    // LineDelta(f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ElementState {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    // Middle,
    // Other(u16),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TouchPhase {
    // Started,
    Moved,
    // Ended,
    // Cancelled,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WindowEvent {
    MouseWheel {
        delta: MouseScrollDelta,
        phase: TouchPhase,
    },
    CursorMoved {
        position: PhysicalPosition,
    },
    MouseInput {
        state: ElementState,
        button: MouseButton,
    },
}
