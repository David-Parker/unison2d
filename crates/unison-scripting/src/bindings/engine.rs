//! Engine bindings — texture loading, screen size, anti-aliasing.
//!
//! Also manages the thread-local state needed to bridge Lua calls with the
//! Engine/Renderer, which are only available during `ScriptedGame`'s trait
//! method calls.
//!
//! ```lua
//! local tex = engine.load_texture("textures/donut-pink.png")
//! local w, h = engine.screen_size()
//! engine.set_anti_aliasing("msaa8x")
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;
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

    /// World to auto-render, set by `world:auto_render()`, consumed by `ScriptedGame::render`.
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
/// Used by [`crate::bindings::engine::reset()`].
pub fn clear_engine_ptr() {
    ENGINE_PTR.with(|c| c.set(None));
}

// ===================================================================
// Lua registration
// ===================================================================

/// Register the `engine` global table.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let engine = lua.create_table()?;

    // engine.load_texture("path") → integer texture ID
    // Loads synchronously via the thread-local engine pointer (set during init).
    engine.set("load_texture", lua.create_function(|_, path: String| {
        ENGINE_PTR.with(|c| {
            match c.get() {
                Some(ptr) => {
                    // Safety: ptr is valid while ScriptedGame::init() is on the stack.
                    let engine = unsafe { &mut *ptr };
                    match engine.load_texture(&path) {
                        Ok(tid) => Ok(tid.raw()),
                        Err(e) => {
                            eprintln!("[unison-scripting] Failed to load texture '{path}': {e}");
                            Ok(0)
                        }
                    }
                }
                None => {
                    eprintln!("[unison-scripting] engine.load_texture() called outside init — engine not available");
                    Ok(0)
                }
            }
        })
    })?)?;

    // engine.screen_size() → width, height
    engine.set("screen_size", lua.create_function(|_, ()| {
        let (w, h) = SCREEN_SIZE.with(|c| c.get());
        Ok((w, h))
    })?)?;

    // engine.set_anti_aliasing("msaa8x")
    engine.set("set_anti_aliasing", lua.create_function(|_, mode: String| {
        AA_REQUEST.with(|cell| {
            *cell.borrow_mut() = Some(mode);
        });
        Ok(())
    })?)?;

    lua.globals().set("engine", engine)?;
    Ok(())
}

/// Reset all thread-local engine state.
/// Called from `ScriptedGame::drop()` to avoid leaking state between instances.
pub fn reset() {
    SCREEN_SIZE.with(|c| c.set((960.0, 540.0)));
    CLEAR_COLOR.with(|c| c.set([0.1, 0.1, 0.12]));
    AUTO_RENDER_WORLD.with(|cell| *cell.borrow_mut() = None);
    AA_REQUEST.with(|cell| *cell.borrow_mut() = None);
    ENGINE_PTR.with(|c| c.set(None));
}

