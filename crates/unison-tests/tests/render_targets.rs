//! Integration tests for multi-camera render-to-targets.
//!
//! Regression tests for a bug where rendering to multiple targets via
//! `render_to_targets()` caused:
//! - Output targets never cleared (artifacts accumulate across frames)
//! - White/stale background in PiP because the lit-layer composite path
//!   only clears the intermediate scene FBO, not the output target itself

use std::cell::RefCell;
use std::rc::Rc;
use unison2d::World;
use unison_core::{Color, Vec2};
use unison_render::{
    BlendMode, Camera, DrawSprite, RenderCommand, RenderTargetId, Renderer,
    TextureDescriptor, TextureId,
};

// ── Mock renderer ──

#[derive(Debug, Clone)]
enum RenderOp {
    BindTarget(RenderTargetId),
    BeginFrame,
    Clear(Color),
    SetBlend(BlendMode),
    DrawSprite { texture: TextureId },
    DrawRect { color: Color },
    EndFrame,
}

struct MockRenderer {
    ops: Rc<RefCell<Vec<RenderOp>>>,
    next_texture_id: u32,
    next_rt_id: u32,
}

impl MockRenderer {
    fn new() -> Self {
        Self {
            ops: Rc::new(RefCell::new(Vec::new())),
            next_texture_id: 100,
            next_rt_id: 100,
        }
    }

    fn ops(&self) -> Vec<RenderOp> {
        self.ops.borrow().clone()
    }

    fn clear_ops(&self) {
        self.ops.borrow_mut().clear();
    }
}

impl Renderer for MockRenderer {
    type Error = String;

    fn init(&mut self) -> Result<(), String> { Ok(()) }

    fn begin_frame(&mut self, _camera: &Camera) {
        self.ops.borrow_mut().push(RenderOp::BeginFrame);
    }

    fn clear(&mut self, color: Color) {
        self.ops.borrow_mut().push(RenderOp::Clear(color));
    }

    fn draw(&mut self, command: RenderCommand) {
        let op = match command {
            RenderCommand::Sprite(s) => RenderOp::DrawSprite { texture: s.texture },
            RenderCommand::Rect { color, .. } => RenderOp::DrawRect { color },
            _ => return,
        };
        self.ops.borrow_mut().push(op);
    }

    fn end_frame(&mut self) {
        self.ops.borrow_mut().push(RenderOp::EndFrame);
    }

    fn create_texture(&mut self, _desc: &TextureDescriptor) -> Result<TextureId, String> {
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        Ok(TextureId(id))
    }

    fn destroy_texture(&mut self, _id: TextureId) {}

    fn screen_size(&self) -> (f32, f32) {
        (960.0, 540.0)
    }

    fn set_screen_size(&mut self, _width: f32, _height: f32) {}

    fn set_blend_mode(&mut self, mode: BlendMode) {
        self.ops.borrow_mut().push(RenderOp::SetBlend(mode));
    }

    fn create_render_target(&mut self, _w: u32, _h: u32) -> Result<(RenderTargetId, TextureId), String> {
        let rt = self.next_rt_id;
        self.next_rt_id += 1;
        let tex = self.next_texture_id;
        self.next_texture_id += 1;
        Ok((RenderTargetId(rt), TextureId(tex)))
    }

    fn bind_render_target(&mut self, target: RenderTargetId) {
        self.ops.borrow_mut().push(RenderOp::BindTarget(target));
    }

    fn destroy_render_target(&mut self, _target: RenderTargetId) {}
}

// ── Helper to find all Clear ops targeting a specific render target ──

/// Collect all `Clear(color)` ops that occur while `target` is bound.
fn clears_for_target(ops: &[RenderOp], target: RenderTargetId) -> Vec<Color> {
    let mut current_target = RenderTargetId::SCREEN;
    let mut clears = Vec::new();
    for op in ops {
        match op {
            RenderOp::BindTarget(t) => current_target = *t,
            RenderOp::Clear(c) if current_target == target => clears.push(*c),
            _ => {}
        }
    }
    clears
}

