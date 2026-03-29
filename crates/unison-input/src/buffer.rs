//! Double-buffered input for fixed-timestep game loops.
//!
//! Platform code pushes events into the **shared** buffer asynchronously.
//! At the top of each frame the game loop calls [`InputBuffer::transfer`],
//! which swaps shared→engine *only* when an update tick will actually run.
//! This prevents per-frame events (just_pressed / just_released) from being
//! pulled into the engine on a frame where no update processes them, only
//! to be silently discarded by the next swap.

use crate::InputState;

/// Double-buffered input state for use by platform game loops.
///
/// Owns two [`InputState`] buffers:
/// - **shared** — platform event callbacks write here (DOM, iOS, Android, …)
/// - **engine** — game code reads from here during update ticks
///
/// The [`transfer`](Self::transfer) method encapsulates the swap + housekeeping
/// that every platform game loop needs to get right.
pub struct InputBuffer {
    shared: InputState,
    engine: InputState,
}

impl InputBuffer {
    /// Create a new double buffer with empty input state on both sides.
    pub fn new() -> Self {
        Self {
            shared: InputState::new(),
            engine: InputState::new(),
        }
    }

    /// Mutable access to the shared buffer.
    ///
    /// Platform event handlers call this to push events:
    /// ```ignore
    /// input_buffer.shared_mut().key_pressed(KeyCode::Space);
    /// input_buffer.shared_mut().mouse_moved(x, y);
    /// ```
    pub fn shared_mut(&mut self) -> &mut InputState {
        &mut self.shared
    }

    /// Transfer input from the shared buffer to the engine buffer.
    ///
    /// Call this **once per frame**, before the fixed-timestep loop.
    ///
    /// When `will_update` is true (i.e. the accumulator has enough time for
    /// at least one tick), swaps shared↔engine so the engine gets fresh events,
    /// then resets the shared buffer while preserving continuous state (held keys,
    /// mouse position, active touches) so future platform events have correct context.
    ///
    /// When `will_update` is false, does nothing — events stay in the shared
    /// buffer and accumulate until the next frame where a tick runs.
    pub fn transfer(&mut self, will_update: bool) {
        if will_update {
            std::mem::swap(&mut self.engine, &mut self.shared);
            self.shared.begin_frame();
            self.shared.copy_held_from(&self.engine);
        }
    }

    /// Clear per-frame flags on the engine buffer between fixed-timestep ticks.
    ///
    /// Call this before the second (and subsequent) ticks in a frame so that
    /// `just_pressed` / `just_released` only fire on the first tick.
    pub fn begin_tick(&mut self) {
        self.engine.begin_frame();
    }

    /// Read access to the engine-side input state.
    ///
    /// Game code reads this during update ticks.
    pub fn engine(&self) -> &InputState {
        &self.engine
    }

    /// Swap the engine-side buffer into an external `InputState`.
    ///
    /// Use this to move the current engine input into `Engine.input`
    /// without requiring `Clone`. After the call, `target` holds the
    /// engine's input and the internal engine buffer holds whatever
    /// `target` previously contained (typically stale state from the
    /// last tick — harmless, since `transfer` will overwrite it).
    pub fn swap_into(&mut self, target: &mut InputState) {
        std::mem::swap(&mut self.engine, target);
    }
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KeyCode, MouseButton};

    #[test]
    fn transfer_swaps_when_will_update() {
        let mut buf = InputBuffer::new();
        buf.shared_mut().key_pressed(KeyCode::Space);
        buf.shared_mut().mouse_button_pressed(MouseButton::Left);

        // No transfer yet — engine should be empty
        assert!(!buf.engine().is_key_just_pressed(KeyCode::Space));

        buf.transfer(true);

        assert!(buf.engine().is_key_just_pressed(KeyCode::Space));
        assert!(buf.engine().is_mouse_just_pressed(MouseButton::Left));
    }

    #[test]
    fn transfer_defers_when_no_update() {
        let mut buf = InputBuffer::new();
        buf.shared_mut().key_pressed(KeyCode::Space);

        buf.transfer(false);

        // Engine should still be empty — events stayed in shared
        assert!(!buf.engine().is_key_just_pressed(KeyCode::Space));
        assert!(!buf.engine().is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn deferred_events_survive() {
        let mut buf = InputBuffer::new();
        buf.shared_mut().key_pressed(KeyCode::Space);

        // Frame with no update — events stay in shared
        buf.transfer(false);
        assert!(!buf.engine().is_key_just_pressed(KeyCode::Space));

        // Next frame with update — events arrive
        buf.transfer(true);
        assert!(buf.engine().is_key_just_pressed(KeyCode::Space));
        assert!(buf.engine().is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn continuous_state_copied_to_shared() {
        let mut buf = InputBuffer::new();
        buf.shared_mut().mouse_moved(100.0, 200.0);
        buf.shared_mut().key_pressed(KeyCode::A);
        buf.shared_mut().touch_started(1, 50.0, 75.0);

        buf.transfer(true);

        // Engine has the events
        assert_eq!(buf.engine().mouse_position(), unison_core::Vec2::new(100.0, 200.0));
        assert!(buf.engine().is_key_pressed(KeyCode::A));
        assert_eq!(buf.engine().active_touches().len(), 1);

        // Shared buffer got continuous state back via copy_held_from
        assert_eq!(buf.shared.mouse_position(), unison_core::Vec2::new(100.0, 200.0));
        assert!(buf.shared.is_key_pressed(KeyCode::A));
        assert_eq!(buf.shared.active_touches().len(), 1);

        // But shared's per-frame flags are cleared
        assert!(!buf.shared.is_key_just_pressed(KeyCode::A));
    }

    #[test]
    fn begin_tick_clears_per_frame() {
        let mut buf = InputBuffer::new();
        buf.shared_mut().key_pressed(KeyCode::Space);
        buf.transfer(true);

        assert!(buf.engine().is_key_just_pressed(KeyCode::Space));

        buf.begin_tick();

        // just_pressed cleared, but key still held
        assert!(!buf.engine().is_key_just_pressed(KeyCode::Space));
        assert!(buf.engine().is_key_pressed(KeyCode::Space));
    }

    #[test]
    fn mouse_click_survives_deferred_frame() {
        let mut buf = InputBuffer::new();
        buf.shared_mut().mouse_moved(100.0, 50.0);
        buf.transfer(true); // establish mouse position

        // Press on a no-update frame
        buf.shared_mut().mouse_button_pressed(MouseButton::Left);
        buf.transfer(false);

        // Release also on a no-update frame
        buf.shared_mut().mouse_button_released(MouseButton::Left);
        buf.transfer(false);

        // Next update frame — both press and release arrive together
        buf.transfer(true);
        assert!(buf.engine().is_mouse_just_pressed(MouseButton::Left));
        assert!(buf.engine().is_mouse_just_released(MouseButton::Left));
        assert_eq!(buf.engine().mouse_position(), unison_core::Vec2::new(100.0, 50.0));
    }
}
