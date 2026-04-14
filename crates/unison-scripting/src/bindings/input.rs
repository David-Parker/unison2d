//! Input bindings — `unison.input` table refreshed each frame.
//!
//! ```lua
//! if unison.input.is_key_pressed("Space") then ... end
//! if unison.input.is_key_just_pressed("W") then ... end
//! if unison.input.is_mouse_button_just_pressed(0) then ... end
//! local touches = unison.input.touches_started()
//! ```

use std::cell::RefCell;

use mlua::prelude::*;
use unison2d::input::{InputState, KeyCode, MouseButton};

use super::action_map::{self, ActionBinding, AxisBinding};

thread_local! {
    /// Snapshot of the input state, refreshed each frame by `ScriptedGame::update`.
    static INPUT_STATE: RefCell<Option<InputStateSnapshot>> = const { RefCell::new(None) };
}

/// A minimal snapshot of the input state that can be held across Lua calls.
/// We copy the data we need rather than holding a reference to `InputState`.
pub struct InputStateSnapshot {
    pub keys_pressed: Vec<KeyCode>,
    pub keys_just_pressed: Vec<KeyCode>,
    pub keys_just_released: Vec<KeyCode>,
    pub touches_began: Vec<(f32, f32)>,
    pub active_touch: Option<(f32, f32)>,
    pub mouse_pos: (f32, f32),
    pub mouse_buttons_pressed: Vec<MouseButton>,
    pub mouse_buttons_just_pressed: Vec<MouseButton>,
    pub mouse_buttons_just_released: Vec<MouseButton>,
    /// Raw joystick axis: (x, y), each in -1.0..=1.0.
    pub joystick: (f32, f32),
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
        let keys_just_released: Vec<KeyCode> = all_keys.iter()
            .filter(|k| input.is_key_just_released(**k))
            .copied()
            .collect();
        let touches_began: Vec<(f32, f32)> = input.touches_just_began()
            .iter()
            .map(|t| (t.position.x, t.position.y))
            .collect();
        let active_touch = input.active_touches().first()
            .map(|t| (t.position.x, t.position.y));
        let mouse_pos_vec = input.mouse_position();

        // Capture all mouse button states
        let all_buttons = [MouseButton::Left, MouseButton::Right, MouseButton::Middle];
        let mouse_buttons_pressed: Vec<MouseButton> = all_buttons.iter()
            .filter(|b| input.is_mouse_pressed(**b))
            .copied()
            .collect();
        let mouse_buttons_just_pressed: Vec<MouseButton> = all_buttons.iter()
            .filter(|b| input.is_mouse_just_pressed(**b))
            .copied()
            .collect();
        let mouse_buttons_just_released: Vec<MouseButton> = all_buttons.iter()
            .filter(|b| input.is_mouse_just_released(**b))
            .copied()
            .collect();

        let axis_vec = input.axis();
        Self {
            keys_pressed,
            keys_just_pressed,
            keys_just_released,
            touches_began,
            active_touch,
            mouse_pos: (mouse_pos_vec.x, mouse_pos_vec.y),
            mouse_buttons_pressed,
            mouse_buttons_just_pressed,
            mouse_buttons_just_released,
            joystick: (axis_vec.x, axis_vec.y),
        }
    }
}

/// Call `f` with an optional reference to the current input snapshot.
pub(crate) fn with_snapshot<R>(f: impl FnOnce(Option<&InputStateSnapshot>) -> R) -> R {
    INPUT_STATE.with(|cell| f(cell.borrow().as_ref()))
}

/// Update the thread-local input snapshot. Called each frame by `ScriptedGame::update`.
pub fn refresh(input: &InputState) {
    INPUT_STATE.with(|cell| {
        *cell.borrow_mut() = Some(InputStateSnapshot::capture(input));
    });
}

