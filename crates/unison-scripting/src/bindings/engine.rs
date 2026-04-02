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

use super::super::NoAction;

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
    static ENGINE_PTR: std::cell::Cell<Option<*mut Engine<NoAction>>> = const { std::cell::Cell::new(None) };
}

// -- Public accessors for ScriptedGame --

pub fn set_screen_size(w: f32, h: f32) {
    SCREEN_SIZE.with(|c| c.set((w, h)));
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

pub fn take_aa_request() -> Option<String> {
    AA_REQUEST.with(|cell| cell.borrow_mut().take())
}

/// Set the engine pointer for synchronous texture loading during init().
/// # Safety
/// The pointer must remain valid for the duration it is set. Call
/// `clear_engine_ptr()` before the Engine reference goes out of scope.
pub fn set_engine_ptr(engine: &mut Engine<NoAction>) {
    ENGINE_PTR.with(|c| c.set(Some(engine as *mut Engine<NoAction>)));
}

/// Clear the engine pointer after init() completes.
pub fn clear_engine_ptr() {
    ENGINE_PTR.with(|c| c.set(None));
}

// ===================================================================
// Lua registration
// ===================================================================

/// Register the `engine` global table.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let engine = lua.create_table()?;

    // engine.set_background(r, g, b)  — Phase 1 compat
    engine.set("set_background", lua.create_function(|_, args: LuaMultiValue| {
        // Support both hex integer and (r, g, b) float forms.
        if args.len() == 1 {
            // Hex integer form: engine.set_background(0x1a1a2e)
            let hex: u32 = match &args[0] {
                LuaValue::Integer(n) => *n as u32,
                LuaValue::Number(n) => *n as u32,
                other => return Err(LuaError::FromLuaConversionError {
                    from: other.type_name(),
                    to: "integer".into(),
                    message: Some("expected hex color integer".into()),
                }),
            };
            let c = Color::from_hex(hex);
            CLEAR_COLOR.with(|cell| cell.set([c.r, c.g, c.b]));
        } else if args.len() >= 3 {
            // Float form: engine.set_background(r, g, b)
            let r = lua_to_f32(&args[0])?;
            let g = lua_to_f32(&args[1])?;
            let b = lua_to_f32(&args[2])?;
            CLEAR_COLOR.with(|cell| cell.set([r, g, b]));
        }
        Ok(())
    })?)?;

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

    // engine.draw_rect(x, y, w, h, r, g, b) — Phase 1 compat
    engine.set("draw_rect", lua.create_function(|_, (x, y, w, h, r, g, b): (f32, f32, f32, f32, f32, f32, f32)| {
        use unison2d::render::RenderCommand;
        super::super::bridge::push_render_command(RenderCommand::Rect {
            position: [x, y],
            size: [w, h],
            color: Color::new(r, g, b, 1.0),
        });
        Ok(())
    })?)?;

    lua.globals().set("engine", engine)?;
    Ok(())
}

fn lua_to_f32(val: &LuaValue) -> LuaResult<f32> {
    match val {
        LuaValue::Integer(n) => Ok(*n as f32),
        LuaValue::Number(n) => Ok(*n as f32),
        other => Err(LuaError::FromLuaConversionError {
            from: other.type_name(),
            to: "f32".into(),
            message: None,
        }),
    }
}
