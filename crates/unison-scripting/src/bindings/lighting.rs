//! Lighting bindings — `world.lights` facade for point lights, directional lights,
//! and shadow/ambient configuration.
//!
//! ```lua
//! world.lights:set_enabled(true)
//! world.lights:set_ambient(0.1, 0.1, 0.15, 1.0)
//! world.lights:set_ground_shadow(-4.5)   -- pass nil to disable
//!
//! local light = world.lights:add_point({
//!     position = {0, 5}, color = 0xFFDD44,
//!     intensity = 2.0, radius = 8.0,
//!     casts_shadows = true, shadow = "soft",
//! })
//!
//! local dir = world.lights:add_directional({
//!     direction = {-0.5, -1.0}, color = 0xFFFFFF,
//!     intensity = 0.8, casts_shadows = true,
//! })
//!
//! world.lights:set_intensity(light, 3.0)
//! world.lights:set_direction(dir, -0.7, -1.0)
//! world.lights:follow(light, donut)
//! world.lights:follow(light, donut, { offset = {0, 2} })
//! world.lights:unfollow(light)
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use mlua::prelude::*;
use unison2d::core::{Color, Vec2};
use unison2d::lighting::{LightId, PointLight, DirectionalLight, ShadowSettings};
use unison2d::lighting::occluder::ShadowFilter;
use unison2d::{ObjectId, World};

