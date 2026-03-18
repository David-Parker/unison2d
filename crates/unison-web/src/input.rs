//! DOM event wiring — maps browser events into InputState

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{KeyboardEvent, MouseEvent, TouchEvent, HtmlCanvasElement};
use unison_input::{InputState, KeyCode, MouseButton};

/// Wire DOM keyboard, mouse, and touch events into the given InputState.
/// Returns a list of closures that must be kept alive for the duration of the game.
pub fn wire_input(
    canvas: &HtmlCanvasElement,
    input: Rc<RefCell<InputState>>,
) -> Vec<Closure<dyn FnMut(web_sys::Event)>> {
    let window = web_sys::window().expect("no window");
    let mut closures: Vec<Closure<dyn FnMut(web_sys::Event)>> = Vec::new();

    // Keyboard events (on window so we get them even without canvas focus)
    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: KeyboardEvent = event.unchecked_into();
            if event.repeat() {
                return;
            }
            if let Some(key) = map_key_code(&event.code()) {
                input.borrow_mut().key_pressed(key);
                // Prevent scrolling for game keys
                if is_game_key(key) {
                    event.prevent_default();
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        window
            .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
            .expect("keydown listener");
        closures.push(closure);
    }

    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: KeyboardEvent = event.unchecked_into();
            if let Some(key) = map_key_code(&event.code()) {
                input.borrow_mut().key_released(key);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        window
            .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())
            .expect("keyup listener");
        closures.push(closure);
    }

    // Mouse events (on canvas)
    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: MouseEvent = event.unchecked_into();
            input
                .borrow_mut()
                .mouse_moved(event.offset_x() as f32, event.offset_y() as f32);
        }) as Box<dyn FnMut(web_sys::Event)>);
        canvas
            .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
            .expect("mousemove listener");
        closures.push(closure);
    }

    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: MouseEvent = event.unchecked_into();
            if let Some(btn) = map_mouse_button(event.button()) {
                input.borrow_mut().mouse_button_pressed(btn);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        canvas
            .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
            .expect("mousedown listener");
        closures.push(closure);
    }

    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: MouseEvent = event.unchecked_into();
            if let Some(btn) = map_mouse_button(event.button()) {
                input.borrow_mut().mouse_button_released(btn);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        canvas
            .add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())
            .expect("mouseup listener");
        closures.push(closure);
    }

    // Touch events (on canvas)
    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: TouchEvent = event.unchecked_into();
            event.prevent_default();
            let touches = event.changed_touches();
            for i in 0..touches.length() {
                if let Some(touch) = touches.get(i) {
                    input.borrow_mut().touch_started(
                        touch.identifier() as u64,
                        touch.client_x() as f32,
                        touch.client_y() as f32,
                    );
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        canvas
            .add_event_listener_with_callback("touchstart", closure.as_ref().unchecked_ref())
            .expect("touchstart listener");
        closures.push(closure);
    }

    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: TouchEvent = event.unchecked_into();
            event.prevent_default();
            let touches = event.changed_touches();
            for i in 0..touches.length() {
                if let Some(touch) = touches.get(i) {
                    input.borrow_mut().touch_moved(
                        touch.identifier() as u64,
                        touch.client_x() as f32,
                        touch.client_y() as f32,
                    );
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        canvas
            .add_event_listener_with_callback("touchmove", closure.as_ref().unchecked_ref())
            .expect("touchmove listener");
        closures.push(closure);
    }

    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: TouchEvent = event.unchecked_into();
            event.prevent_default();
            let touches = event.changed_touches();
            for i in 0..touches.length() {
                if let Some(touch) = touches.get(i) {
                    input
                        .borrow_mut()
                        .touch_ended(touch.identifier() as u64);
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        canvas
            .add_event_listener_with_callback("touchend", closure.as_ref().unchecked_ref())
            .expect("touchend listener");
        closures.push(closure);
    }

    {
        let input = input.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            let event: TouchEvent = event.unchecked_into();
            event.prevent_default();
            let touches = event.changed_touches();
            for i in 0..touches.length() {
                if let Some(touch) = touches.get(i) {
                    input
                        .borrow_mut()
                        .touch_cancelled(touch.identifier() as u64);
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        canvas
            .add_event_listener_with_callback("touchcancel", closure.as_ref().unchecked_ref())
            .expect("touchcancel listener");
        closures.push(closure);
    }

    closures
}

/// Map DOM KeyboardEvent.code to engine KeyCode
fn map_key_code(code: &str) -> Option<KeyCode> {
    Some(match code {
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "Space" => KeyCode::Space,
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Escape,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "KeyA" => KeyCode::A,
        "KeyB" => KeyCode::B,
        "KeyC" => KeyCode::C,
        "KeyD" => KeyCode::D,
        "KeyE" => KeyCode::E,
        "KeyF" => KeyCode::F,
        "KeyG" => KeyCode::G,
        "KeyH" => KeyCode::H,
        "KeyI" => KeyCode::I,
        "KeyJ" => KeyCode::J,
        "KeyK" => KeyCode::K,
        "KeyL" => KeyCode::L,
        "KeyM" => KeyCode::M,
        "KeyN" => KeyCode::N,
        "KeyO" => KeyCode::O,
        "KeyP" => KeyCode::P,
        "KeyQ" => KeyCode::Q,
        "KeyR" => KeyCode::R,
        "KeyS" => KeyCode::S,
        "KeyT" => KeyCode::T,
        "KeyU" => KeyCode::U,
        "KeyV" => KeyCode::V,
        "KeyW" => KeyCode::W,
        "KeyX" => KeyCode::X,
        "KeyY" => KeyCode::Y,
        "KeyZ" => KeyCode::Z,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        _ => return None,
    })
}

/// Map DOM MouseEvent.button to engine MouseButton
fn map_mouse_button(button: i16) -> Option<MouseButton> {
    match button {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Middle),
        2 => Some(MouseButton::Right),
        _ => None,
    }
}

/// Keys that should prevent default browser behavior (scrolling, etc.)
fn is_game_key(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::Space
            | KeyCode::Tab
    )
}
