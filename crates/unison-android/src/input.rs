//! Touch input helpers for Android.
//!
//! Provides convenience functions for feeding Android MotionEvent data into
//! the engine's `InputBuffer`. Called from the game crate's JNI layer.

use unison_input::InputBuffer;

/// Feed a touch-began event into the input buffer.
pub fn touch_began(input: &mut InputBuffer, id: u64, x: f32, y: f32) {
    input.shared_mut().touch_started(id, x, y);
}

/// Feed a touch-moved event into the input buffer.
pub fn touch_moved(input: &mut InputBuffer, id: u64, x: f32, y: f32) {
    input.shared_mut().touch_moved(id, x, y);
}

/// Feed a touch-ended event into the input buffer.
pub fn touch_ended(input: &mut InputBuffer, id: u64) {
    input.shared_mut().touch_ended(id);
}

/// Feed a touch-cancelled event into the input buffer.
pub fn touch_cancelled(input: &mut InputBuffer, id: u64) {
    input.shared_mut().touch_cancelled(id);
}

/// Set the axis value (e.g., from a virtual joystick).
pub fn set_axis(input: &mut InputBuffer, x: f32, y: f32) {
    input.shared_mut().set_axis(x, y);
}
