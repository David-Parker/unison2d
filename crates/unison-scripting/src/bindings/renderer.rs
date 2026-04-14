//! Renderer bindings — `unison.renderer` table.
//!
//! ```lua
//! local w, h = unison.renderer.screen_size()
//! local mode = unison.renderer.anti_aliasing()
//! unison.renderer.set_anti_aliasing("msaa8x")
//! local target, tex = unison.renderer.create_target(256, 256)
//! ```

use mlua::prelude::*;

use super::engine_state::{get_screen_size, get_aa_mode, set_aa_request, with_engine_ptr};

/// Populate `unison.renderer` on the given `unison` table.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let renderer = lua.create_table()?;

    // unison.renderer.screen_size() → width, height
    renderer.set("screen_size", lua.create_function(|_, ()| {
        let (w, h) = get_screen_size();
        Ok((w, h))
    })?)?;

    // unison.renderer.anti_aliasing() → mode string (or nil if not set)
    renderer.set("anti_aliasing", lua.create_function(|_, ()| {
        Ok(get_aa_mode())
    })?)?;

    // unison.renderer.set_anti_aliasing("msaa8x")
    renderer.set("set_anti_aliasing", lua.create_function(|_, mode: String| {
        set_aa_request(mode);
        Ok(())
    })?)?;

    // unison.renderer.create_target(w, h) → target_id, texture_id
    // Calls through the thread-local engine pointer synchronously.
    // Must be called during init() or on_enter() when the engine pointer is live.
    renderer.set("create_target", lua.create_function(|_, (w, h): (u32, u32)| {
        match with_engine_ptr(|engine| engine.create_render_target(w, h)) {
            Some(Ok((target_id, texture_id))) => Ok((target_id.raw(), texture_id.raw())),
            Some(Err(e)) => {
                eprintln!("[unison-scripting] create_target failed: {e}");
                Ok((0u32, 0u32))
            }
            None => {
                eprintln!("[unison-scripting] unison.renderer.create_target() called outside init — engine not available");
                Ok((0u32, 0u32))
            }
        }
    })?)?;

    unison.set("renderer", renderer)?;
    Ok(())
}
