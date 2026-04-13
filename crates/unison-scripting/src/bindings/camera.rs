//! Camera bindings — methods on World for camera follow and management.
//!
//! ```lua
//! world:camera_follow("main", donut_id, 0.08)
//! world:camera_follow_with_offset("main", donut_id, 0.08, 0, 3.5)
//! world:camera_add("overview", 20, 15)
//! local cx, cy = world:camera_get_position("main")
//! ```

use mlua::prelude::*;
use unison2d::core::Vec2;
use unison2d::render::Camera;
use unison2d::ObjectId;

use super::world::LuaWorld;

/// Register camera-related methods on the LuaWorld userdata.
pub fn add_world_methods<M: LuaUserDataMethods<LuaWorld>>(methods: &mut M) {
    methods.add_method("camera_follow", |_, this, (name, id, smoothing): (String, u64, f32)| {
        this.0.borrow_mut().cameras.follow(&name, ObjectId::from_raw(id), smoothing);
        Ok(())
    });

    methods.add_method("camera_follow_with_offset", |_, this, (name, id, smoothing, ox, oy): (String, u64, f32, f32, f32)| {
        this.0.borrow_mut().cameras.follow_with_offset(
            &name,
            ObjectId::from_raw(id),
            smoothing,
            Vec2::new(ox, oy),
        );
        Ok(())
    });

    methods.add_method("camera_add", |_, this, (name, w, h): (String, f32, f32)| {
        this.0.borrow_mut().cameras.add(&name, Camera::new(w, h));
        Ok(())
    });

    methods.add_method("camera_get_position", |_, this, name: String| {
        let world = this.0.borrow();
        match world.cameras.get(&name) {
            Some(cam) => Ok((cam.x, cam.y)),
            None => Err(LuaError::RuntimeError(format!("camera '{name}' not found"))),
        }
    });

    // world:screen_to_world(screen_x, screen_y) → world_x, world_y
    //
    // Converts a screen-space point (e.g., from input.pointer_just_pressed() or
    // input.mouse_position()) to world-space using the active "main" camera.
    // Uses the current screen size captured by the engine layer each frame.
    methods.add_method("screen_to_world", |_, this, (sx, sy): (f32, f32)| {
        let world = this.0.borrow();
        let cam = world.cameras.get("main")
            .ok_or_else(|| LuaError::RuntimeError("main camera not found".into()))?;
        let (sw, sh) = super::engine::get_screen_size();
        let (wx, wy) = cam.screen_to_world(sx, sy, sw, sh);
        Ok((wx, wy))
    });
}
