//! Frame loop for Android — mirrors `unison-ios/src/game_loop.rs`.
//!
//! Unlike the web version (which owns the rAF loop), this module provides
//! a `GameState` struct whose `frame` method is called from Kotlin via JNI
//! on each GLSurfaceView draw callback.

use unison2d::{Engine, Game};
use unison_input::InputBuffer;
use unison_profiler::Profiler;

use crate::GlesRenderer;

/// Fixed timestep: 60 updates per second.
const FIXED_DT: f32 = 1.0 / 60.0;
/// Max accumulated time per frame (prevents spiral of death).
const MAX_ACCUMULATOR: f32 = 0.1;
/// How often (in frames) to log profiler stats to logcat.
const PROFILER_LOG_INTERVAL: u64 = 120;

/// Owns the game, engine, and input buffer. One instance per app lifetime.
///
/// Generic over any `Game` type — the concrete type (e.g., `DonutGame`) is
/// supplied by the game crate's JNI layer, not by `unison-android`.
pub struct GameState<G: Game> {
    game: G,
    engine: Engine<G::Action>,
    input: InputBuffer,
    accumulator: f32,
    initialized: bool,
    /// Raw pointer to the GlesRenderer inside `engine.renderer`.
    /// Used to call `begin_display_frame` / `end_display_frame` which are
    /// GLES-specific methods not on the `Renderer` trait.
    /// Safety: valid for the lifetime of the Engine (which owns the Box<dyn Renderer>).
    gles_renderer: *mut GlesRenderer,
}

impl<G: Game> GameState<G> {
    /// Create a new game state.
    ///
    /// `renderer` is moved into the engine as a trait object, but we keep a
    /// raw pointer to call GLES-specific display frame methods.
    pub fn new(renderer: GlesRenderer, game: G) -> Self {
        let mut engine = Engine::<G::Action>::new();

        // Box the renderer and stash a raw pointer before giving ownership to the engine
        let mut boxed = Box::new(renderer);
        let gles_renderer: *mut GlesRenderer = &mut *boxed;
        engine.renderer = Some(boxed);

        Self {
            game,
            engine,
            input: InputBuffer::new(),
            accumulator: 0.0,
            initialized: false,
            gles_renderer,
        }
    }

    /// Initialize the game. Must be called once after construction.
    pub fn init(&mut self) {
        if self.initialized {
            return;
        }

        // Set up profiler time function (clock_gettime CLOCK_MONOTONIC -> milliseconds)
        use std::sync::Once;
        static PROFILER_INIT: Once = Once::new();
        PROFILER_INIT.call_once(|| {
            unison_profiler::set_time_fn(monotonic_time_ms);
            Profiler::set_enabled(true);
        });

        self.engine.fixed_dt = FIXED_DT;
        self.game.init(&mut self.engine);
        self.initialized = true;
    }

    /// Access the input buffer for feeding touch/keyboard events from JNI.
    pub fn input_mut(&mut self) -> &mut InputBuffer {
        &mut self.input
    }

    /// Access the engine (e.g., to update screen size).
    pub fn engine_mut(&mut self) -> &mut Engine<G::Action> {
        &mut self.engine
    }

    /// Run one display frame: fixed-timestep updates + render.
    ///
    /// Called from Kotlin on each GLSurfaceView.Renderer.onDrawFrame callback.
    /// `dt` is the time since the last frame in seconds.
    pub fn frame(&mut self, dt: f32) {
        if !self.initialized {
            return;
        }

        Profiler::begin_frame();

        self.accumulator += dt.min(MAX_ACCUMULATOR);

        // Transfer touch/input events into the engine
        let will_update = self.accumulator >= FIXED_DT;
        if will_update {
            self.input.transfer(true);
            self.input.swap_into(&mut self.engine.input);
        }

        // Fixed-timestep updates
        let mut first_tick = true;
        while self.accumulator >= FIXED_DT {
            if !first_tick {
                self.engine.input.begin_frame();
            }
            first_tick = false;
            self.engine.pre_update();
            self.game.update(&mut self.engine);
            self.accumulator -= FIXED_DT;
        }

        // Begin GLES display frame (bind default FBO, clear screen)
        unsafe { (*self.gles_renderer).begin_display_frame() };

        // Engine render cycle (may call begin_frame/end_frame/clear/bind_render_target
        // multiple times — these are now lightweight operations)
        self.game.render(&mut self.engine);

        // End GLES display frame (GLSurfaceView handles eglSwapBuffers)
        unsafe { (*self.gles_renderer).end_display_frame() };

        // End profiler frame and periodically log stats to logcat
        Profiler::end_frame();
        let frame_count = Profiler::frame_count();
        if frame_count > 0 && frame_count % PROFILER_LOG_INTERVAL == 0 {
            let stats = Profiler::format_stats();
            eprintln!("{}", stats);
            Profiler::reset();
        }
    }
}

/// Bare function (no captures) suitable for `set_time_fn`.
/// Uses CLOCK_MONOTONIC via std::time::Instant for portability.
fn monotonic_time_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static EPOCH: OnceLock<Instant> = OnceLock::new();
    let epoch = EPOCH.get_or_init(Instant::now);
    epoch.elapsed().as_secs_f64() * 1000.0
}
