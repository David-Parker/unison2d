//! Composer module — builds the `unison` global table from all subsystems.
//!
//! After `register` runs, the Lua VM has a single global `unison` with:
//!
//! ```text
//! unison.assets       -- load_texture
//! unison.renderer     -- screen_size, anti_aliasing, set_anti_aliasing, create_target
//! unison.input        -- is_key_pressed, is_pointer_just_pressed, ...
//! unison.scenes       -- set, current
//! unison.events       -- on, emit, on_collision, ...
//! unison.UI           -- new(font_path)
//! unison.debug        -- log, draw_point, show_physics, show_fps
//! unison.math         -- lerp, smoothstep, clamp
//! unison.Color        -- hex, rgba  (+ Color userdata)
//! unison.Rng          -- new       (+ Rng userdata)
//! unison.World        -- new       (+ World userdata)
//! ```

use mlua::prelude::*;

/// Register the `unison` global and populate all subsystems.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let unison = lua.create_table()?;

    super::assets::populate(lua, &unison)?;
    super::renderer::populate(lua, &unison)?;
    super::input::populate(lua, &unison)?;
    super::scene::populate(lua, &unison)?;
    super::events::populate(lua, &unison)?;
    super::ui::populate(lua, &unison)?;
    super::debug::populate(lua, &unison)?;
    super::math::populate(lua, &unison)?;
    super::world::populate(lua, &unison)?;
    super::audio::populate(lua, &unison)?;

    lua.globals().set("unison", unison)?;
    Ok(())
}
