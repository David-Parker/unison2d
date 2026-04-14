//! Camera bindings — `world.cameras` facade for camera management and follow.
//!
//! ```lua
//! world.cameras:add("overview", 20, 15)
//! world.cameras:follow("main", donut_id, { smoothing = 0.08 })
//! world.cameras:follow("main", donut_id, { smoothing = 0.08, offset = {0, 3.5} })
//! local cx, cy = world.cameras:position("main")
//! local wx, wy = world.cameras:screen_to_world(sx, sy)
//! world.cameras:unfollow("main")
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;
use unison2d::core::Vec2;
use unison2d::render::Camera;
use unison2d::{ObjectId, World};

/// Build the `world.cameras` facade table.
///
/// Each closure clones the `Rc<RefCell<World>>` and dispatches into the Rust API.
/// Lua callers use colon syntax (`world.cameras:follow("main", id, opts)`); the
/// table itself is passed as the first argument and is discarded (`_self`).
pub fn build_cameras_facade(lua: &Lua, world_rc: Rc<RefCell<World>>) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;

    // ---------------------------------------------------------------
    // add(name, width, height) — register a new named camera
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("add", lua.create_function(move |_, (_self, name, width, height): (LuaTable, String, f32, f32)| {
        w.borrow_mut().cameras.add(&name, Camera::new(width, height));
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // follow(name, id, opts?)
    //   opts = { smoothing?, offset? }
    //   offset can be {x, y} positional or omitted.
    //   Missing opts → smoothing=0, offset=(0,0)
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("follow", lua.create_function(move |_, (_self, name, id, opts): (LuaTable, String, u64, Option<LuaTable>)| {
        let (smoothing, offset) = match opts {
            Some(t) => {
                let smoothing = t.get::<Option<f32>>("smoothing")?.unwrap_or(0.0);
                let offset = match t.get::<Option<LuaTable>>("offset")? {
                    Some(ot) => {
                        let ox: f32 = ot.get(1)?;
                        let oy: f32 = ot.get(2)?;
                        Vec2::new(ox, oy)
                    }
                    None => Vec2::ZERO,
                };
                (smoothing, offset)
            }
            None => (0.0, Vec2::ZERO),
        };
        w.borrow_mut().cameras.follow_with_offset(
            &name,
            ObjectId::from_raw(id),
            smoothing,
            offset,
        );
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // unfollow(name) — stop a camera from following any object
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("unfollow", lua.create_function(move |_, (_self, name): (LuaTable, String)| {
        w.borrow_mut().cameras.unfollow(&name);
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // position(name) → x, y — get current camera center
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("position", lua.create_function(move |_, (_self, name): (LuaTable, String)| {
        let world = w.borrow();
        match world.cameras.get(&name) {
            Some(cam) => Ok((cam.x, cam.y)),
            None => Err(LuaError::RuntimeError(format!("camera '{name}' not found"))),
        }
    })?)?;

    // ---------------------------------------------------------------
    // screen_to_world(sx, sy) → wx, wy
    //
    // Converts a screen-space point (e.g. from input.pointer_position()
    // or input.mouse_position()) to world-space using the active "main" camera.
    // Uses the current screen size captured by the engine layer each frame.
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("screen_to_world", lua.create_function(move |_, (_self, sx, sy): (LuaTable, f32, f32)| {
        let world = w.borrow();
        let cam = world.cameras.get("main")
            .ok_or_else(|| LuaError::RuntimeError("main camera not found".into()))?;
        let (sw, sh) = super::engine_state::get_screen_size();
        let (wx, wy) = cam.screen_to_world(sx, sy, sw, sh);
        Ok((wx, wy))
    })?)?;

    Ok(tbl)
}
