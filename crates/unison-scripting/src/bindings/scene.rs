//! Scene management — Lua-side scene tables replace Rust Level trait.
//!
//! A scene is a Lua table with lifecycle functions:
//!
//! ```lua
//! local gameplay = {
//!     on_enter = function() ... end,
//!     update = function(dt) ... end,
//!     render = function() ... end,
//!     on_exit = function() ... end,
//! }
//!
//! unison.scenes.set(gameplay)
//! ```
//!
//! The scene system stores the current scene table in a thread-local. The
//! `ScriptedGame` dispatches `update(dt)` and `render()` to the active scene
//! instead of directly to the `__game` table when scenes are in use.

use std::cell::RefCell;

use mlua::prelude::*;

// ===================================================================
// Thread-local scene state
// ===================================================================

thread_local! {
    /// Registry key for the current scene table, if scene management is active.
    static CURRENT_SCENE: RefCell<Option<LuaRegistryKey>> = const { RefCell::new(None) };

    /// Whether scene management is active (at least one set_scene call).
    static SCENES_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Check if scene management is active.
pub fn is_active() -> bool {
    SCENES_ACTIVE.with(|c| c.get())
}

/// Call the current scene's `update(dt)`. Returns Ok(true) if a scene handled it.
pub fn call_scene_update(lua: &Lua, dt: f32) -> LuaResult<bool> {
    if !is_active() {
        return Ok(false);
    }

    CURRENT_SCENE.with(|cell| {
        let key = cell.borrow();
        let key = match key.as_ref() {
            Some(k) => k,
            None => return Ok(false),
        };

        let scene: LuaTable = lua.registry_value(key)?;
        if let Ok(func) = scene.get::<LuaFunction>("update") {
            func.call::<()>(dt)?;
        }
        Ok(true)
    })
}

/// Call the current scene's `render()`. Returns Ok(true) if a scene handled it.
pub fn call_scene_render(lua: &Lua) -> LuaResult<bool> {
    if !is_active() {
        return Ok(false);
    }

    CURRENT_SCENE.with(|cell| {
        let key = cell.borrow();
        let key = match key.as_ref() {
            Some(k) => k,
            None => return Ok(false),
        };

        let scene: LuaTable = lua.registry_value(key)?;
        if let Ok(func) = scene.get::<LuaFunction>("render") {
            func.call::<()>(())?;
        }
        Ok(true)
    })
}

/// Reset the scene system — clears current scene and deactivates scene management.
/// Called from `ScriptedGame::drop()` to avoid leaking thread-local state.
pub fn reset() {
    CURRENT_SCENE.with(|cell| {
        // We cannot call lua.remove_registry_value here without a Lua reference,
        // but the Lua VM is being dropped anyway, so just clear the Option.
        *cell.borrow_mut() = None;
    });
    SCENES_ACTIVE.with(|c| c.set(false));
}

// ===================================================================
// Registration under unison.scenes
// ===================================================================

/// Populate `unison.scenes` on the given `unison` table.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let scenes = lua.create_table()?;

    // unison.scenes.set(scene_table)
    // Unified set+switch: calls on_exit on the current scene (if any),
    // then on_enter on the new scene.
    scenes.set("set", lua.create_function(|lua, new_scene: LuaTable| {
        // Call on_exit on the old scene, if one is active.
        CURRENT_SCENE.with(|cell| -> LuaResult<()> {
            if let Some(old_key) = cell.borrow().as_ref() {
                if let Ok(old_scene) = lua.registry_value::<LuaTable>(old_key) {
                    if let Ok(func) = old_scene.get::<LuaFunction>("on_exit") {
                        func.call::<()>(())?;
                    }
                }
            }
            Ok(())
        })?;

        // Call on_enter on new scene.
        if let Ok(func) = new_scene.get::<LuaFunction>("on_enter") {
            func.call::<()>(())?;
        }

        // Store new scene.
        let key = lua.create_registry_value(new_scene)?;
        CURRENT_SCENE.with(|cell| {
            if let Some(old) = cell.borrow_mut().take() {
                lua.remove_registry_value(old).ok();
            }
            *cell.borrow_mut() = Some(key);
        });
        SCENES_ACTIVE.with(|c| c.set(true));

        Ok(())
    })?)?;

    // unison.scenes.current() → scene table or nil
    scenes.set("current", lua.create_function(|lua, ()| -> LuaResult<LuaValue> {
        CURRENT_SCENE.with(|cell| {
            match cell.borrow().as_ref() {
                Some(key) => match lua.registry_value::<LuaTable>(key) {
                    Ok(t) => Ok(LuaValue::Table(t)),
                    Err(_) => Ok(LuaValue::Nil),
                },
                None => Ok(LuaValue::Nil),
            }
        })
    })?)?;

    unison.set("scenes", scenes)?;
    Ok(())
}
