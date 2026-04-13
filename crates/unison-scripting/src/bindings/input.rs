//! Input bindings — global `input` table refreshed each frame.
//!
//! ```lua
//! if input.is_key_pressed("Space") then ... end
//! if input.is_key_just_pressed("W") then ... end
//! local ax, ay = input.axis()
//! local touches = input.touches_just_began()
//! ```

use std::cell::RefCell;

use mlua::prelude::*;
use unison2d::input::{InputState, KeyCode, MouseButton};

thread_local! {
    /// Snapshot of the input state, refreshed each frame by `ScriptedGame::update`.
    static INPUT_STATE: RefCell<Option<InputStateSnapshot>> = const { RefCell::new(None) };
}

/// A minimal snapshot of the input state that can be held across Lua calls.
/// We copy the data we need rather than holding a reference to `InputState`.
pub struct InputStateSnapshot {
    pub keys_pressed: Vec<KeyCode>,
    pub keys_just_pressed: Vec<KeyCode>,
    pub axis: (f32, f32),
    pub touches_began: Vec<(f32, f32)>,
    pub active_touch: Option<(f32, f32)>,
    pub mouse_pos: (f32, f32),
    pub mouse_left_just_pressed: bool,
    pub mouse_left_pressed: bool,
}

impl InputStateSnapshot {
    pub fn capture(input: &InputState) -> Self {
        // Build list of pressed keys by testing all known variants.
        let all_keys = all_key_codes();
        let keys_pressed: Vec<KeyCode> = all_keys.iter()
            .filter(|k| input.is_key_pressed(**k))
            .copied()
            .collect();
        let keys_just_pressed: Vec<KeyCode> = all_keys.iter()
            .filter(|k| input.is_key_just_pressed(**k))
            .copied()
            .collect();
        let axis_vec = input.axis();
        let touches_began: Vec<(f32, f32)> = input.touches_just_began()
            .iter()
            .map(|t| (t.position.x, t.position.y))
            .collect();
        let active_touch = input.active_touches().first()
            .map(|t| (t.position.x, t.position.y));
        let mouse_pos_vec = input.mouse_position();

        Self {
            keys_pressed,
            keys_just_pressed,
            axis: (axis_vec.x, axis_vec.y),
            touches_began,
            active_touch,
            mouse_pos: (mouse_pos_vec.x, mouse_pos_vec.y),
            mouse_left_just_pressed: input.is_mouse_just_pressed(MouseButton::Left),
            mouse_left_pressed: input.is_mouse_pressed(MouseButton::Left),
        }
    }
}

/// Update the thread-local input snapshot. Called each frame by `ScriptedGame::update`.
pub fn refresh(input: &InputState) {
    INPUT_STATE.with(|cell| {
        *cell.borrow_mut() = Some(InputStateSnapshot::capture(input));
    });
}

/// Register the `input` global table.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let input = lua.create_table()?;

    // input.is_key_pressed("Space") → bool
    input.set("is_key_pressed", lua.create_function(|_, name: String| {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            match &*snap {
                Some(s) => match parse_key_code(&name) {
                    Some(k) => Ok(s.keys_pressed.contains(&k)),
                    None => Ok(false),
                },
                None => Ok(false),
            }
        })
    })?)?;

    // input.is_key_just_pressed("W") → bool
    input.set("is_key_just_pressed", lua.create_function(|_, name: String| {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            match &*snap {
                Some(s) => match parse_key_code(&name) {
                    Some(k) => Ok(s.keys_just_pressed.contains(&k)),
                    None => Ok(false),
                },
                None => Ok(false),
            }
        })
    })?)?;

    // input.axis_x() → f32
    input.set("axis_x", lua.create_function(|_, ()| {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            Ok(snap.as_ref().map(|s| s.axis.0).unwrap_or(0.0))
        })
    })?)?;

    // input.axis_y() → f32
    input.set("axis_y", lua.create_function(|_, ()| {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            Ok(snap.as_ref().map(|s| s.axis.1).unwrap_or(0.0))
        })
    })?)?;

    // input.touches_just_began() → array of {x, y} tables
    input.set("touches_just_began", lua.create_function(|lua, ()| {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            let touches = snap.as_ref().map(|s| &s.touches_began[..]).unwrap_or(&[]);
            let table = lua.create_table()?;
            for (i, &(x, y)) in touches.iter().enumerate() {
                let t = lua.create_table()?;
                t.set("x", x)?;
                t.set("y", y)?;
                table.set(i + 1, t)?;
            }
            Ok(table)
        })
    })?)?;

    // input.is_mouse_just_pressed() → bool (left button, this frame)
    input.set("is_mouse_just_pressed", lua.create_function(|_, ()| {
        INPUT_STATE.with(|cell| {
            Ok(cell.borrow().as_ref().is_some_and(|s| s.mouse_left_just_pressed))
        })
    })?)?;

    // input.mouse_position() → x, y (screen-space)
    input.set("mouse_position", lua.create_function(|_, ()| {
        INPUT_STATE.with(|cell| {
            Ok(cell.borrow().as_ref().map(|s| s.mouse_pos).unwrap_or((0.0, 0.0)))
        })
    })?)?;

    // input.pointer_just_pressed() → x, y or nil, nil
    // Unified cross-platform "tap/click": returns the position of a touch that
    // began this frame, OR the mouse position if the primary button was just
    // pressed. Returns `nil, nil` when neither is active.
    input.set("pointer_just_pressed", lua.create_function(|_, ()| -> LuaResult<(Option<f32>, Option<f32>)> {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            let Some(s) = snap.as_ref() else {
                return Ok((None, None));
            };
            if let Some(&(x, y)) = s.touches_began.first() {
                return Ok((Some(x), Some(y)));
            }
            if s.mouse_left_just_pressed {
                return Ok((Some(s.mouse_pos.0), Some(s.mouse_pos.1)));
            }
            Ok((None, None))
        })
    })?)?;

    // input.pointer_position() → x, y or nil, nil
    // "While held" counterpart to `pointer_just_pressed`: returns the position
    // of an active touch OR the mouse position if the primary button is
    // currently held. Returns `nil, nil` when no pointer is active.
    input.set("pointer_position", lua.create_function(|_, ()| -> LuaResult<(Option<f32>, Option<f32>)> {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            let Some(s) = snap.as_ref() else {
                return Ok((None, None));
            };
            if let Some((x, y)) = s.active_touch {
                return Ok((Some(x), Some(y)));
            }
            if s.mouse_left_pressed {
                return Ok((Some(s.mouse_pos.0), Some(s.mouse_pos.1)));
            }
            Ok((None, None))
        })
    })?)?;

    lua.globals().set("input", input)?;
    Ok(())
}

