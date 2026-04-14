//! Debug utility bindings — logging, draw helpers, and debug visualization toggles.
//!
//! ```lua
//! -- Print to the platform console (varargs, joined with tab)
//! debug.log("player pos:", x, y)
//!
//! -- Draw a small point in world space (uses a tiny rect render command)
//! debug.draw_point(x, y, 0xFF0000)
//!
//! -- Toggle debug visualizations (no-op until engine support lands)
//! debug.show_physics(true)
//! debug.show_fps(true)
//! ```

use mlua::prelude::*;

/// Populate `unison.debug` on the given `unison` table.
///
/// The Lua standard `debug` table (with traceback, getinfo, etc.) is left
/// untouched; engine-specific helpers live under `unison.debug` instead.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let debug = lua.create_table()?;

    // debug.log(...) — print varargs to platform console, joined with tab.
    debug.set("log", lua.create_function(|lua, args: LuaMultiValue| {
        let tostring: LuaFunction = lua.globals().get("tostring")?;
        let parts: Vec<String> = args.iter()
            .map(|v| tostring.call::<String>(v.clone()).unwrap_or_else(|_| "?".to_string()))
            .collect();
        eprintln!("[debug] {}", parts.join("\t"));
        Ok(())
    })?)?;

    // debug.draw_point(x, y, color) — draw a small debug point in world space.
    // Uses a 0.1-unit rect centered at (x, y) so it's visible but unobtrusive.
    // Color is an integer hex value (e.g. 0xFF0000 for red).
    debug.set("draw_point", lua.create_function(|_, (x, y, color): (f32, f32, u32)| {
        use unison2d::render::{Color, RenderCommand};
        use crate::bridge;

        let c = Color::from_hex(color);
        let half = 0.05_f32;
        bridge::push_render_command(RenderCommand::Rect {
            position: [x - half, y - half],
            size: [half * 2.0, half * 2.0],
            color: c,
        });
        Ok(())
    })?)?;

    // debug.show_physics(enabled) — toggle physics debug visualization.
    // TODO: wire up when the engine exposes a physics debug draw API.
    debug.set("show_physics", lua.create_function(|_, enabled: bool| {
        let _ = enabled;
        // TODO: call physics debug visualization toggle when available in unison2d
        Ok(())
    })?)?;

    // debug.show_fps(enabled) — toggle FPS counter overlay.
    // TODO: wire up when the engine exposes an FPS HUD API.
    debug.set("show_fps", lua.create_function(|_, enabled: bool| {
        let _ = enabled;
        // TODO: call engine FPS counter toggle when available in unison2d
        Ok(())
    })?)?;

    unison.set("debug", debug)?;
    Ok(())
}
