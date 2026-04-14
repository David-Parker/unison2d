//! Thread-local engine state — bridge between the Lua VM and the Rust engine.
//!
//! This module is **internal only** — it has no Lua registration. It contains
//! the thread-locals that let Lua closures call engine methods synchronously
//! during `ScriptedGame` lifecycle calls, plus public helpers that other
//! binding modules use.

use std::cell::RefCell;
use std::rc::Rc;

use unison2d::render::Color;
use unison2d::{Engine, World};

// ===================================================================
// Thread-local bridge state
// ===================================================================

thread_local! {
    /// Screen dimensions — refreshed each frame.
    static SCREEN_SIZE: std::cell::Cell<(f32, f32)> = const { std::cell::Cell::new((960.0, 540.0)) };

    /// Background clear color — set by the old Phase 1 `engine.set_background(r,g,b)`.
    static CLEAR_COLOR: std::cell::Cell<[f32; 3]> = const { std::cell::Cell::new([0.1, 0.1, 0.12]) };

    /// World to render, set by `world:render()`, consumed by `ScriptedGame::render`.
    static AUTO_RENDER_WORLD: RefCell<Option<Rc<RefCell<World>>>> = const { RefCell::new(None) };

    /// Anti-aliasing mode request (string), consumed during init.
    static AA_REQUEST: RefCell<Option<String>> = const { RefCell::new(None) };

    /// Raw pointer to the Engine, set during init() so Lua closures can call
    /// engine methods (like load_texture) synchronously. Only valid while
    /// the ScriptedGame::init() call is on the stack.
    static ENGINE_PTR: std::cell::Cell<Option<*mut Engine>> = const { std::cell::Cell::new(None) };
}

// -- Public accessors for ScriptedGame --

pub fn set_screen_size(w: f32, h: f32) {
    SCREEN_SIZE.with(|c| c.set((w, h)));
}

pub fn get_screen_size() -> (f32, f32) {
    SCREEN_SIZE.with(|c| c.get())
}

pub fn get_clear_color() -> Color {
    let [r, g, b] = CLEAR_COLOR.with(|c| c.get());
    Color::new(r, g, b, 1.0)
}

pub fn request_auto_render(world: Rc<RefCell<World>>) {
    AUTO_RENDER_WORLD.with(|cell| {
        *cell.borrow_mut() = Some(world);
    });
}

pub fn take_auto_render_world() -> Option<Rc<RefCell<World>>> {
    AUTO_RENDER_WORLD.with(|cell| cell.borrow_mut().take())
}

/// Non-consuming peek at the auto-render world (for collision event flushing).
pub fn peek_auto_render_world() -> Option<Rc<RefCell<World>>> {
    AUTO_RENDER_WORLD.with(|cell| cell.borrow().clone())
}

pub fn take_aa_request() -> Option<String> {
    AA_REQUEST.with(|cell| cell.borrow_mut().take())
}

pub fn set_aa_request(mode: String) {
    AA_REQUEST.with(|cell| {
        *cell.borrow_mut() = Some(mode);
    });
}

pub fn get_aa_mode() -> Option<String> {
    AA_REQUEST.with(|cell| cell.borrow().clone())
}

/// Call a closure with a mutable reference to the engine, if the pointer is set.
/// Returns `None` when the engine pointer is not live.
///
/// # Safety
/// The returned reference is only valid while the `ScriptedGame` lifecycle method
/// that set the pointer is still on the stack.
pub fn with_engine_ptr<R>(f: impl FnOnce(&mut Engine) -> R) -> Option<R> {
    ENGINE_PTR.with(|c| {
        c.get().map(|ptr| {
            // Safety: ptr is valid while ScriptedGame::init()/update()/render()
            // is on the stack and the guard has not yet been dropped.
            let engine = unsafe { &mut *ptr };
            f(engine)
        })
    })
}

/// RAII guard that clears the engine pointer when dropped.
///
/// Bind the return value of [`set_engine_ptr`] to a named variable (e.g.
/// `let _guard = set_engine_ptr(engine)`) so the pointer is automatically
/// cleared when the guard goes out of scope.
pub struct EngineGuard;

impl Drop for EngineGuard {
    fn drop(&mut self) {
        ENGINE_PTR.with(|c| c.set(None));
    }
}

/// Set the engine pointer for synchronous texture loading and return an RAII
/// guard that clears it on drop.
///
/// # Safety
/// The pointer must remain valid for the lifetime of the returned [`EngineGuard`].
/// Drop the guard before `engine` goes out of scope or is moved.
pub fn set_engine_ptr(engine: &mut Engine) -> EngineGuard {
    ENGINE_PTR.with(|c| c.set(Some(engine as *mut Engine)));
    EngineGuard
}

/// Explicitly clear the engine pointer without needing the guard.
pub fn clear_engine_ptr() {
    ENGINE_PTR.with(|c| c.set(None));
}

/// Reset all thread-local engine state.
/// Called from `ScriptedGame::drop()` to avoid leaking state between instances.
pub fn reset() {
    SCREEN_SIZE.with(|c| c.set((960.0, 540.0)));
    CLEAR_COLOR.with(|c| c.set([0.1, 0.1, 0.12]));
    AUTO_RENDER_WORLD.with(|cell| *cell.borrow_mut() = None);
    AA_REQUEST.with(|cell| *cell.borrow_mut() = None);
    ENGINE_PTR.with(|c| c.set(None));
    super::action_map::reset();
}
