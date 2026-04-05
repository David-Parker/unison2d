//! Render layer bindings — create layers, queue draw commands.
//!
//! ```lua
//! local sky = world:create_render_layer("sky", { lit = false, clear_color = 0x020206 })
//! world:draw_to(sky, "circle", { x = 0, y = 5, radius = 0.5, color = 0xFFFFAA }, 10)
//! world:set_layer_clear_color(sky, 0x000000)
//! ```

use mlua::prelude::*;
use unison2d::core::{Color, Vec2};
use unison2d::render::RenderCommand;
use unison2d::render::primitives::{circle, gradient_circle};
use unison2d::{RenderLayerConfig, RenderLayerId};

use super::world::LuaWorld;

/// Register render-layer methods on the LuaWorld userdata.
pub fn add_world_methods<M: LuaUserDataMethods<LuaWorld>>(methods: &mut M) {
    methods.add_method("create_render_layer", |_, this, (name, desc): (String, LuaTable)| {
        let lit: bool = desc.get("lit").unwrap_or(true);
        let clear_color = read_layer_color(&desc)?;
        let config = RenderLayerConfig { lit, clear_color };
        let id = this.0.borrow_mut().create_render_layer(&name, config);
        Ok(id.raw())
    });

    methods.add_method("create_render_layer_before", |_, this, (name, before, desc): (String, usize, LuaTable)| {
        let lit: bool = desc.get("lit").unwrap_or(true);
        let clear_color = read_layer_color(&desc)?;
        let config = RenderLayerConfig { lit, clear_color };
        let id = this.0.borrow_mut().create_render_layer_before(
            &name,
            config,
            RenderLayerId::from_raw(before),
        );
        Ok(id.raw())
    });

    methods.add_method("set_layer_clear_color", |_, this, (layer, color): (usize, u32)| {
        this.0.borrow_mut().set_layer_clear_color(
            RenderLayerId::from_raw(layer),
            Color::from_hex(color),
        );
        Ok(())
    });

    methods.add_method("default_layer", |_, this, ()| {
        Ok(this.0.borrow().default_layer().raw())
    });

    // world:draw_to(layer, shape_type, params, z_order)
    methods.add_method("draw_to", |_, this, (layer, shape, params, z): (usize, String, LuaTable, i32)| {
        let cmd = build_render_command(&shape, &params)?;
        this.0.borrow_mut().draw_to(RenderLayerId::from_raw(layer), cmd, z);
        Ok(())
    });

    // world:draw(shape_type, params, z_order) — draw to default layer
    methods.add_method("draw", |_, this, (shape, params, z): (String, LuaTable, i32)| {
        let cmd = build_render_command(&shape, &params)?;
        this.0.borrow_mut().draw(cmd, z);
        Ok(())
    });

    // world:draw_unlit(shape_type, params, z_order)
    methods.add_method("draw_unlit", |_, this, (shape, params, z): (String, LuaTable, i32)| {
        let cmd = build_render_command(&shape, &params)?;
        this.0.borrow_mut().draw_unlit(cmd, z);
        Ok(())
    });
}

// ── Helpers ──

fn read_layer_color(desc: &LuaTable) -> LuaResult<Color> {
    match desc.get::<LuaValue>("clear_color")? {
        LuaValue::Integer(n) => Ok(Color::from_hex(n as u32)),
        LuaValue::Number(n) => Ok(Color::from_hex(n as u32)),
        LuaValue::Nil => Ok(Color::BLACK),
        other => Err(LuaError::FromLuaConversionError {
            from: other.type_name(),
            to: "color".into(),
            message: Some("expected hex integer".into()),
        }),
    }
}

fn build_render_command(shape: &str, params: &LuaTable) -> LuaResult<RenderCommand> {
    match shape {
        "rect" => {
            let x: f32 = params.get("x")?;
            let y: f32 = params.get("y")?;
            let w: f32 = params.get("width")?;
            let h: f32 = params.get("height")?;
            let color = read_param_color(params)?;
            Ok(RenderCommand::Rect {
                position: [x, y],
                size: [w, h],
                color,
            })
        }
        "line" => {
            let x1: f32 = params.get("x1")?;
            let y1: f32 = params.get("y1")?;
            let x2: f32 = params.get("x2")?;
            let y2: f32 = params.get("y2")?;
            let color = read_param_color(params)?;
            let width: f32 = params.get("width").unwrap_or(1.0);
            Ok(RenderCommand::Line {
                start: [x1, y1],
                end: [x2, y2],
                color,
                width,
            })
        }
        "circle" => {
            let x: f32 = params.get("x")?;
            let y: f32 = params.get("y")?;
            let radius: f32 = params.get("radius")?;
            let color = read_param_color(params)?;
            Ok(circle(Vec2::new(x, y), radius, color))
        }
        "gradient_circle" => {
            let x: f32 = params.get("x")?;
            let y: f32 = params.get("y")?;
            let radius: f32 = params.get("radius")?;
            let color = read_param_color(params)?;
            Ok(gradient_circle(Vec2::new(x, y), radius, color))
        }
        other => Err(LuaError::RuntimeError(format!("Unknown shape type: '{other}'"))),
    }
}

fn read_param_color(params: &LuaTable) -> LuaResult<Color> {
    match params.get::<LuaValue>("color")? {
        LuaValue::Integer(n) => Ok(Color::from_hex(n as u32)),
        LuaValue::Number(n) => Ok(Color::from_hex(n as u32)),
        LuaValue::Nil => Ok(Color::WHITE),
        other => Err(LuaError::FromLuaConversionError {
            from: other.type_name(),
            to: "color".into(),
            message: Some("expected hex integer".into()),
        }),
    }
}
