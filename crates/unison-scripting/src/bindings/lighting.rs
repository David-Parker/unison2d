//! Lighting bindings — point lights, directional lights, shadows.
//!
//! Registered as methods on the `LuaWorld` userdata:
//!
//! ```lua
//! world:lighting_set_enabled(true)
//! world:lighting_set_ambient(0.1, 0.1, 0.15, 1.0)
//! world:lighting_set_ground_shadow(ground_y)
//!
//! local light = world:add_point_light({
//!     position = {0, 5}, color = 0xFFDD44,
//!     intensity = 2.0, radius = 8.0,
//!     casts_shadows = true, shadow = "soft",
//! })
//!
//! local dir = world:add_directional_light({
//!     direction = {-0.5, -1.0}, color = 0xFFFFFF,
//!     intensity = 0.8, casts_shadows = true,
//! })
//!
//! world:set_light_intensity(light, 3.0)
//! world:set_directional_light_direction(dir, -0.7, -1.0)
//! world:light_follow(light, donut)
//! world:light_follow_with_offset(light, donut, 0, 2)
//! world:light_unfollow(light)
//! ```

use mlua::prelude::*;
use unison2d::core::{Color, Vec2};
use unison2d::lighting::{LightId, PointLight, DirectionalLight, ShadowSettings};
use unison2d::lighting::occluder::ShadowFilter;
use unison2d::ObjectId;

use super::world::LuaWorld;

/// Register lighting-related methods on the LuaWorld userdata.
pub fn add_world_methods<M: LuaUserDataMethods<LuaWorld>>(methods: &mut M) {
    // -- System config --

    methods.add_method("lighting_set_enabled", |_, this, enabled: bool| {
        this.0.borrow_mut().lighting.set_enabled(enabled);
        Ok(())
    });

    methods.add_method("lighting_set_ambient", |_, this, (r, g, b, a): (f32, f32, f32, f32)| {
        this.0.borrow_mut().lighting.set_ambient(Color::new(r, g, b, a));
        Ok(())
    });

    methods.add_method("lighting_set_ground_shadow", |_, this, y: LuaValue| {
        let mut world = this.0.borrow_mut();
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
    });

    // -- Add lights --

    methods.add_method("add_point_light", |_, this, desc: LuaTable| {
        let position = read_vec2(&desc, "position")?;
        let color = read_light_color(&desc)?;
        let intensity: f32 = desc.get("intensity").unwrap_or(1.0);
        let radius: f32 = desc.get("radius").unwrap_or(5.0);

        let mut light = PointLight::new(position, color, intensity, radius);
        light.casts_shadows = desc.get("casts_shadows").unwrap_or(false);
        light.shadow = resolve_shadow_settings(&desc)?;

        let id = this.0.borrow_mut().lighting.add_light(light);
        Ok(id.raw())
    });

    methods.add_method("add_directional_light", |_, this, desc: LuaTable| {
        let direction = read_vec2(&desc, "direction")?;
        let color = read_light_color(&desc)?;
        let intensity: f32 = desc.get("intensity").unwrap_or(1.0);

        let mut light = DirectionalLight::new(direction, color, intensity);
        light.casts_shadows = desc.get("casts_shadows").unwrap_or(false);
        light.shadow = resolve_shadow_settings(&desc)?;

        let id = this.0.borrow_mut().lighting.add_directional_light(light);
        Ok(id.raw())
    });

    // -- Modify lights --

    methods.add_method("set_light_intensity", |_, this, (id, intensity): (u32, f32)| {
        let mut world = this.0.borrow_mut();
        let lid = LightId::from_raw(id);
        if let Some(light) = world.lighting.get_light_mut(lid) {
            light.intensity = intensity;
        } else if let Some(light) = world.lighting.get_directional_light_mut(lid) {
            light.intensity = intensity;
        }
        Ok(())
    });

    methods.add_method("set_directional_light_direction", |_, this, (id, dx, dy): (u32, f32, f32)| {
        let mut world = this.0.borrow_mut();
        if let Some(light) = world.lighting.get_directional_light_mut(LightId::from_raw(id)) {
            light.direction = Vec2::new(dx, dy).normalized();
        }
        Ok(())
    });

    // -- Light follow --

    methods.add_method("light_follow", |_, this, (light_id, obj_id): (u32, u64)| {
        this.0.borrow_mut().light_follow(
            LightId::from_raw(light_id),
            ObjectId::from_raw(obj_id),
        );
        Ok(())
    });

    methods.add_method("light_follow_with_offset", |_, this, (light_id, obj_id, ox, oy): (u32, u64, f32, f32)| {
        this.0.borrow_mut().light_follow_with_offset(
            LightId::from_raw(light_id),
            ObjectId::from_raw(obj_id),
            Vec2::new(ox, oy),
        );
        Ok(())
    });

    methods.add_method("light_unfollow", |_, this, light_id: u32| {
        this.0.borrow_mut().light_unfollow(LightId::from_raw(light_id));
        Ok(())
    });
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
