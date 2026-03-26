//! Frame loop for iOS — mirrors `unison-web/src/game_loop.rs`.
//!
//! Unlike the web version (which owns the rAF loop), this module provides
//! a `GameState` struct whose `frame` method is called from Swift via FFI
//! on each CADisplayLink / MTKView tick.

use unison2d::{Engine, Game};
use unison_input::InputBuffer;
use unison_profiler::Profiler;

use crate::MetalRenderer;

/// Fixed timestep: 60 updates per second.
const FIXED_DT: f32 = 1.0 / 60.0;
/// Max accumulated time per frame (prevents spiral of death).
const MAX_ACCUMULATOR: f32 = 0.1;
/// How often (in frames) to log profiler stats to Xcode console.
const PROFILER_LOG_INTERVAL: u64 = 120;

/// Owns the game, engine, and input buffer. One instance per app lifetime.
///
/// Generic over any `Game` type — the concrete type (e.g., `DonutGame`) is
/// supplied by the game crate's FFI layer, not by `unison-ios`.
pub struct GameState<G: Game> {
    game: G,
    engine: Engine<G::Action>,
    input: InputBuffer,
    accumulator: f32,
    initialized: bool,
    /// Raw pointer to the MetalRenderer inside `engine.renderer`.
    /// Used to call `begin_display_frame` / `end_display_frame` which are
    /// Metal-specific methods not on the `Renderer` trait.
    /// Safety: valid for the lifetime of the Engine (which owns the Box<dyn Renderer>).
    metal_renderer: *mut MetalRenderer,
}

impl<G: Game> GameState<G> {
    /// Create a new game state.
    ///
    /// `renderer` is moved into the engine as a trait object, but we keep a
    /// raw pointer to call Metal-specific display frame methods.
    pub fn new(renderer: MetalRenderer, game: G) -> Self {
        let mut engine = Engine::<G::Action>::new();

        // Box the renderer and stash a raw pointer before giving ownership to the engine
        let mut boxed = Box::new(renderer);
        let metal_renderer: *mut MetalRenderer = &mut *boxed;
        engine.renderer = Some(boxed);

        Self {
            game,
            engine,
            input: InputBuffer::new(),
            accumulator: 0.0,
            initialized: false,
            metal_renderer,
        }
    }

    /// Initialize the game. Must be called once after construction.
    pub fn init(&mut self) {
        if self.initialized {
            return;
        }

        // Set up profiler time function (mach_absolute_time → milliseconds)
        use std::sync::Once;
        static PROFILER_INIT: Once = Once::new();
        PROFILER_INIT.call_once(|| {
            // Query the timebase once — ratio to convert ticks → nanoseconds.
            // On Apple Silicon this is always 1:1, but we handle the general case.
            let mut info = mach_timebase_info { numer: 0, denom: 0 };
            unsafe { mach_timebase_info(&mut info) };
            unsafe {
                NANOS_PER_TICK = info.numer as f64 / info.denom as f64;
            }

            unison_profiler::set_time_fn(mach_time_ms);
            Profiler::set_enabled(true);
        });

        self.engine.fixed_dt = FIXED_DT;
        self.game.init(&mut self.engine);
        self.initialized = true;
    }

    /// Access the input buffer for feeding touch/keyboard events from FFI.
    pub fn input_mut(&mut self) -> &mut InputBuffer {
        &mut self.input
    }

    /// Access the engine (e.g., to update screen size).
    pub fn engine_mut(&mut self) -> &mut Engine<G::Action> {
        &mut self.engine
    }

    /// Run one display frame: fixed-timestep updates + render + present.
    ///
    /// Called from Swift on each display refresh (CADisplayLink / MTKViewDelegate).
    /// `dt` is the time since the last frame in seconds.
    /// # Safety
    /// `drawable` must be a valid CAMetalDrawable pointer from MTKView.currentDrawable.
    pub unsafe fn frame(&mut self, dt: f32, drawable: *mut objc::runtime::Object) {
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

        // Begin Metal display frame (create command buffer, use MTKView's drawable)
        (*self.metal_renderer).begin_display_frame(drawable);

        // Engine render cycle (may call begin_frame/end_frame/clear/bind_render_target
        // multiple times — these are now lightweight operations)
        self.game.render(&mut self.engine);

        // End Metal display frame (present drawable, commit command buffer)
        (*self.metal_renderer).end_display_frame();

        // End profiler frame and periodically log stats to Xcode console
        Profiler::end_frame();
        let frame_count = Profiler::frame_count();
        if frame_count > 0 && frame_count % PROFILER_LOG_INTERVAL == 0 {
            let stats = Profiler::format_stats();
            eprintln!("{}", stats);
            Profiler::reset();
        }
    }
}

// --- mach_absolute_time plumbing for profiler ---

/// Cached timebase ratio (set once during init, read every frame).
/// Safety: written once under `Once`, then only read — no data race.
static mut NANOS_PER_TICK: f64 = 1.0;

/// Bare function (no captures) suitable for `set_time_fn`.
fn mach_time_ms() -> f64 {
    let ticks = unsafe { mach_absolute_time() };
    (ticks as f64 * unsafe { NANOS_PER_TICK }) / 1_000_000.0
}

#[repr(C)]
struct mach_timebase_info {
    numer: u32,
    denom: u32,
}

extern "C" {
    fn mach_absolute_time() -> u64;
    fn mach_timebase_info(info: *mut mach_timebase_info) -> i32;
}
