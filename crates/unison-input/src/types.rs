//! Input types: KeyCode, MouseButton, Touch, TouchPhase

use unison_core::Vec2;

/// Keyboard key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Arrow keys
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Numbers
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,

    // Control keys
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
}

/// Mouse buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Touch phase (lifecycle of a single touch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    /// Touch just started this frame
    Began,
    /// Touch moved since last frame
    Moved,
    /// Touch is active but did not move
    Stationary,
    /// Touch ended this frame
    Ended,
    /// Touch was cancelled (e.g., by system gesture)
    Cancelled,
}

/// A single touch point
#[derive(Debug, Clone, Copy)]
pub struct Touch {
    /// Unique identifier for this touch (persists across frames)
    pub id: u64,
    /// Current position in screen coordinates
    pub position: Vec2,
    /// Phase of this touch in the current frame
    pub phase: TouchPhase,
}