/// Build the `world.lights` facade table.
///
/// Each closure clones the `Rc<RefCell<World>>` and dispatches into the Rust API.
/// Lua callers use colon syntax (`world.lights:follow(light, id, opts)`); the
/// table itself is passed as the first argument and is discarded (`_self`).
pub fn build_lights_facade(lua: &Lua, world_rc: Rc<RefCell<World>>) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;

    // ---------------------------------------------------------------
    // set_enabled(bool) — enable or disable the lighting system
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("set_enabled", lua.create_function(move |_, (_self, enabled): (LuaTable, bool)| {
        w.borrow_mut().lighting.set_enabled(enabled);
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // set_ambient(r, g, b, a) — set ambient light color
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("set_ambient", lua.create_function(move |_, (_self, r, g, b, a): (LuaTable, f32, f32, f32, f32)| {
        w.borrow_mut().lighting.set_ambient(Color::new(r, g, b, a));
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // set_ground_shadow(y | nil) — add/remove a ground shadow plane
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("set_ground_shadow", lua.create_function(move |_, (_self, y): (LuaTable, LuaValue)| {
        let mut world = w.borrow_mut();
        match y {
            LuaValue::Nil | LuaValue::Boolean(false) => {
                world.lighting.set_ground_shadow(None);
            }
            LuaValue::Number(n) => {
                world.lighting.set_ground_shadow(Some(n as f32));
            }
            LuaValue::Integer(n) => {
                world.lighting.set_ground_shadow(Some(n as f32));
            }
            _ => {
                return Err(LuaError::FromLuaConversionError {
                    from: y.type_name(),
                    to: "number or nil".into(),
                    message: Some("expected ground Y position or nil/false to disable".into()),
                });
            }
        }
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // add_point(desc) → light_id — add a point light
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("add_point", lua.create_function(move |_, (_self, desc): (LuaTable, LuaTable)| {
        let position = read_vec2(&desc, "position")?;
        let color = read_light_color(&desc)?;
        let intensity: f32 = desc.get("intensity").unwrap_or(1.0);
        let radius: f32 = desc.get("radius").unwrap_or(5.0);

        let mut light = PointLight::new(position, color, intensity, radius);
        light.casts_shadows = desc.get("casts_shadows").unwrap_or(false);
        light.shadow = resolve_shadow_settings(&desc)?;

        let id = w.borrow_mut().lighting.add_light(light);
        Ok(id.raw())
    })?)?;

    // ---------------------------------------------------------------
    // add_directional(desc) → light_id — add a directional light
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("add_directional", lua.create_function(move |_, (_self, desc): (LuaTable, LuaTable)| {
        let direction = read_vec2(&desc, "direction")?;
        let color = read_light_color(&desc)?;
        let intensity: f32 = desc.get("intensity").unwrap_or(1.0);

        let mut light = DirectionalLight::new(direction, color, intensity);
        light.casts_shadows = desc.get("casts_shadows").unwrap_or(false);
        light.shadow = resolve_shadow_settings(&desc)?;

        let id = w.borrow_mut().lighting.add_directional_light(light);
        Ok(id.raw())
    })?)?;

    // ---------------------------------------------------------------
    // set_intensity(light_id, value) — update light intensity
    // Works for both point and directional lights.
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("set_intensity", lua.create_function(move |_, (_self, id, intensity): (LuaTable, u32, f32)| {
        let mut world = w.borrow_mut();
        let lid = LightId::from_raw(id);
        if let Some(light) = world.lighting.get_light_mut(lid) {
            light.intensity = intensity;
        } else if let Some(light) = world.lighting.get_directional_light_mut(lid) {
            light.intensity = intensity;
        }
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // set_direction(light_id, dx, dy) — update directional light direction
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("set_direction", lua.create_function(move |_, (_self, id, dx, dy): (LuaTable, u32, f32, f32)| {
        let mut world = w.borrow_mut();
        if let Some(light) = world.lighting.get_directional_light_mut(LightId::from_raw(id)) {
            light.direction = Vec2::new(dx, dy).normalized();
        }
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // follow(light_id, obj_id, opts?) — make light track an object
    //   opts = { offset? = {ox, oy} }
    //   Consolidates light_follow + light_follow_with_offset.
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("follow", lua.create_function(move |_, (_self, light_id, obj_id, opts): (LuaTable, u32, u64, Option<LuaTable>)| {
        let offset = match opts {
            Some(t) => match t.get::<Option<LuaTable>>("offset")? {
                Some(ot) => {
                    let ox: f32 = ot.get(1)?;
                    let oy: f32 = ot.get(2)?;
                    Vec2::new(ox, oy)
                }
                None => Vec2::ZERO,
            },
            None => Vec2::ZERO,
        };
        w.borrow_mut().light_follow_with_offset(
            LightId::from_raw(light_id),
            ObjectId::from_raw(obj_id),
            offset,
        );
        Ok(())
    })?)?;

    // ---------------------------------------------------------------
    // unfollow(light_id) — stop a light from tracking an object
    // ---------------------------------------------------------------
    let w = world_rc.clone();
    tbl.set("unfollow", lua.create_function(move |_, (_self, light_id): (LuaTable, u32)| {
        w.borrow_mut().light_unfollow(LightId::from_raw(light_id));
        Ok(())
    })?)?;

    Ok(tbl)
}

// ── Helpers ──

fn read_vec2(table: &LuaTable, key: &str) -> LuaResult<Vec2> {
    let pos: LuaTable = table.get(key)?;
    let x: f32 = pos.get(1)?;
    let y: f32 = pos.get(2)?;
    Ok(Vec2::new(x, y))
}

fn read_light_color(desc: &LuaTable) -> LuaResult<Color> {
    match desc.get::<LuaValue>("color")? {
        LuaValue::Integer(n) => Ok(Color::from_hex(n as u32)),
        LuaValue::Number(n) => Ok(Color::from_hex(n as u32)),
        LuaValue::Nil => Ok(Color::WHITE),
        other => Err(LuaError::FromLuaConversionError {
            from: other.type_name(),
            to: "color".into(),
            message: Some("expected hex integer or nil".into()),
        }),
    }
}

fn resolve_shadow_settings(desc: &LuaTable) -> LuaResult<ShadowSettings> {
    match desc.get::<LuaValue>("shadow")? {
        LuaValue::Nil => Ok(ShadowSettings::default()),
        LuaValue::String(s) => {
            let name = s.to_str()?;
            match &*name {
                "hard" => Ok(ShadowSettings::hard()),
                "soft" => Ok(ShadowSettings::soft()),
                _ => Ok(ShadowSettings::default()),
            }
        }
        LuaValue::Table(t) => {
            let filter_str: String = t.get("filter").unwrap_or_else(|_| "none".to_string());
            let filter = match filter_str.as_str() {
                "pcf5" | "soft" => ShadowFilter::Pcf5,
                "pcf13" => ShadowFilter::Pcf13,
                _ => ShadowFilter::None,
            };
            Ok(ShadowSettings {
                filter,
                strength: t.get("strength").unwrap_or(1.0),
                distance: t.get("distance").unwrap_or(0.0),
                attenuation: t.get("attenuation").unwrap_or(1.0),
            })
        }
        _ => Ok(ShadowSettings::default()),
    }
}
