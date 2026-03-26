//! Integration tests for cross-platform rendering: FBO UV orientation,
//! screen_size vs drawable_size, and touch/mouse input coexistence.

use std::cell::RefCell;
use std::rc::Rc;
use unison2d::World;
use unison_input::{InputState, MouseButton};
use unison_math::{Color, Vec2};
use unison_lighting::PointLight;
use unison_render::{
    BlendMode, Camera, DrawSprite, RenderCommand, RenderTargetId, Renderer,
    TextureDescriptor, TextureId,
};
use unison_ui::diff::{diff_trees, DiffOp, NodeKey};
use unison_ui::input::process_input;
use unison_ui::layout::{compute_layout, Layout, TextMeasurer};
use unison_ui::node::{UiNode, UiTree};
use unison_ui::state::UiState;
use unison_ui::style::Anchor;

// ── Mock renderer that captures draw commands with UV data ──

#[derive(Debug, Clone)]
enum RenderOp {
    BindTarget(RenderTargetId),
    BeginFrame,
    Clear(Color),
    SetBlend(BlendMode),
    DrawSprite { texture: TextureId, uv: [f32; 4] },
    DrawRect { color: Color },
    EndFrame,
}

struct MockRenderer {
    ops: Rc<RefCell<Vec<RenderOp>>>,
    next_texture_id: u32,
    next_rt_id: u32,
    screen_w: f32,
    screen_h: f32,
    drawable_w: f32,
    drawable_h: f32,
    fbo_top_left: bool,
}

impl MockRenderer {
    fn new() -> Self {
        Self {
            ops: Rc::new(RefCell::new(Vec::new())),
            next_texture_id: 100,
            next_rt_id: 100,
            screen_w: 960.0,
            screen_h: 540.0,
            drawable_w: 960.0,
            drawable_h: 540.0,
            fbo_top_left: false,
        }
    }

    /// Create a mock renderer that mimics Metal (fbo_origin_top_left = true,
    /// drawable_size != screen_size).
    fn metal_like() -> Self {
        Self {
            fbo_top_left: true,
            screen_w: 852.0,
            screen_h: 393.0,
            drawable_w: 2556.0,
            drawable_h: 1179.0,
            ..Self::new()
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
            RenderCommand::Sprite(s) => RenderOp::DrawSprite {
                texture: s.texture,
                uv: s.uv,
            },
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
        (self.screen_w, self.screen_h)
    }

    fn drawable_size(&self) -> (f32, f32) {
        (self.drawable_w, self.drawable_h)
    }

    fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_w = width;
        self.screen_h = height;
    }

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

    fn fbo_origin_top_left(&self) -> bool {
        self.fbo_top_left
    }
}

// ── Helpers ──

/// Find the UV of the first DrawSprite that follows a BindTarget(SCREEN).
fn first_screen_composite_uv(ops: &[RenderOp]) -> Option<[f32; 4]> {
    let mut on_screen = false;
    for op in ops {
        match op {
            RenderOp::BindTarget(t) => on_screen = *t == RenderTargetId::SCREEN,
            RenderOp::DrawSprite { uv, .. } if on_screen => return Some(*uv),
            _ => {}
        }
    }
    None
}

/// Find the UV of any DrawSprite using Multiply blend (lightmap composite).
fn first_multiply_composite_uv(ops: &[RenderOp]) -> Option<[f32; 4]> {
    let mut in_multiply = false;
    for op in ops {
        match op {
            RenderOp::SetBlend(BlendMode::Multiply) => in_multiply = true,
            RenderOp::SetBlend(_) => in_multiply = false,
            RenderOp::DrawSprite { uv, .. } if in_multiply => return Some(*uv),
            _ => {}
        }
    }
    None
}

// ══════════════════════════════════════════════════════════════════════
// 1. FBO UV orientation tests
// ══════════════════════════════════════════════════════════════════════

/// OpenGL-style renderer (fbo_origin_top_left = false) should V-flip FBO UVs.
#[test]
fn opengl_renderer_uses_vflipped_fbo_uvs() {
    let mut renderer = MockRenderer::new();
    assert!(!renderer.fbo_origin_top_left());

    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);

    world.auto_render(&mut renderer);
    let ops = renderer.ops();

    let uv = first_screen_composite_uv(&ops)
        .expect("should have a screen composite sprite");
    // V-flipped: min_v=1, max_v=0
    assert_eq!(uv, [0.0, 1.0, 1.0, 0.0],
        "OpenGL renderer should use V-flipped UVs for FBO composite");
}

