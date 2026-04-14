//! Lua-facing action map — `unison.input.bind_action` / `bind_axis` / queries.
//!
//! State is thread-local, matching the pattern in `bindings/input.rs`.

use std::cell::RefCell;
use std::collections::HashMap;

use unison2d::input::{KeyCode, MouseButton};

// ===================================================================
// Types
// ===================================================================

/// A named action binding: set of keys and mouse buttons that trigger it.
#[derive(Default, Clone)]
pub struct ActionBinding {
    pub keys: Vec<KeyCode>,
    pub mouse_buttons: Vec<MouseButton>,
}

/// A named axis binding: optional negative/positive actions plus an optional joystick axis.
#[derive(Default, Clone)]
pub struct AxisBinding {
    pub negative: Option<String>,
    pub positive: Option<String>,
    /// 0 = X axis, 1 = Y axis
    pub joystick_axis: Option<u8>,
}

// ===================================================================
// Thread-local storage
// ===================================================================

thread_local! {
    static ACTIONS: RefCell<HashMap<String, ActionBinding>> = RefCell::new(HashMap::new());
    static AXES: RefCell<HashMap<String, AxisBinding>> = RefCell::new(HashMap::new());
}

// ===================================================================
// Mutation
// ===================================================================

/// Register (or replace) a named action binding.
pub fn bind_action(name: &str, binding: ActionBinding) {
    ACTIONS.with(|cell| cell.borrow_mut().insert(name.to_owned(), binding));
}

/// Register (or replace) a named axis binding.
pub fn bind_axis(name: &str, binding: AxisBinding) {
    AXES.with(|cell| cell.borrow_mut().insert(name.to_owned(), binding));
}

// ===================================================================
// Queries
// ===================================================================

/// True while any key or mouse button bound to `name` is held.
pub fn is_action_pressed(name: &str) -> bool {
    let binding = ACTIONS.with(|cell| cell.borrow().get(name).cloned());
    let Some(b) = binding else { return false };
    super::input::with_snapshot(|snap| {
        let Some(s) = snap else { return false };
        b.keys.iter().any(|k| s.keys_pressed.contains(k))
            || b.mouse_buttons.iter().any(|mb| s.mouse_buttons_pressed.contains(mb))
    })
}

/// True only on the frame the action was first triggered (any bound input just pressed).
pub fn is_action_just_pressed(name: &str) -> bool {
    let binding = ACTIONS.with(|cell| cell.borrow().get(name).cloned());
    let Some(b) = binding else { return false };
    super::input::with_snapshot(|snap| {
        let Some(s) = snap else { return false };
        b.keys.iter().any(|k| s.keys_just_pressed.contains(k))
            || b.mouse_buttons.iter().any(|mb| s.mouse_buttons_just_pressed.contains(mb))
    })
}

/// True only on the frame all bound inputs were released.
pub fn is_action_just_released(name: &str) -> bool {
    let binding = ACTIONS.with(|cell| cell.borrow().get(name).cloned());
    let Some(b) = binding else { return false };
    super::input::with_snapshot(|snap| {
        let Some(s) = snap else { return false };
        b.keys.iter().any(|k| s.keys_just_released.contains(k))
            || b.mouse_buttons.iter().any(|mb| s.mouse_buttons_just_released.contains(mb))
    })
}

/// Digital axis value in [-1, 1] from negative/positive actions, plus raw joystick if bound.
/// No clamping — joystick and digital contributions are summed.
pub fn axis(name: &str) -> f32 {
    let binding = AXES.with(|cell| cell.borrow().get(name).cloned());
    let Some(b) = binding else { return 0.0 };

    let pos = b.positive.as_deref().map(is_action_pressed).unwrap_or(false);
    let neg = b.negative.as_deref().map(is_action_pressed).unwrap_or(false);
    let digital = pos as i32 as f32 - neg as i32 as f32;

    let joy = b.joystick_axis.map(|ax| {
        super::input::with_snapshot(|snap| {
            snap.map(|s| match ax {
                0 => s.joystick.0,
                1 => s.joystick.1,
                _ => 0.0,
            }).unwrap_or(0.0)
        })
    }).unwrap_or(0.0);

    digital + joy
}

// ===================================================================
// Lifecycle
// ===================================================================

/// Clear all bindings. Called from `engine_state::reset`.
pub fn reset() {
    ACTIONS.with(|cell| cell.borrow_mut().clear());
    AXES.with(|cell| cell.borrow_mut().clear());
}
