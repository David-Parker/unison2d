//! Render target & compositing bindings.
//!
//! ```lua
//! local target, tex = engine.create_render_target(256, 256)
//!
//! -- In render:
//! world:render_to_targets({
//!     {"main", "screen"},
//!     {"overview", target},
//! })
//! engine.draw_overlay(tex, 0.7, 0.7, 0.25, 0.25)
//! ```
//!
//! Render targets require renderer access, so `create_render_target` and
//! `draw_overlay` are deferred through thread-local state, resolved by
//! `ScriptedGame` during the init/render phases.

use std::cell::RefCell;

use mlua::prelude::*;
use unison2d::render::RenderTargetId;

// ===================================================================
// Thread-local bridge state
// ===================================================================

/// A pending render-target creation request.
pub struct RenderTargetRequest {
    pub width: u32,
    pub height: u32,
}

/// A pending overlay draw request.
pub struct OverlayRequest {
    pub texture_id: u32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub border: Option<OverlayBorder>,
}

pub struct OverlayBorder {
    pub width: f32,
    pub color: u32,
}

thread_local! {
    /// Pending render target creation requests, resolved during init.
    static RT_REQUESTS: RefCell<Vec<RenderTargetRequest>> = const { RefCell::new(Vec::new()) };
    /// Results from render target creation: (target_id_raw, texture_id_raw).
    static RT_RESULTS: RefCell<Vec<(u32, u32)>> = const { RefCell::new(Vec::new()) };
    /// Pending overlay draw requests, consumed during render.
    static OVERLAY_REQUESTS: RefCell<Vec<OverlayRequest>> = const { RefCell::new(Vec::new()) };
    /// Pending render_to_targets calls: list of (camera_name, target_id_raw).
    static RENDER_TO_TARGETS: RefCell<Option<Vec<(String, u32)>>> = const { RefCell::new(None) };
}

// -- Public accessors for ScriptedGame --

pub fn take_render_target_requests() -> Vec<RenderTargetRequest> {
    RT_REQUESTS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

pub fn push_render_target_result(target_id: u32, texture_id: u32) {
    RT_RESULTS.with(|cell| cell.borrow_mut().push((target_id, texture_id)));
}

pub fn take_overlay_requests() -> Vec<OverlayRequest> {
    OVERLAY_REQUESTS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

pub fn take_render_to_targets() -> Option<Vec<(String, u32)>> {
    RENDER_TO_TARGETS.with(|cell| cell.borrow_mut().take())
}

// ===================================================================
// Registration
// ===================================================================

/// Register render target functions on the `engine` global table.
/// Must be called after `engine::register()`.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let engine: LuaTable = lua.globals().get("engine")?;

    // engine.create_render_target(w, h) → target_id, texture_id
    // During init, this is resolved synchronously via the engine pointer.
    // We queue the request and return the result from the results buffer.
    engine.set("create_render_target", lua.create_function(|_, (w, h): (u32, u32)| {
        let idx = RT_REQUESTS.with(|cell| {
            let mut v = cell.borrow_mut();
            let idx = v.len();
            v.push(RenderTargetRequest { width: w, height: h });
            idx
        });
        // Check if result is already available (synchronous path)
        RT_RESULTS.with(|cell| {
            let results = cell.borrow();
            if idx < results.len() {
                Ok((results[idx].0, results[idx].1))
            } else {
                // Not yet resolved — return placeholders
                Ok((0u32, 0u32))
            }
        })
    })?)?;

    // engine.draw_overlay(texture_id, x, y, w, h)
    engine.set("draw_overlay", lua.create_function(|_, (tex, x, y, w, h): (u32, f32, f32, f32, f32)| {
        OVERLAY_REQUESTS.with(|cell| {
            cell.borrow_mut().push(OverlayRequest {
                texture_id: tex,
                x, y, w, h,
                border: None,
            });
        });
        Ok(())
    })?)?;

    // engine.draw_overlay_bordered(texture_id, x, y, w, h, border_width, border_color)
    engine.set("draw_overlay_bordered", lua.create_function(|_, (tex, x, y, w, h, bw, bc): (u32, f32, f32, f32, f32, f32, u32)| {
        OVERLAY_REQUESTS.with(|cell| {
            cell.borrow_mut().push(OverlayRequest {
                texture_id: tex,
                x, y, w, h,
                border: Some(OverlayBorder { width: bw, color: bc }),
            });
        });
        Ok(())
    })?)?;

    Ok(())
}

/// Register render_to_targets as a method on World userdata.
pub fn add_world_methods<M: LuaUserDataMethods<super::world::LuaWorld>>(methods: &mut M) {
    // world:render_to_targets({{"main", "screen"}, {"overview", target_id}})
    methods.add_method("render_to_targets", |_, _this, mapping: LuaTable| {
        let mut targets = Vec::new();
        for pair in mapping.sequence_values::<LuaTable>() {
            let pair = pair?;
            let camera_name: String = pair.get(1)?;
            let target_val: LuaValue = pair.get(2)?;

            let target_raw = match target_val {
                LuaValue::String(s) if &*s.to_str()? == "screen" => RenderTargetId::SCREEN.raw(),
                LuaValue::Integer(n) => n as u32,
                LuaValue::Number(n) => n as u32,
                _ => RenderTargetId::SCREEN.raw(),
            };

            targets.push((camera_name, target_raw));
        }

        RENDER_TO_TARGETS.with(|cell| {
            *cell.borrow_mut() = Some(targets);
        });
        Ok(())
    });
}