/// Populate `unison.input` on the given `unison` table.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
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

    // input.is_key_just_released("W") → bool
    input.set("is_key_just_released", lua.create_function(|_, name: String| {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            match &*snap {
                Some(s) => match parse_key_code(&name) {
                    Some(k) => Ok(s.keys_just_released.contains(&k)),
                    None => Ok(false),
                },
                None => Ok(false),
            }
        })
    })?)?;

    // input.touches_started() → array of {x, y} tables
    input.set("touches_started", lua.create_function(|lua, ()| {
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

    // input.is_mouse_button_pressed(button: int) → bool (0=Left, 1=Right, 2=Middle)
    input.set("is_mouse_button_pressed", lua.create_function(|_, button: i32| {
        let mb = match button {
            0 => MouseButton::Left,
            1 => MouseButton::Right,
            2 => MouseButton::Middle,
            _ => return Ok(false),
        };
        INPUT_STATE.with(|cell| {
            Ok(cell.borrow().as_ref().is_some_and(|s| s.mouse_buttons_pressed.contains(&mb)))
        })
    })?)?;

    // input.is_mouse_button_just_pressed(button: int) → bool (0=Left, 1=Right, 2=Middle)
    input.set("is_mouse_button_just_pressed", lua.create_function(|_, button: i32| {
        let mb = match button {
            0 => MouseButton::Left,
            1 => MouseButton::Right,
            2 => MouseButton::Middle,
            _ => return Ok(false),
        };
        INPUT_STATE.with(|cell| {
            Ok(cell.borrow().as_ref().is_some_and(|s| s.mouse_buttons_just_pressed.contains(&mb)))
        })
    })?)?;

    // input.is_mouse_button_just_released(button: int) → bool (0=Left, 1=Right, 2=Middle)
    input.set("is_mouse_button_just_released", lua.create_function(|_, button: i32| {
        let mb = match button {
            0 => MouseButton::Left,
            1 => MouseButton::Right,
            2 => MouseButton::Middle,
            _ => return Ok(false),
        };
        INPUT_STATE.with(|cell| {
            Ok(cell.borrow().as_ref().is_some_and(|s| s.mouse_buttons_just_released.contains(&mb)))
        })
    })?)?;

    // input.mouse_position() → x, y (screen-space)
    input.set("mouse_position", lua.create_function(|_, ()| {
        INPUT_STATE.with(|cell| {
            Ok(cell.borrow().as_ref().map(|s| s.mouse_pos).unwrap_or((0.0, 0.0)))
        })
    })?)?;

    // input.is_pointer_just_pressed() → bool
    // Unified cross-platform "tap/click" detector: returns true if a touch began
    // this frame OR the primary mouse button was just pressed.
    input.set("is_pointer_just_pressed", lua.create_function(|_, ()| {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            let Some(s) = snap.as_ref() else {
                return Ok(false);
            };
            if !s.touches_began.is_empty() {
                return Ok(true);
            }
            Ok(s.mouse_buttons_just_pressed.contains(&MouseButton::Left))
        })
    })?)?;

    // input.pointer_position() → (x, y) or (nil, nil)
    // Returns the position of an active touch OR the mouse position if the
    // primary button is currently held. Returns (nil, nil) when no pointer is active.
    input.set("pointer_position", lua.create_function(|_, ()| {
        INPUT_STATE.with(|cell| {
            let snap = cell.borrow();
            let Some(s) = snap.as_ref() else {
                return Ok((None::<f32>, None::<f32>));
            };
            if let Some((x, y)) = s.active_touch {
                return Ok((Some(x), Some(y)));
            }
            if s.mouse_buttons_pressed.contains(&MouseButton::Left) {
                return Ok((Some(s.mouse_pos.0), Some(s.mouse_pos.1)));
            }
            Ok((None, None))
        })
    })?)?;

    // ---------------------------------------------------------------
    // Action map
    // ---------------------------------------------------------------

    // input.bind_action(name, { keys = {...}, mouse_buttons = {...} })
    input.set("bind_action", lua.create_function(|_, (name, opts): (String, LuaTable)| {
        let mut binding = ActionBinding::default();
        if let Ok(keys_tbl) = opts.get::<LuaTable>("keys") {
            for pair in keys_tbl.sequence_values::<String>() {
                let k = pair?;
                if let Some(kc) = parse_key_code(&k) {
                    binding.keys.push(kc);
                }
            }
        }
        if let Ok(mb_tbl) = opts.get::<LuaTable>("mouse_buttons") {
            for pair in mb_tbl.sequence_values::<i32>() {
                let b = pair?;
                let mb = match b {
                    0 => MouseButton::Left,
                    1 => MouseButton::Right,
                    2 => MouseButton::Middle,
                    _ => continue,
                };
                binding.mouse_buttons.push(mb);
            }
        }
        action_map::bind_action(&name, binding);
        Ok(())
    })?)?;

    // input.bind_axis(name, { negative = "action", positive = "action", joystick_axis = "x"|"y" })
    input.set("bind_axis", lua.create_function(|_, (name, opts): (String, LuaTable)| {
        let binding = AxisBinding {
            negative: opts.get::<String>("negative").ok(),
            positive: opts.get::<String>("positive").ok(),
            joystick_axis: match opts.get::<String>("joystick_axis").ok().as_deref() {
                Some("x") | Some("X") => Some(0),
                Some("y") | Some("Y") => Some(1),
                _ => None,
            },
        };
        action_map::bind_axis(&name, binding);
        Ok(())
    })?)?;

    // input.is_action_pressed(name) → bool
    input.set("is_action_pressed", lua.create_function(|_, name: String| {
        Ok(action_map::is_action_pressed(&name))
    })?)?;

    // input.is_action_just_pressed(name) → bool
    input.set("is_action_just_pressed", lua.create_function(|_, name: String| {
        Ok(action_map::is_action_just_pressed(&name))
    })?)?;

    // input.is_action_just_released(name) → bool
    input.set("is_action_just_released", lua.create_function(|_, name: String| {
        Ok(action_map::is_action_just_released(&name))
    })?)?;

    // input.axis(name) → number
    input.set("axis", lua.create_function(|_, name: String| {
        Ok(action_map::axis(&name))
    })?)?;

    unison.set("input", input)?;
    Ok(())
}

/// Parse a key name string into a KeyCode variant.
pub(crate) fn parse_key_code(name: &str) -> Option<KeyCode> {
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
