//! Asset loading bindings — `unison.assets` table.
//!
//! ```lua
//! local tex = unison.assets.load_texture("textures/donut-pink.png")
//! ```

use mlua::prelude::*;

use super::engine_state::with_engine_ptr;

/// Populate `unison.assets` on the given `unison` table.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let assets = lua.create_table()?;

    // unison.assets.load_texture("path") → integer texture ID
    // Loads synchronously via the thread-local engine pointer (set during init).
    assets.set("load_texture", lua.create_function(|_, path: String| {
        match with_engine_ptr(|engine| engine.load_texture(&path)) {
            Some(Ok(tid)) => Ok(tid.raw()),
            Some(Err(e)) => {
                eprintln!("[unison-scripting] Failed to load texture '{path}': {e}");
                Ok(0)
            }
            None => {
                eprintln!("[unison-scripting] unison.assets.load_texture() called outside init — engine not available");
                Ok(0)
            }
        }
    })?)?;

    unison.set("assets", assets)?;
    Ok(())
}
