//! `World` userdata — Lua scripts create and interact with a World instance.
//!
//! ```lua
//! local world = unison.World.new()
//! world:set_gravity(-9.8)
//! world:set_ground(-4.5)
//! world:step(dt)
//! world:render()
//!
//! -- Object facade (Task 11)
//! local id = world.objects:spawn_soft_body({...})
//! local x, y = world.objects:position(id)
//! world.objects:apply_torque(id, 200)
//! world.objects:despawn(id)
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;
use unison2d::render::Color;
use unison2d::World;

/// Newtype around `Rc<RefCell<World>>` so we can implement `UserData`.
#[derive(Clone)]
pub struct LuaWorld(pub Rc<RefCell<World>>);

impl LuaUserData for LuaWorld {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Expose the `world.objects` facade as a field.
        // The facade table holds closures that close over the Rc<RefCell<World>>.
        fields.add_field_method_get("objects", |lua, this| {
            super::objects::build_objects_facade(lua, this.0.clone())
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // -- Background --
        methods.add_method("set_background", |_, this, color: u32| {
            this.0.borrow_mut().set_background(Color::from_hex(color));
            Ok(())
        });

        // -- Physics config (delegated to world.objects) --
        methods.add_method("set_gravity", |_, this, g: f32| {
            this.0.borrow_mut().objects.set_gravity(g);
            Ok(())
        });

        methods.add_method("set_ground", |_, this, y: f32| {
            this.0.borrow_mut().objects.set_ground(y);
            Ok(())
        });

        methods.add_method("set_ground_restitution", |_, this, r: f32| {
            this.0.borrow_mut().objects.set_ground_restitution(r);
            Ok(())
        });

        methods.add_method("set_ground_friction", |_, this, f: f32| {
            this.0.borrow_mut().objects.set_ground_friction(f);
            Ok(())
        });

        // -- Simulation --
        methods.add_method("step", |_, this, dt: f32| {
            this.0.borrow_mut().step(dt);
            Ok(())
        });

        // -- Rendering --
        // render is handled specially: we buffer the request and the
        // ScriptedGame render phase submits it with the actual renderer.
        methods.add_method("render", |_, this, ()| {
            super::engine_state::request_auto_render(this.0.clone());
            Ok(())
        });

        // -- Overlay drawing (moved from engine.draw_overlay) --

        // world:draw_overlay(texture_id, x, y, w, h)
        methods.add_method("draw_overlay", |_, _this, (tex, x, y, w, h): (u32, f32, f32, f32, f32)| {
            use super::render_targets::{push_overlay, OverlayRequest};
            push_overlay(OverlayRequest {
                texture_id: tex,
                x, y, w, h,
                border: None,
            });
            Ok(())
        });

        // world:draw_overlay_bordered(texture_id, x, y, w, h, border_width, border_color)
        methods.add_method("draw_overlay_bordered", |_, _this, (tex, x, y, w, h, bw, bc): (u32, f32, f32, f32, f32, f32, u32)| {
            use super::render_targets::{push_overlay, OverlayRequest, OverlayBorder};
            push_overlay(OverlayRequest {
                texture_id: tex,
                x, y, w, h,
                border: Some(OverlayBorder { width: bw, color: bc }),
            });
            Ok(())
        });

        // Register camera methods
        super::camera::add_world_methods(methods);

        // Register lighting methods
        super::lighting::add_world_methods(methods);

        // Register render layer methods
        super::render_layers::add_world_methods(methods);

        // Register render target methods (render_to_targets)
        super::render_targets::add_world_methods(methods);
    }
}

/// Populate `unison.World` constructor on the given `unison` table.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let world_table = lua.create_table()?;

    world_table.set("new", lua.create_function(|_, ()| {
        Ok(LuaWorld(Rc::new(RefCell::new(World::new()))))
    })?)?;

    unison.set("World", world_table)?;
    Ok(())
}
