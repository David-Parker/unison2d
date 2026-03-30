//! Minimal Lua bridge — exposes a small `engine` table to scripts.
//!
//! These bindings are sufficient for Phase 1b:
//! - `engine.set_background(r, g, b)` — set the clear color
//! - `engine.draw_rect(x, y, w, h, r, g, b)` — draw a colored rectangle
//! - `engine.screen_size()` — returns width, height
//!
//! Render commands are buffered during the Lua `render()` call and submitted
//! to the renderer after the call returns. This avoids storing a raw renderer
//! pointer in a thread-local across a Lua call boundary.

use mlua::prelude::*;
use unison2d::render::{Renderer, RenderCommand, Color};

thread_local! {
    // Commands buffered by bridge calls during a render frame.
    static PENDING_COMMANDS: std::cell::RefCell<Vec<RenderCommand>> =
        std::cell::RefCell::new(Vec::new());
    // Cached screen dimensions (updated each frame from update()).
    static SCREEN_SIZE: std::cell::Cell<(f32, f32)> =
        std::cell::Cell::new((960.0, 540.0));
    // Background clear color set by engine.set_background().
    static CLEAR_COLOR: std::cell::Cell<[f32; 3]> =
        std::cell::Cell::new([0.1, 0.1, 0.12]);
}

/// Update the cached screen size (called each frame).
pub fn set_screen_size(w: f32, h: f32) {
    SCREEN_SIZE.with(|cell| cell.set((w, h)));
}

/// Get the clear color configured by Lua's `engine.set_background()`.
pub fn get_clear_color() -> Color {
    let [r, g, b] = CLEAR_COLOR.with(|c| c.get());
    Color::new(r, g, b, 1.0)
}

/// Drain buffered render commands and submit them to the renderer.
pub fn flush_commands(renderer: &mut dyn Renderer<Error = String>) {
    PENDING_COMMANDS.with(|cell| {
        let mut cmds = cell.borrow_mut();
        for cmd in cmds.drain(..) {
            renderer.draw(cmd);
        }
    });
}

/// Register the `engine` global table in the Lua VM.
pub fn register_engine_globals(lua: &Lua) -> LuaResult<()> {
    let engine = lua.create_table()?;

    // engine.set_background(r, g, b)
    engine.set("set_background", lua.create_function(|_, (r, g, b): (f32, f32, f32)| {
        CLEAR_COLOR.with(|c| c.set([r, g, b]));
        Ok(())
    })?)?;

    // engine.draw_rect(x, y, w, h, r, g, b)
    engine.set("draw_rect", lua.create_function(|_, (x, y, w, h, r, g, b): (f32, f32, f32, f32, f32, f32, f32)| {
        PENDING_COMMANDS.with(|cell| {
            cell.borrow_mut().push(RenderCommand::Rect {
                position: [x, y],
                size: [w, h],
                color: Color::new(r, g, b, 1.0),
            });
        });
        Ok(())
    })?)?;

    // engine.screen_size() → width, height
    engine.set("screen_size", lua.create_function(|_, ()| {
        let (w, h) = SCREEN_SIZE.with(|c| c.get());
        Ok((w, h))
    })?)?;

    lua.globals().set("engine", engine)?;
    Ok(())
}
