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
use unison2d::World;

// ===================================================================
// Thread-local bridge state
// ===================================================================

thread_local! {
    /// Screen dimensions — refreshed each frame.
    static SCREEN_SIZE: std::cell::Cell<(f32, f32)> = const { std::cell::Cell::new((960.0, 540.0)) };

    /// Background clear color — set by the old Phase 1 `engine.set_background(r,g,b)`.
    static CLEAR_COLOR: std::cell::Cell<[f32; 3]> = const { std::cell::Cell::new([0.1, 0.1, 0.12]) };

    /// Texture load requests queued during `init()`. Each entry is an asset path.
    /// Resolved by ScriptedGame after the Lua init() call returns.
    static TEXTURE_REQUESTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };

    /// Texture IDs resulting from load requests, indexed in the same order.
    static TEXTURE_RESULTS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };

    /// World to auto-render, set by `world:auto_render()`, consumed by `ScriptedGame::render`.
    static AUTO_RENDER_WORLD: RefCell<Option<Rc<RefCell<World>>>> = const { RefCell::new(None) };

    /// Anti-aliasing mode request (string), consumed during init.
    static AA_REQUEST: RefCell<Option<String>> = const { RefCell::new(None) };
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

pub fn take_texture_requests() -> Vec<String> {
    TEXTURE_REQUESTS.with(|cell| {
        let mut v = cell.borrow_mut();
        std::mem::take(&mut *v)
    })
}

pub fn push_texture_result(id: u32) {
    TEXTURE_RESULTS.with(|cell| cell.borrow_mut().push(id));
}

pub fn take_aa_request() -> Option<String> {
    AA_REQUEST.with(|cell| cell.borrow_mut().take())
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
    // During init: queues the request; the ID is assigned after Lua returns.
    // We use a synchronous two-phase approach: the Rust side resolves requests
    // between the script load and the init() call.
    engine.set("load_texture", lua.create_function(|_, path: String| {
        // Queue the request.
        let idx = TEXTURE_REQUESTS.with(|cell| {
            let mut v = cell.borrow_mut();
            let idx = v.len();
            v.push(path);
            idx
        });
        // Return a "pending" index. The ScriptedGame will resolve this
        // after Lua init() by populating TEXTURE_RESULTS, and we patch
        // the Lua global with real IDs. For now, return the index directly;
        // the actual engine.load_texture will be resolved synchronously
        // by calling the closure *within* init, so the result is available.
        TEXTURE_RESULTS.with(|cell| {
            let results = cell.borrow();
            // If already resolved (during init replay), return the real ID.
            if idx < results.len() {
                Ok(results[idx])
            } else {
                // Not yet resolved — return the index as a placeholder.
                // This shouldn't happen in normal flow since we resolve inline.
                Ok(idx as u32)
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