/// Returns true if the ops contain a Clear for the given target with an
/// opaque color (i.e. not TRANSPARENT).
fn has_opaque_clear_for(ops: &[RenderOp], target: RenderTargetId) -> bool {
    clears_for_target(ops, target).iter().any(|c| c.a > 0.0)
}

/// Returns true if any `Clear` on `target` occurs in a BeginFrame..EndFrame
/// span that contains no draw commands (Sprite, Rect, etc.).  Such an isolated
/// clear produces a single-frame flash of the background color — i.e. flicker.
fn has_clear_only_frame_for(ops: &[RenderOp], target: RenderTargetId) -> bool {
    let mut current_target = RenderTargetId::SCREEN;
    let mut in_frame = false;
    let mut frame_has_clear_on_target = false;
    let mut frame_has_draw = false;

    for op in ops {
        match op {
            RenderOp::BindTarget(t) => current_target = *t,
            RenderOp::BeginFrame => {
                in_frame = true;
                frame_has_clear_on_target = false;
                frame_has_draw = false;
            }
            RenderOp::Clear(_) if in_frame && current_target == target => {
                frame_has_clear_on_target = true;
            }
            RenderOp::DrawSprite { .. } | RenderOp::DrawRect { .. } if in_frame => {
                frame_has_draw = true;
            }
            RenderOp::EndFrame if in_frame => {
                if frame_has_clear_on_target && !frame_has_draw {
                    return true;
                }
                in_frame = false;
            }
            _ => {}
        }
    }
    false
}

// ── Tests ──

/// The default World has a single lit "scene" layer. When rendered via
/// `render` to SCREEN, the output target must be cleared with the
/// background color before the lit-layer composite. Without this clear,
/// stale content from previous frames bleeds through.
#[test]
fn render_clears_screen_with_background_color() {
    let mut renderer = MockRenderer::new();
    let mut world = World::new();
    let bg = Color::from_hex(0x1a1a2e);
    world.set_background(bg);

    // Queue a draw command so the lit layer has content and the FBO path runs
    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);

    world.render(&mut renderer);

    let ops = renderer.ops();
    let screen_clears = clears_for_target(&ops, RenderTargetId::SCREEN);

    assert!(
        !screen_clears.is_empty(),
        "SCREEN must be cleared at least once; got ops: {ops:#?}"
    );
    assert!(
        has_opaque_clear_for(&ops, RenderTargetId::SCREEN),
        "SCREEN must receive an opaque clear (background color), not just TRANSPARENT.\n\
         Clears found: {screen_clears:?}"
    );
}

/// When rendering to multiple targets via `render_to_targets`, EVERY output
/// target must be cleared with the background color. This prevents artifacts
/// from accumulating in PiP targets across frames.
#[test]
fn render_to_targets_clears_each_output_target() {
    let mut renderer = MockRenderer::new();
    let mut world = World::new();
    let bg = Color::from_hex(0x1a1a2e);
    world.set_background(bg);

    // Add a second camera for PiP
    world.cameras.add("overview", Camera::new(20.0, 15.0));

    // Create a PiP render target
    let (pip_target, _pip_tex) = renderer.create_render_target(240, 135).unwrap();

    // Queue a draw command so the lit layer has content
    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);

    world.render_to_targets(&mut renderer, &[
        ("overview", pip_target),
        ("main", RenderTargetId::SCREEN),
    ]);

    let ops = renderer.ops();

    // Both targets must receive an opaque clear
    assert!(
        has_opaque_clear_for(&ops, pip_target),
        "PiP target must be cleared with an opaque background color.\n\
         PiP clears: {:?}\nAll ops: {ops:#?}",
        clears_for_target(&ops, pip_target),
    );
    assert!(
        has_opaque_clear_for(&ops, RenderTargetId::SCREEN),
        "SCREEN must be cleared with an opaque background color.\n\
         Screen clears: {:?}\nAll ops: {ops:#?}",
        clears_for_target(&ops, RenderTargetId::SCREEN),
    );
}