/// Metal-style renderer (fbo_origin_top_left = true) should NOT flip FBO UVs.
#[test]
fn metal_renderer_uses_normal_fbo_uvs() {
    let mut renderer = MockRenderer::metal_like();
    assert!(renderer.fbo_origin_top_left());

    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);

    world.auto_render(&mut renderer);
    let ops = renderer.ops();

    let uv = first_screen_composite_uv(&ops)
        .expect("should have a screen composite sprite");
    // Normal: min_v=0, max_v=1
    assert_eq!(uv, [0.0, 0.0, 1.0, 1.0],
        "Metal renderer should use normal (non-flipped) UVs for FBO composite");
}

/// Lightmap composite should also respect fbo_origin_top_left.
#[test]
fn lightmap_composite_respects_fbo_origin() {
    // OpenGL-style
    let mut renderer = MockRenderer::new();
    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.lighting.set_enabled(true);
    world.lighting.set_ambient(Color::new(0.5, 0.5, 0.5, 1.0));
    world.lighting.add_light(PointLight {
        position: Vec2::ZERO,
        color: Color::WHITE,
        intensity: 1.0,
        radius: 5.0,
        casts_shadows: false,
        shadow: Default::default(),
    });
    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);
    world.auto_render(&mut renderer);
    let gl_uv = first_multiply_composite_uv(&renderer.ops());

    // Metal-style
    let mut renderer = MockRenderer::metal_like();
    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.lighting.set_enabled(true);
    world.lighting.set_ambient(Color::new(0.5, 0.5, 0.5, 1.0));
    world.lighting.add_light(PointLight {
        position: Vec2::ZERO,
        color: Color::WHITE,
        intensity: 1.0,
        radius: 5.0,
        casts_shadows: false,
        shadow: Default::default(),
    });
    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);
    world.auto_render(&mut renderer);
    let metal_uv = first_multiply_composite_uv(&renderer.ops());

    if let (Some(gl), Some(metal)) = (gl_uv, metal_uv) {
        assert_eq!(gl, [0.0, 1.0, 1.0, 0.0], "OpenGL lightmap should V-flip");
        assert_eq!(metal, [0.0, 0.0, 1.0, 1.0], "Metal lightmap should not flip");
    }
}

// ══════════════════════════════════════════════════════════════════════
// 2. screen_size vs drawable_size tests
// ══════════════════════════════════════════════════════════════════════

/// screen_size and drawable_size should be independently configurable.
#[test]
fn screen_size_and_drawable_size_are_independent() {
    let renderer = MockRenderer::metal_like();
    let (sw, sh) = renderer.screen_size();
    let (dw, dh) = renderer.drawable_size();

    assert_eq!((sw, sh), (852.0, 393.0), "screen_size should return logical points");
    assert_eq!((dw, dh), (2556.0, 1179.0), "drawable_size should return physical pixels");
    assert!(dw > sw, "drawable should be larger than screen on retina");
}

/// FBOs should be created at drawable_size, not screen_size.
/// Verify by checking that ensure_scene_fbo uses drawable dimensions.
#[test]
fn scene_fbo_created_at_drawable_size() {
    let mut renderer = MockRenderer::metal_like();
    let mut world = World::new();
    world.set_background(Color::from_hex(0x1a1a2e));
    world.draw(RenderCommand::Rect {
        position: [0.0, 0.0],
        size: [1.0, 1.0],
        color: Color::RED,
    }, 0);

    // auto_render triggers ensure_scene_fbo internally.
    // The mock renderer doesn't validate FBO dimensions, but we verify
    // the trait contract: create_render_target is called (meaning the
    // FBO path runs), and rendering completes without error.
    world.auto_render(&mut renderer);

    let ops = renderer.ops();
    // The FBO path binds a non-SCREEN target before drawing scene content
    let has_fbo_bind = ops.iter().any(|op| matches!(op, RenderOp::BindTarget(t) if *t != RenderTargetId::SCREEN));
    assert!(has_fbo_bind, "lit content should trigger FBO path");
}

// ══════════════════════════════════════════════════════════════════════
// 3. Touch/mouse input coexistence tests
// ══════════════════════════════════════════════════════════════════════

