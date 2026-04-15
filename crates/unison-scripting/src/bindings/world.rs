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
//!
//! -- Camera facade (Task 12)
//! world.cameras:add("overview", 20, 15)
//! world.cameras:follow("main", id, { smoothing = 0.08, offset = {0, 3.5} })
//! local cx, cy = world.cameras:position("main")
//! local wx, wy = world.cameras:screen_to_world(sx, sy)
//!
//! -- Lights facade (Task 13)
//! world.lights:set_enabled(true)
//! world.lights:set_ambient(0.1, 0.1, 0.15, 1.0)
//! local light = world.lights:add_point({ position = {0, 5}, color = 0xFFDD44, radius = 8.0 })
//! world.lights:follow(light, id, { offset = {0, 2} })
//! world.lights:unfollow(light)
//!
//! -- Collision callbacks (Task 14)
//! world:on_collision(function(a, b, info)
//!     print("collision between " .. a .. " and " .. b)
//! end)
//! world:on_collision_with(donut, function(other, info)
//!     print("donut hit " .. other)
//! end)
//! world:on_collision_between(donut, platform, function(info)
//!     print("donut landed!")
//! end)
//! ```

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use mlua::prelude::*;
use unison2d::render::Color;
use unison2d::World;

/// Monotonic counter for per-world spatial-audio tags.
/// Starts at 1 so `0` can be reserved as "no world" if needed.
static NEXT_WORLD_TAG: AtomicU32 = AtomicU32::new(1);

/// Newtype around `Rc<RefCell<World>>` so we can implement `UserData`.
///
/// The second field is a stable `u32` tag assigned at construction and used
/// to scope spatial-audio playbacks to the owning world (see
/// `world:play_sound_at` / `world:clear_sounds`).
#[derive(Clone)]
pub struct LuaWorld(pub Rc<RefCell<World>>, pub u32 /* tag */);

impl LuaWorld {
    /// Return the world key used to index the collision registry.
    fn world_key(&self) -> super::collisions::WorldKey {
        super::collisions::key_of(&self.0)
    }
}

impl Drop for LuaWorld {
    fn drop(&mut self) {
        // Only clear when we are the last Lua-side reference to this World.
        // `Rc::strong_count` includes the count inside the Rc itself plus any
        // clones held by closures. When it reaches 1 this is the sole owner.
        if Rc::strong_count(&self.0) == 1 {
            super::collisions::clear_world(self.world_key());
        }
    }
}

impl LuaUserData for LuaWorld {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        // Expose the `world.objects` facade as a field.
        // The facade table holds closures that close over the Rc<RefCell<World>>.
        fields.add_field_method_get("objects", |lua, this| {
            super::objects::build_objects_facade(lua, this.0.clone())
        });

        // Expose the `world.cameras` facade as a field.
        fields.add_field_method_get("cameras", |lua, this| {
            super::camera::build_cameras_facade(lua, this.0.clone())
        });

        // Expose the `world.lights` facade as a field.
        fields.add_field_method_get("lights", |lua, this| {
            super::lighting::build_lights_facade(lua, this.0.clone())
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
            super::engine_state::request_render(this.0.clone());
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

        // Register render layer methods
        super::render_layers::add_world_methods(methods);

        // Register render target methods (render_to_targets)
        super::render_targets::add_world_methods(methods);

        // -- Collision callbacks --

        // world:on_collision(fn(a, b, info))
        methods.add_method("on_collision", |lua, this, cb: LuaFunction| {
            super::collisions::register_any(lua, this.world_key(), &this.0, cb)
        });

        // world:on_collision_with(id, fn(other, info))
        methods.add_method("on_collision_with", |lua, this, (id, cb): (u64, LuaFunction)| {
            super::collisions::register_with(lua, this.world_key(), &this.0, id, cb)
        });

        // world:on_collision_between(a, b, fn(info))
        methods.add_method("on_collision_between", |lua, this, (a, b, cb): (u64, u64, LuaFunction)| {
            super::collisions::register_between(lua, this.world_key(), &this.0, a, b, cb)
        });

        // -- Spatial audio (world-scoped) --

        // world:play_sound_at(snd, x, y, opts?) -> PlaybackId
        methods.add_method("play_sound_at",
            |_, this, (snd, x, y, opts): (u32, f32, f32, Option<LuaTable>)| {
            let tag = this.1;
            let pb = super::engine_state::with_engine_ptr(|e| {
                let bus = match opts.as_ref().and_then(|t| t.get::<String>("bus").ok()) {
                    Some(name) => e.audio.bus_by_name(&name).unwrap_or_else(|| e.audio.sfx_bus()),
                    None => e.audio.sfx_bus(),
                };
                let mut p = unison_audio::SpatialParams::at(unison2d::core::Vec2::new(x, y), bus);
                if let Some(t) = opts {
                    if let Ok(v) = t.get::<f32>("volume")       { p.volume = v; }
                    if let Ok(v) = t.get::<f32>("pitch")        { p.pitch = v; }
                    if let Ok(v) = t.get::<f32>("max_distance") { p.max_distance = v; }
                    if let Ok(b) = t.get::<bool>("looping")     { p.looping = b; }
                    p.fade_in = t.get::<f32>("fade_in").ok();
                    if let Ok(s) = t.get::<String>("rolloff") {
                        p.rolloff = match s.as_str() {
                            "linear" => unison_audio::Rolloff::Linear,
                            _        => unison_audio::Rolloff::InverseSquare,
                        };
                    }
                }
                e.audio.play_spatial(unison_audio::SoundId::from_raw(snd), p, Some(tag)).ok()
            }).flatten().map(|p| p.raw()).unwrap_or(0);
            Ok(pb)
        });

        // world:set_sound_position(pb, x, y)
        methods.add_method("set_sound_position",
            |_, _this, (pb, x, y): (u32, f32, f32)| {
            super::engine_state::with_engine_ptr(|e| {
                e.audio.set_position(unison_audio::PlaybackId::from_raw(pb),
                                     unison2d::core::Vec2::new(x, y));
            });
            Ok(())
        });

        // world:clear_sounds(opts?) — stops all spatial playbacks for this world
        methods.add_method("clear_sounds",
            |_, this, opts: Option<LuaTable>| {
            let tag = this.1;
            let fade = opts.and_then(|t| t.get::<f32>("fade_out").ok());
            super::engine_state::with_engine_ptr(|e| {
                e.audio.stop_all_spatial_for(tag, fade);
            });
            Ok(())
        });
    }
}

/// Populate `unison.World` constructor on the given `unison` table.
pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let world_table = lua.create_table()?;

    world_table.set("new", lua.create_function(|_, ()| {
        let tag = NEXT_WORLD_TAG.fetch_add(1, Ordering::Relaxed);
        Ok(LuaWorld(Rc::new(RefCell::new(World::new())), tag))
    })?)?;

    unison.set("World", world_table)?;
    Ok(())
}