/// Parse a key name string into a KeyCode variant.
fn parse_key_code(name: &str) -> Option<KeyCode> {
    match name {
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "Space" => Some(KeyCode::Space),
        "Enter" => Some(KeyCode::Enter),
        "Escape" => Some(KeyCode::Escape),
        "Tab" => Some(KeyCode::Tab),
        "Backspace" => Some(KeyCode::Backspace),
        "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" => Some(KeyCode::ControlRight),
        "AltLeft" => Some(KeyCode::AltLeft),
        "AltRight" => Some(KeyCode::AltRight),
        // Single letter A-Z
        s if s.len() == 1 => {
            let ch = s.chars().next()?;
            match ch.to_ascii_uppercase() {
                'A' => Some(KeyCode::A), 'B' => Some(KeyCode::B),
                'C' => Some(KeyCode::C), 'D' => Some(KeyCode::D),
                'E' => Some(KeyCode::E), 'F' => Some(KeyCode::F),
                'G' => Some(KeyCode::G), 'H' => Some(KeyCode::H),
                'I' => Some(KeyCode::I), 'J' => Some(KeyCode::J),
                'K' => Some(KeyCode::K), 'L' => Some(KeyCode::L),
                'M' => Some(KeyCode::M), 'N' => Some(KeyCode::N),
                'O' => Some(KeyCode::O), 'P' => Some(KeyCode::P),
                'Q' => Some(KeyCode::Q), 'R' => Some(KeyCode::R),
                'S' => Some(KeyCode::S), 'T' => Some(KeyCode::T),
                'U' => Some(KeyCode::U), 'V' => Some(KeyCode::V),
                'W' => Some(KeyCode::W), 'X' => Some(KeyCode::X),
                'Y' => Some(KeyCode::Y), 'Z' => Some(KeyCode::Z),
                _ => None,
            }
        }
        // Digit0..Digit9
        "Digit0" | "0" => Some(KeyCode::Digit0),
        "Digit1" | "1" => Some(KeyCode::Digit1),
        "Digit2" | "2" => Some(KeyCode::Digit2),
        "Digit3" | "3" => Some(KeyCode::Digit3),
        "Digit4" | "4" => Some(KeyCode::Digit4),
        "Digit5" | "5" => Some(KeyCode::Digit5),
        "Digit6" | "6" => Some(KeyCode::Digit6),
        "Digit7" | "7" => Some(KeyCode::Digit7),
        "Digit8" | "8" => Some(KeyCode::Digit8),
        "Digit9" | "9" => Some(KeyCode::Digit9),
        _ => None,
    }
}

/// All known KeyCode variants (for snapshot iteration).
fn all_key_codes() -> &'static [KeyCode] {
    &[
        KeyCode::ArrowUp, KeyCode::ArrowDown, KeyCode::ArrowLeft, KeyCode::ArrowRight,
        KeyCode::A, KeyCode::B, KeyCode::C, KeyCode::D, KeyCode::E, KeyCode::F,
        KeyCode::G, KeyCode::H, KeyCode::I, KeyCode::J, KeyCode::K, KeyCode::L,
        KeyCode::M, KeyCode::N, KeyCode::O, KeyCode::P, KeyCode::Q, KeyCode::R,
        KeyCode::S, KeyCode::T, KeyCode::U, KeyCode::V, KeyCode::W, KeyCode::X,
        KeyCode::Y, KeyCode::Z,
        KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4,
        KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
        KeyCode::Space, KeyCode::Enter, KeyCode::Escape, KeyCode::Tab, KeyCode::Backspace,
        KeyCode::ShiftLeft, KeyCode::ShiftRight,
        KeyCode::ControlLeft, KeyCode::ControlRight,
        KeyCode::AltLeft, KeyCode::AltRight,
    ]
}
