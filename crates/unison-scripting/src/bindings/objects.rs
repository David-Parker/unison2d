//! Object bindings — spawn, despawn, physics, and query methods on World.
//!
//! These are registered as methods on the `LuaWorld` userdata, so Lua calls
//! them as `world:spawn_soft_body(...)`, `world:apply_force(id, fx, fy)`, etc.

use mlua::prelude::*;
use unison2d::core::{Color, Vec2};
use unison2d::physics::{Material, Collider};
use unison2d::physics::mesh::*;
use unison2d::render::TextureId;
use unison2d::{ObjectId, SoftBodyDesc, RigidBodyDesc, SpriteDesc};

use super::world::LuaWorld;

/// Register object-related methods on the LuaWorld userdata.
pub fn add_world_methods<M: LuaUserDataMethods<LuaWorld>>(methods: &mut M) {
    // ---------------------------------------------------------------
    // Spawning
    // ---------------------------------------------------------------

    methods.add_method("spawn_soft_body", |_, this, desc: LuaTable| {
        let mesh = resolve_mesh(&desc)?;
        let material = resolve_material(&desc)?;
        let position = read_vec2(&desc, "position")?;
        let color = read_color(&desc, "color")?.unwrap_or(Color::WHITE);
        let texture_id = desc.get::<u32>("texture").unwrap_or(0);

        let id = this.0.borrow_mut().objects.spawn_soft_body(SoftBodyDesc {
            mesh,
            material,
            position,
            color,
            texture: TextureId::from_raw(texture_id),
        });
        Ok(id.raw())
    });

    methods.add_method("spawn_rigid_body", |_, this, desc: LuaTable| {
        let collider = resolve_collider(&desc)?;
        let position = read_vec2(&desc, "position")?;
        let color = read_color(&desc, "color")?.unwrap_or(Color::WHITE);
        let is_static = desc.get::<bool>("is_static").unwrap_or(false);

        let id = this.0.borrow_mut().objects.spawn_rigid_body(RigidBodyDesc {
            collider,
            position,
            color,
            is_static,
        });
        Ok(id.raw())
    });

    methods.add_method("spawn_static_rect", |_, this, (pos, size, color): (LuaTable, LuaTable, u32)| {
        let position = table_to_vec2(&pos)?;
        let sz = table_to_vec2(&size)?;
        let id = this.0.borrow_mut().objects.spawn_static_rect(
            position, sz, Color::from_hex(color),
        );
        Ok(id.raw())
    });

    methods.add_method("spawn_sprite", |_, this, desc: LuaTable| {
        let texture_id = desc.get::<u32>("texture").unwrap_or(0);
        let position = read_vec2(&desc, "position")?;
        let size = read_vec2(&desc, "size").unwrap_or(Vec2::new(1.0, 1.0));
        let rotation = desc.get::<f32>("rotation").unwrap_or(0.0);
        let color = read_color(&desc, "color")?.unwrap_or(Color::WHITE);

        let id = this.0.borrow_mut().objects.spawn_sprite(SpriteDesc {
            texture: TextureId::from_raw(texture_id),
            position,
            size,
            rotation,
            color,
        });
        Ok(id.raw())
    });

    methods.add_method("despawn", |_, this, id: u64| {
        this.0.borrow_mut().objects.despawn(ObjectId::from_raw(id));
        Ok(())
    });

    // ---------------------------------------------------------------
    // Physics interaction
    // ---------------------------------------------------------------

    methods.add_method("apply_force", |_, this, (id, fx, fy): (u64, f32, f32)| {
        this.0.borrow_mut().objects.apply_force(ObjectId::from_raw(id), Vec2::new(fx, fy));
        Ok(())
    });

    methods.add_method("apply_impulse", |_, this, (id, ix, iy): (u64, f32, f32)| {
        this.0.borrow_mut().objects.apply_impulse(ObjectId::from_raw(id), Vec2::new(ix, iy));
        Ok(())
    });

    methods.add_method("apply_torque", |_, this, (id, torque, dt): (u64, f32, f32)| {
        this.0.borrow_mut().objects.apply_torque(ObjectId::from_raw(id), torque, dt);
        Ok(())
    });

    // ---------------------------------------------------------------
    // Queries
    // ---------------------------------------------------------------

    methods.add_method("get_position", |_, this, id: u64| {
        let pos = this.0.borrow().objects.get_position(ObjectId::from_raw(id));
        Ok((pos.x, pos.y))
    });

    methods.add_method("get_velocity", |_, this, id: u64| {
        let vel = this.0.borrow().objects.get_velocity(ObjectId::from_raw(id));
        Ok((vel.x, vel.y))
    });

    methods.add_method("is_grounded", |_, this, id: u64| {
        Ok(this.0.borrow().objects.is_grounded(ObjectId::from_raw(id)))
    });

    methods.add_method("is_touching", |_, this, (a, b): (u64, u64)| {
        Ok(this.0.borrow().objects.is_touching(
            ObjectId::from_raw(a),
            ObjectId::from_raw(b),
        ))
    });

    // ---------------------------------------------------------------
    // Display properties
    // ---------------------------------------------------------------

    methods.add_method("set_z_order", |_, this, (id, z): (u64, i32)| {
        this.0.borrow_mut().objects.set_z_order(ObjectId::from_raw(id), z);
        Ok(())
    });

    methods.add_method("set_casts_shadow", |_, this, (id, casts): (u64, bool)| {
        this.0.borrow_mut().objects.set_casts_shadow(ObjectId::from_raw(id), casts);
        Ok(())
    });

    methods.add_method("set_position", |_, this, (id, x, y): (u64, f32, f32)| {
        this.0.borrow_mut().objects.set_position(ObjectId::from_raw(id), Vec2::new(x, y));
        Ok(())
    });
}

