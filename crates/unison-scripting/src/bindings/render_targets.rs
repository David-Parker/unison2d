//! Render target & compositing bindings.
//!
//! ```lua
//! local target, tex = unison.renderer.create_target(256, 256)
//!
//! -- In render:
//! world:render_to_targets({
//!     {"main", "screen"},
//!     {"overview", target},
//! })
//! world:draw_overlay(tex, 0.7, 0.7, 0.25, 0.25)
//! ```
//!
//! `create_target` is in `renderer.rs` (synchronous, engine-ptr pattern).
//! `draw_overlay` / `draw_overlay_bordered` are methods on World userdata (world.rs).
//! `render_to_targets` is registered as a World method here.
//! Overlay and render-to-target requests are deferred into thread-local queues
//! that `ScriptedGame` drains during the render phase.

use std::cell::RefCell;

use mlua::prelude::*;
use unison2d::render::RenderTargetId;

// ===================================================================
// Thread-local bridge state
// ===================================================================

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
    /// Pending overlay draw requests, consumed during render.
    pub(super) static OVERLAY_REQUESTS: RefCell<Vec<OverlayRequest>> = const { RefCell::new(Vec::new()) };
    /// Pending render_to_targets calls: list of (camera_name, target_id_raw).
    static RENDER_TO_TARGETS: RefCell<Option<Vec<(String, u32)>>> = const { RefCell::new(None) };
}

// -- Public helpers for pushing overlay requests (called from world.rs methods) --

pub fn push_overlay(request: OverlayRequest) {
    OVERLAY_REQUESTS.with(|cell| cell.borrow_mut().push(request));
}

// -- Public accessors for ScriptedGame --

pub fn take_overlay_requests() -> Vec<OverlayRequest> {
    OVERLAY_REQUESTS.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
}

pub fn take_render_to_targets() -> Option<Vec<(String, u32)>> {
    RENDER_TO_TARGETS.with(|cell| cell.borrow_mut().take())
}

/// Reset all render-target thread-local state.
/// Called from `ScriptedGame::drop()`.
pub fn reset() {
    OVERLAY_REQUESTS.with(|cell| cell.borrow_mut().clear());
    RENDER_TO_TARGETS.with(|cell| *cell.borrow_mut() = None);
}

// ===================================================================
// World method registration
// ===================================================================

/// Register render_to_targets as a method on World userdata.
pub fn add_world_methods<M: LuaUserDataMethods<super::world::LuaWorld>>(methods: &mut M) {
    // world:render_to_targets({{"main", "screen"}, {"overview", target_id}})
    methods.add_method("render_to_targets", |_, this, mapping: LuaTable| {
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

        // Ensure the render pass runs even when the script uses multi-camera
        // rendering instead of world:render().
        super::engine_state::request_render(this.0.clone());

        Ok(())
    });
}