/// Running multiple frames must clear each target every frame — no artifact
/// accumulation from the previous frame.
#[test]
fn render_to_targets_clears_every_frame() {
    let mut renderer = MockRenderer::new();
    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.cameras.add("overview", Camera::new(20.0, 15.0));
    let (pip_target, _pip_tex) = renderer.create_render_target(240, 135).unwrap();

    for frame in 0..3 {
        // Queue fresh content each frame
        world.draw(RenderCommand::Rect {
            position: [0.0, 0.0],
            size: [1.0, 1.0],
            color: Color::RED,
        }, 0);

        renderer.clear_ops();
        world.render_to_targets(&mut renderer, &[
            ("overview", pip_target),
            ("main", RenderTargetId::SCREEN),
        ]);

        let ops = renderer.ops();
        assert!(
            has_opaque_clear_for(&ops, pip_target),
            "Frame {frame}: PiP target must be cleared.\n\
             PiP clears: {:?}",
            clears_for_target(&ops, pip_target),
        );
        assert!(
            has_opaque_clear_for(&ops, RenderTargetId::SCREEN),
            "Frame {frame}: SCREEN must be cleared.\n\
             Screen clears: {:?}",
            clears_for_target(&ops, RenderTargetId::SCREEN),
        );
    }
}

/// With only lit layers (the default), the output target must still be
/// cleared. This is the core regression: the lit-layer code path clears
/// the intermediate scene FBO with TRANSPARENT but never clears the
/// actual output target.
#[test]
fn lit_only_world_clears_output_target() {
    let mut renderer = MockRenderer::new();
    let mut world = World::new();
    // Default world has exactly one layer: the lit "scene" layer.
    // Set a distinctive background so we can verify the clear color.
    let bg = Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 };
    world.set_background(bg);

    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);

    world.render(&mut renderer);

    let ops = renderer.ops();
    let screen_clears = clears_for_target(&ops, RenderTargetId::SCREEN);

    // The output must be cleared with the opaque background before compositing
    assert!(
        screen_clears.iter().any(|c| c.a == 1.0 && (c.r - bg.r).abs() < 0.01),
        "SCREEN must be cleared with the layer's background color ({bg:?}).\n\
         Clears found: {screen_clears:?}"
    );
}

// ── Flicker regression tests ──
//
// A "clear-only frame" is a BeginFrame..EndFrame span on an output target
// that contains a Clear but no draw commands.  This produces a single-frame
// flash of the background color (flicker).  The clear must always be in the
// same frame as the composite draw.

/// render must not produce a clear-only frame on SCREEN.
#[test]
fn render_no_flicker_on_screen() {
    let mut renderer = MockRenderer::new();
    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));

    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);

    world.render(&mut renderer);

    assert!(
        !has_clear_only_frame_for(&renderer.ops(), RenderTargetId::SCREEN),
        "SCREEN has a clear-only frame (would cause flicker).\nOps: {:#?}",
        renderer.ops(),
    );
}

/// render_to_targets must not produce a clear-only frame on any target.
#[test]
fn render_to_targets_no_flicker() {
    let mut renderer = MockRenderer::new();
    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.cameras.add("overview", Camera::new(20.0, 15.0));
    let (pip_target, _pip_tex) = renderer.create_render_target(240, 135).unwrap();

    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);

    world.render_to_targets(&mut renderer, &[
        ("overview", pip_target),
        ("main", RenderTargetId::SCREEN),
    ]);

    let ops = renderer.ops();
    assert!(
        !has_clear_only_frame_for(&ops, pip_target),
        "PiP target has a clear-only frame (would cause flicker).\nOps: {ops:#?}",
    );
    assert!(
        !has_clear_only_frame_for(&ops, RenderTargetId::SCREEN),
        "SCREEN has a clear-only frame (would cause flicker).\nOps: {ops:#?}",
    );
}
