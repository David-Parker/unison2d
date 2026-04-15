//! Unison Web — Web platform crate for the Unison 2D engine.
//!
//! Provides:
//! - WebGL2 renderer (implements the `Renderer` trait)
//! - DOM input wiring (keyboard, mouse, touch → `InputState`)
//! - `requestAnimationFrame` game loop with fixed timestep
//!
//! # Usage
//!
//! Game code is authored in Lua (or TypeScript → Lua via TSTL) using
//! `unison_scripting::ScriptedGame`. The macro wires everything up:
//!
//! ```ignore
//! // project/lib.rs
//! unison_scripting::scripted_game_entry!("scripts/main.lua", assets::ASSETS);
//! ```
//!
//! See `docs/scripting/getting-started/lua.md` for the full scripting guide.

mod renderer;
mod shaders;
mod input;
mod game_loop;

pub use renderer::WebGlRenderer;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::WebGl2RenderingContext as GL;
use unison2d::{Engine, Game};
use unison_input::InputBuffer;
use unison_render::Renderer;

/// Run a game on the web platform.
///
/// This is the main entry point. It:
/// 1. Gets the canvas and WebGL2 context
/// 2. Creates the renderer, input state, and engine
/// 3. Wires DOM events into the input system
/// 4. Starts the requestAnimationFrame game loop
///
/// Call this from your `#[wasm_bindgen(start)]` function.
pub fn run<G: Game + 'static>(game: G) {
    // Get canvas and WebGL2 context
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let canvas = document
        .get_element_by_id("canvas")
        .expect("no canvas element with id 'canvas'")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("element is not a canvas");

    // Scale canvas buffer to device pixel ratio for sharp rendering
    let dpr = window.device_pixel_ratio() as f32;
    let css_width = canvas.client_width() as f32;
    let css_height = canvas.client_height() as f32;
    let width = (css_width * dpr).round();
    let height = (css_height * dpr).round();
    canvas.set_width(width as u32);
    canvas.set_height(height as u32);

    let context_options = js_sys::Object::new();
    js_sys::Reflect::set(&context_options, &"antialias".into(), &true.into())
        .expect("failed to set antialias");

    let gl: GL = canvas
        .get_context_with_context_options("webgl2", &context_options)
        .expect("getContext failed")
        .expect("WebGL2 not supported")
        .dyn_into::<GL>()
        .expect("not a WebGL2 context");

    // Create renderer (physical pixels for GPU buffer, logical/CSS pixels for UI/input)
    let mut web_renderer =
        WebGlRenderer::new(gl, width, height, css_width, css_height, dpr)
            .expect("Failed to create WebGL renderer");
    web_renderer.init().expect("Failed to init renderer");

    // Set up profiler time function (returns milliseconds)
    unison_profiler::set_time_fn(|| {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0)
    });
    unison_profiler::Profiler::set_enabled(true);

    // Create shared input buffer
    let input = Rc::new(RefCell::new(InputBuffer::new()));

    // Wire DOM events
    let _closures = self::input::wire_input(&canvas, input.clone());
    // Leak closures so they live forever (the game loop never ends)
    std::mem::forget(_closures);

    // Create engine with renderer
    let mut engine = Engine::new();
    engine.renderer = Some(Box::new(web_renderer));

    // Defer audio init until first user gesture (browser autoplay policy).
    // The audio system starts unarmed; a one-shot listener below arms it
    // on the first keydown / mousedown / touchstart.
    engine.audio.unarm_for_web();

    let engine = Rc::new(RefCell::new(engine));

    // Install a one-shot gesture listener to arm the audio system.
    install_audio_arm_listener(engine.clone());

    // Start game loop
    game_loop::start_loop(game, engine, input);
}

/// Install listeners on `window` that call `engine.audio.arm()` on the first
/// user gesture (keydown, mousedown, or touchstart). Required because browsers
/// block audio playback until a user interaction has occurred.
///
/// The closure is leaked via `.forget()` — it lives for the page lifetime,
/// matching the engine and game loop.
fn install_audio_arm_listener(engine: Rc<RefCell<Engine>>) {
    let armed = Rc::new(Cell::new(false));
    let handler = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        if armed.get() {
            return;
        }
        armed.set(true);
        engine.borrow_mut().audio.arm();
    }) as Box<dyn FnMut(web_sys::Event)>);

    let window = web_sys::window().expect("no window");
    for event in &["keydown", "mousedown", "touchstart"] {
        window
            .add_event_listener_with_callback(event, handler.as_ref().unchecked_ref())
            .expect("audio arm listener");
    }
    // Leak the closure — it must live for the page lifetime.
    handler.forget();
}