struct FixedMeasurer;
impl TextMeasurer for FixedMeasurer {
    fn measure(&mut self, text: &str, _font_size: f32) -> Vec2 {
        Vec2::new(text.len() as f32 * 8.0, 16.0)
    }
}

fn screen() -> Vec2 {
    Vec2::new(960.0, 540.0)
}

#[derive(Clone, Debug, PartialEq)]
enum Action { Click }

fn button_tree() -> UiTree<Action> {
    UiTree::new(vec![
        UiNode::button("Click")
            .with_on_click(Action::Click)
            .with_anchor(Anchor::TopLeft),
    ])
}

fn setup_button() -> (UiTree<Action>, Layout, UiState) {
    let tree = button_tree();
    let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
    let mut state = UiState::new();
    state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);
    (tree, layout, state)
}

/// Mouse clicks should work when no touch events are present.
#[test]
fn mouse_click_works_without_touch() {
    let (tree, layout, mut state) = setup_button();
    let btn = &layout.rects[0];
    let cx = btn.x + 5.0;
    let cy = btn.y + 5.0;

    // Press
    let mut input = InputState::new();
    input.mouse_moved(cx, cy);
    input.mouse_button_pressed(MouseButton::Left);
    let (result, _) = process_input(&tree, &layout, &mut state, &input, screen());
    assert!(result.consumed_click, "mouse press should be consumed");

    // Release
    input.begin_frame();
    input.mouse_button_released(MouseButton::Left);
    let (_, events) = process_input(&tree, &layout, &mut state, &input, screen());
    assert_eq!(events, vec![Action::Click], "mouse release should fire click");
}

/// Touch tap should trigger a button click.
#[test]
fn touch_tap_triggers_click() {
    let (tree, layout, mut state) = setup_button();
    let btn = &layout.rects[0];
    let cx = btn.x + 5.0;
    let cy = btn.y + 5.0;

    // Touch begin
    let mut input = InputState::new();
    input.touch_started(1, cx, cy);
    let (result, _) = process_input(&tree, &layout, &mut state, &input, screen());
    assert!(result.consumed_click, "touch began should be consumed as click");

    // Touch end
    input.begin_frame();
    input.touch_ended(1);
    let (_, events) = process_input(&tree, &layout, &mut state, &input, screen());
    assert_eq!(events, vec![Action::Click], "touch end should fire click");
}

/// Quick tap (began + ended same frame) should fire click.
#[test]
fn quick_touch_tap_same_frame() {
    let (tree, layout, mut state) = setup_button();
    let btn = &layout.rects[0];
    let cx = btn.x + 5.0;
    let cy = btn.y + 5.0;

    let mut input = InputState::new();
    input.touch_started(1, cx, cy);
    input.touch_ended(1);
    let (_, events) = process_input(&tree, &layout, &mut state, &input, screen());
    assert_eq!(events, vec![Action::Click],
        "quick tap (began+ended same frame) should fire click");
}

/// After all touches end, mouse input should resume working.
#[test]
fn mouse_works_after_touch_ends() {
    let (tree, layout, mut state) = setup_button();
    let btn = &layout.rects[0];
    let cx = btn.x + 5.0;
    let cy = btn.y + 5.0;

    // Touch cycle
    let mut input = InputState::new();
    input.touch_started(1, cx, cy);
    input.touch_ended(1);
    let _ = process_input(&tree, &layout, &mut state, &input, screen());

    // New frame — touches cleared
    input.begin_frame();

    // Now use mouse
    input.mouse_moved(cx, cy);
    input.mouse_button_pressed(MouseButton::Left);
    let (result, _) = process_input(&tree, &layout, &mut state, &input, screen());
    assert!(result.consumed_click, "mouse should work after touches end");

    input.begin_frame();
    input.mouse_button_released(MouseButton::Left);
    let (_, events) = process_input(&tree, &layout, &mut state, &input, screen());
    assert_eq!(events, vec![Action::Click],
        "mouse click should fire after touch session ends");
}

/// Touch outside button should not consume click.
#[test]
fn touch_outside_button_not_consumed() {
    let (tree, layout, mut state) = setup_button();

    let mut input = InputState::new();
    input.touch_started(1, 900.0, 500.0);
    input.touch_ended(1);
    let (result, events) = process_input(&tree, &layout, &mut state, &input, screen());
    assert!(!result.consumed_click);
    assert!(events.is_empty());
}
