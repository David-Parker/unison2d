//! Asset loading bindings — `unison.assets` table.
//!
//! All loaders return an opaque handle on success and `nil` on failure.
//! Errors are logged via `eprintln!`; scripts should test the result with
//! `if not id then ...` or (in TypeScript) `if (id !== undefined) { ... }`.
//!
//! ```lua
//! local tex  = unison.assets.load_texture("textures/donut-pink.png")
//! local snd  = unison.assets.load_sound("audio/jump.ogg")
//! local font = unison.assets.load_font("fonts/DejaVuSans-Bold.ttf")
//! local data = unison.assets.load_bytes("data/levels/01.json")
//! ```

use mlua::prelude::*;

use super::engine_state::with_engine_ptr;

/// Populate `unison.assets` on the given `unison` table.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let assets = lua.create_table()?;

    assets.set("load_texture", lua.create_function(|_, path: String| {
        let result = with_engine_ptr(|engine| engine.load_texture(&path));
        match result {
            Some(Ok(tid)) => Ok(LuaValue::Integer(tid.raw() as i64)),
            Some(Err(e)) => {
                eprintln!("[unison-scripting] Failed to load texture '{path}': {e}");
                Ok(LuaValue::Nil)
            }
            None => {
                eprintln!("[unison-scripting] unison.assets.load_texture() called outside init — engine not available");
                Ok(LuaValue::Nil)
            }
        }
    })?)?;

    assets.set("load_sound", lua.create_function(|_, path: String| {
        let result = with_engine_ptr(|e| {
            let bytes = e.assets().get(&path)
                .map(|b| b.to_vec())
                .ok_or_else(|| format!("Asset not found: '{path}'"))?;
            e.audio.load(&bytes)
                .map_err(|err| format!("audio load failed: {err}"))
        });
        match result {
            Some(Ok(id)) => Ok(LuaValue::Integer(id.raw() as i64)),
            Some(Err(e)) => {
                eprintln!("[unison-scripting] Failed to load sound '{path}': {e}");
                Ok(LuaValue::Nil)
            }
            None => {
                eprintln!("[unison-scripting] unison.assets.load_sound() called outside init — engine not available");
                Ok(LuaValue::Nil)
            }
        }
    })?)?;

    assets.set("load_font", lua.create_function(|_, path: String| {
        let result = with_engine_ptr(|e| e.load_font(&path));
        match result {
            Some(Ok(id)) => Ok(LuaValue::Integer(id.raw() as i64)),
            Some(Err(e)) => {
                eprintln!("[unison-scripting] Failed to load font '{path}': {e}");
                Ok(LuaValue::Nil)
            }
            None => {
                eprintln!("[unison-scripting] unison.assets.load_font() called outside init — engine not available");
                Ok(LuaValue::Nil)
            }
        }
    })?)?;

    assets.set("load_bytes", lua.create_function(|lua, path: String| {
        let result = with_engine_ptr(|e| {
            e.assets().get(&path).map(|b| b.to_vec())
        });
        match result {
            Some(Some(bytes)) => Ok(LuaValue::String(lua.create_string(&bytes)?)),
            Some(None) => {
                eprintln!("[unison-scripting] Failed to load bytes '{path}': asset not found");
                Ok(LuaValue::Nil)
            }
            None => {
                eprintln!("[unison-scripting] unison.assets.load_bytes() called outside init — engine not available");
                Ok(LuaValue::Nil)
            }
        }
    })?)?;

    unison.set("assets", assets)?;
    Ok(())
}
