//! `World` userdata — Lua scripts create and interact with a World instance.
//!
//! ```lua
//! local world = World.new()
//! world:set_gravity(-9.8)
//! world:set_ground(-4.5)
//! world:step(dt)
//! world:auto_render()
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
        // auto_render is handled specially: we buffer the request and the
        // ScriptedGame render phase submits it with the actual renderer.
        methods.add_method("auto_render", |_, this, ()| {
            super::engine::request_auto_render(this.0.clone());
            Ok(())
        });

        // Register object methods (spawn, physics, queries)
        super::objects::add_world_methods(methods);

        // Register camera methods
        super::camera::add_world_methods(methods);
    }
}

/// Register the `World` constructor as a global.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let world_table = lua.create_table()?;

    world_table.set("new", lua.create_function(|_, ()| {
        Ok(LuaWorld(Rc::new(RefCell::new(World::new()))))
    })?)?;

    lua.globals().set("World", world_table)?;
    Ok(())
}
