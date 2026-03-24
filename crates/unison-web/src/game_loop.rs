//! requestAnimationFrame game loop with fixed timestep

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use unison2d::{Engine, Game};
use unison_input::InputBuffer;
use unison_profiler::Profiler;

/// How often (in frames) to log profiler stats to the console
const PROFILER_LOG_INTERVAL: u64 = 120;

/// Fixed timestep: 60 updates per second
const FIXED_DT: f32 = 1.0 / 60.0;
/// Max accumulated time per frame (prevents spiral of death)
const MAX_ACCUMULATOR: f32 = 0.1;

/// Start the requestAnimationFrame loop.
///
/// Takes ownership of the game and engine. The loop runs until the page is closed.
pub fn start_loop<G: Game + 'static>(
    mut game: G,
    engine: Rc<RefCell<Engine<G::Action>>>,
    input: Rc<RefCell<InputBuffer>>,
) {
    let f: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    let mut accumulator: f32 = 0.0;
    let mut last_time: Option<f64> = None;

    // Initialize the game
    {
        let mut engine_ref = engine.borrow_mut();
        engine_ref.fixed_dt = FIXED_DT;
        game.init(&mut engine_ref);
    }

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp: f64| {
        // Begin profiler frame
        Profiler::begin_frame();

        // Calculate delta time
        let dt = if let Some(prev) = last_time {
            ((timestamp - prev) / 1000.0) as f32
        } else {
            FIXED_DT
        };
        last_time = Some(timestamp);

        // Clamp accumulator to prevent spiral of death
        accumulator += dt.min(MAX_ACCUMULATOR);

        // Transfer DOM events into the engine.
        // InputBuffer only swaps when an update tick will run, so per-frame
        // events (just_pressed / just_released) never get discarded unprocessed.
        let will_update = accumulator >= FIXED_DT;
        if will_update {
            let mut input_ref = input.borrow_mut();
            input_ref.transfer(true);
            let mut engine_ref = engine.borrow_mut();
            input_ref.swap_into(&mut engine_ref.input);
        }

        // Fixed timestep updates
        let mut first_tick = true;
        while accumulator >= FIXED_DT {
            {
                let mut engine_ref = engine.borrow_mut();
                if !first_tick {
                    engine_ref.input.begin_frame();
                }
                first_tick = false;
                engine_ref.pre_update();
                game.update(&mut engine_ref);
            }
            accumulator -= FIXED_DT;
        }

        // Render once per frame — the game controls all rendering
        {
            let mut engine_ref = engine.borrow_mut();
            game.render(&mut engine_ref);
        }

        // End profiler frame and periodically log stats
        Profiler::end_frame();
        let frame_count = Profiler::frame_count();
        if frame_count > 0 && frame_count % PROFILER_LOG_INTERVAL == 0 {
            let stats = Profiler::format_stats();
            web_sys::console::log_1(&stats.into());
            Profiler::reset();
        }

        // Request next frame
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut(f64)>));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn request_animation_frame(f: &Closure<dyn FnMut(f64)>) {
    web_sys::window()
        .expect("no window")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("requestAnimationFrame failed");
}