// ===================================================================
// Helper functions for reading Lua tables into Rust types
// ===================================================================

/// Read a `{x, y}` or `{[1]=x, [2]=y}` table into Vec2.
fn table_to_vec2(t: &LuaTable) -> LuaResult<Vec2> {
    let x: f32 = t.get(1)?;
    let y: f32 = t.get(2)?;
    Ok(Vec2::new(x, y))
}

/// Read a named field that is a `{x, y}` array table.
fn read_vec2(desc: &LuaTable, field: &str) -> LuaResult<Vec2> {
    let t: LuaTable = desc.get(field)?;
    table_to_vec2(&t)
}

/// Read an optional color field. Supports integer hex (0xFF00FF) or nil.
fn read_color(desc: &LuaTable, field: &str) -> LuaResult<Option<Color>> {
    match desc.get::<LuaValue>(field)? {
        LuaValue::Integer(n) => Ok(Some(Color::from_hex(n as u32))),
        LuaValue::Number(n) => Ok(Some(Color::from_hex(n as u32))),
        LuaValue::Nil => Ok(None),
        other => Err(LuaError::FromLuaConversionError {
            from: other.type_name(),
            to: "Color".into(),
            message: Some(format!("expected integer hex color for '{field}'")),
        }),
    }
}

/// Resolve a mesh from a descriptor table's `mesh` and `mesh_params` fields.
///
/// Supported mesh names: `"ring"`, `"square"`, `"ellipse"`, `"star"`, `"blob"`, `"rounded_box"`.
fn resolve_mesh(desc: &LuaTable) -> LuaResult<unison2d::physics::Mesh> {
    let name: String = desc.get("mesh")?;
    let params: LuaTable = desc.get("mesh_params")?;

    match name.as_str() {
        "ring" => {
            let outer: f32 = params.get(1)?;
            let inner: f32 = params.get(2)?;
            let segments: u32 = params.get(3)?;
            let radial: u32 = params.get(4)?;
            Ok(create_ring_mesh(outer, inner, segments, radial))
        }
        "square" => {
            let size: f32 = params.get(1)?;
            let divisions: u32 = params.get(2).unwrap_or(4);
            Ok(create_square_mesh(size, divisions))
        }
        "ellipse" => {
            let rx: f32 = params.get(1)?;
            let ry: f32 = params.get(2)?;
            let segments: u32 = params.get(3)?;
            let rings: u32 = params.get(4)?;
            Ok(create_ellipse_mesh(rx, ry, segments, rings))
        }
        "star" => {
            let outer: f32 = params.get(1)?;
            let inner: f32 = params.get(2)?;
            let points: u32 = params.get(3)?;
            let divisions: u32 = params.get(4).unwrap_or(4);
            Ok(create_star_mesh(outer, inner, points, divisions))
        }
        "blob" => {
            let radius: f32 = params.get(1)?;
            let variation: f32 = params.get(2)?;
            let segments: u32 = params.get(3)?;
            let rings: u32 = params.get(4)?;
            let seed: u32 = params.get(5).unwrap_or(42);
            Ok(create_blob_mesh(radius, variation, segments, rings, seed))
        }
        "rounded_box" => {
            let w: f32 = params.get(1)?;
            let h: f32 = params.get(2)?;
            let corner_r: f32 = params.get(3)?;
            let corner_seg: u32 = params.get(4)?;
            Ok(create_rounded_box_mesh(w, h, corner_r, corner_seg))
        }
        other => Err(LuaError::RuntimeError(format!(
            "unknown mesh type: '{other}'. Expected one of: ring, square, ellipse, star, blob, rounded_box"
        ))),
    }
}

