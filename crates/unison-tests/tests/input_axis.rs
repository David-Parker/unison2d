//! Integration tests for axis input through the InputBuffer double-buffer lifecycle.
//!
//! Verifies that axis values (e.g., from a virtual joystick) flow correctly
//! through the shared→engine buffer swap, survive deferred frames, and
//! coexist with other input types like touch.

use unison_input::{InputBuffer, InputState};
use unison_core::Vec2;

/// Set axis on shared, transfer, verify engine reads it.
#[test]
fn axis_flows_through_input_buffer() {
    let mut buf = InputBuffer::new();
    buf.shared_mut().set_axis(0.75, 0.0);

    buf.transfer(true);

    let mut engine = InputState::new();
    buf.swap_into(&mut engine);
    assert_eq!(engine.axis(), Vec2::new(0.75, 0.0));
}

/// Set axis, transfer with will_update=false, verify it persists until next transfer.
#[test]
fn axis_survives_deferred_frame() {
    let mut buf = InputBuffer::new();
    buf.shared_mut().set_axis(-0.5, 0.0);

    // No-update frame — axis stays in shared
    buf.transfer(false);
    assert_eq!(buf.engine().axis(), Vec2::ZERO);

    // Next frame with update — axis arrives
    buf.transfer(true);

    let mut engine = InputState::new();
    buf.swap_into(&mut engine);
    assert_eq!(engine.axis(), Vec2::new(-0.5, 0.0));
}

/// Set axis, then set to zero, verify after transfer.
#[test]
fn axis_resets_when_cleared() {
    let mut buf = InputBuffer::new();
    let mut engine = InputState::new();

    // Set axis
    buf.shared_mut().set_axis(1.0, 0.0);
    buf.transfer(true);
    buf.swap_into(&mut engine);
    assert_eq!(engine.axis(), Vec2::new(1.0, 0.0));

    // Clear axis
    buf.shared_mut().set_axis(0.0, 0.0);
    buf.transfer(true);
    buf.swap_into(&mut engine);
    assert_eq!(engine.axis(), Vec2::ZERO);
}

/// Verify copy_held_from carries axis across buffer swaps (continuous state).
#[test]
fn axis_copied_on_transfer() {
    let mut buf = InputBuffer::new();
    let mut engine = InputState::new();

    buf.shared_mut().set_axis(0.6, 0.0);
    buf.transfer(true);
    buf.swap_into(&mut engine);
    assert_eq!(engine.axis(), Vec2::new(0.6, 0.0));

    // Don't change axis — it should persist through the next transfer
    // because copy_held_from copies continuous state back to shared.
    buf.transfer(true);
    buf.swap_into(&mut engine);
    assert_eq!(engine.axis(), Vec2::new(0.6, 0.0));
}

/// Set axis AND start a touch simultaneously, verify both are independently readable.
#[test]
fn axis_coexists_with_touch() {
    let mut buf = InputBuffer::new();
    let mut engine = InputState::new();

    buf.shared_mut().set_axis(-0.8, 0.0);
    buf.shared_mut().touch_started(1, 300.0, 200.0);

    buf.transfer(true);
    buf.swap_into(&mut engine);

    // Both axis and touch should be present
    assert_eq!(engine.axis(), Vec2::new(-0.8, 0.0));
    assert_eq!(engine.active_touches().len(), 1);
    assert_eq!(engine.touches_just_began().len(), 1);
    assert_eq!(engine.touches_just_began()[0].id, 1);
}