/// Resolve material from a descriptor table's `material` field.
///
/// Accepts a string preset (`"rubber"`, `"jello"`, `"wood"`, `"metal"`, `"slime"`)
/// or a table `{density, edge_compliance, area_compliance}`.
fn resolve_material(desc: &LuaTable) -> LuaResult<Material> {
    let val: LuaValue = desc.get("material")?;
    match val {
        LuaValue::String(s) => {
            let name = s.to_str()?;
            match &*name {
                "rubber" => Ok(Material::RUBBER),
                "jello" => Ok(Material::JELLO),
                "wood" => Ok(Material::WOOD),
                "metal" => Ok(Material::METAL),
                "slime" => Ok(Material::SLIME),
                other => Err(LuaError::RuntimeError(format!(
                    "unknown material preset: '{other}'. Expected: rubber, jello, wood, metal, slime"
                ))),
            }
        }
        LuaValue::Table(t) => {
            let density: f32 = t.get("density")?;
            let edge_compliance: f32 = t.get("edge_compliance")?;
            let area_compliance: f32 = t.get("area_compliance")?;
            Ok(Material { density, edge_compliance, area_compliance })
        }
        LuaValue::Nil => Ok(Material::default()),
        other => Err(LuaError::FromLuaConversionError {
            from: other.type_name(),
            to: "Material".into(),
            message: Some("expected string preset or {density, edge_compliance, area_compliance} table".into()),
        }),
    }
}

/// Resolve collider from a descriptor table.
///
/// Supports `collider = "circle"` with `radius` field, or `collider = "aabb"` with
/// `half_width`/`half_height` fields.
fn resolve_collider(desc: &LuaTable) -> LuaResult<Collider> {
    let kind: String = desc.get("collider")?;
    match kind.as_str() {
        "circle" => {
            let radius: f32 = desc.get("radius")?;
            Ok(Collider::circle(radius))
        }
        "aabb" => {
            let hw: f32 = desc.get("half_width")?;
            let hh: f32 = desc.get("half_height")?;
            Ok(Collider::aabb(hw, hh))
        }
        other => Err(LuaError::RuntimeError(format!(
            "unknown collider type: '{other}'. Expected: circle, aabb"
        ))),
    }
}
