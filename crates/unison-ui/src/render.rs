//! Render pipeline — converts a laid-out UI tree into overlay RenderCommands.
//!
//! Walks the UI tree alongside the layout rects, emitting `RenderCommand`s
//! for each visible widget. Commands use the overlay coordinate system:
//! (0,0) = bottom-left, (1,1) = top-right.
//!
//! The caller submits these to `World::draw_overlay(cmd, z_order)`.

use unison_core::{Color, Vec2};
use unison_render::{DrawSprite, RenderCommand, Renderer, TextureId};

use crate::animation::compute_animation;
use crate::diff::NodeKey;
use crate::layout::{Layout, LayoutRect};
use crate::node::{UiNode, UiTree, WidgetKind};
use crate::state::{UiState, WidgetState};
use crate::text::TextRenderer;

/// Default button colors.
const BUTTON_NORMAL: Color = Color::new(0.25, 0.25, 0.3, 0.9);
const BUTTON_HOVER: Color = Color::new(0.35, 0.35, 0.42, 0.95);
const BUTTON_PRESS: Color = Color::new(0.15, 0.15, 0.2, 0.95);

/// Default progress bar colors.
const PROGRESS_BG: Color = Color::new(0.2, 0.2, 0.2, 0.8);
const PROGRESS_FG: Color = Color::new(0.3, 0.7, 0.3, 1.0);

/// An overlay render command with a z-order for submission to `World::draw_overlay`.
#[derive(Debug, Clone)]
pub struct OverlayCommand {
    pub command: RenderCommand,
    pub z_order: i32,
}

/// Render a laid-out UI tree into overlay commands.
///
/// `screen_size` is the viewport size in CSS pixels, used for pixel→overlay conversion.
/// `text_renderer` is needed for label/button text glyph generation.
///
/// Returns a list of `OverlayCommand`s sorted by z-order (lowest first).
pub fn render_ui<E>(
    tree: &UiTree<E>,
    layout: &Layout,
    ui_state: &UiState,
    screen_size: Vec2,
    text_renderer: &mut TextRenderer,
    renderer: &mut dyn Renderer<Error = String>,
) -> Vec<OverlayCommand> {
    let mut commands = Vec::new();
    let mut rect_idx = 0;
    let mut z = 0i32;

    for (root_i, root) in tree.roots.iter().enumerate() {
        let key = NodeKey::root(root_i as u16);
        render_node(
            root,
            &key,
            layout,
            ui_state,
            screen_size,
            text_renderer,
            renderer,
            &mut rect_idx,
            &mut z,
            &mut commands,
        );
    }

    commands
}

/// Recursively render a node and its children.
fn render_node<E>(
    node: &UiNode<E>,
    key: &NodeKey,
    layout: &Layout,
    ui_state: &UiState,
    screen_size: Vec2,
    text_renderer: &mut TextRenderer,
    renderer: &mut dyn Renderer<Error = String>,
    rect_idx: &mut usize,
    z: &mut i32,
    commands: &mut Vec<OverlayCommand>,
) {
    if !node.visible {
        skip_count(node, rect_idx);
        return;
    }

    let my_idx = *rect_idx;
    *rect_idx += 1;

    let rect = match layout.rects.get(my_idx) {
        Some(r) => r,
        None => return,
    };

    let widget_state = ui_state.get(key);
    let anim = compute_animation(widget_state);

    // Emit commands for this widget (with animation alpha applied)
    match &node.kind {
        WidgetKind::Panel => {
            render_panel(node, rect, screen_size, anim.alpha, z, commands);
        }
        WidgetKind::Label { text } => {
            render_label(text, node, rect, screen_size, anim.alpha, text_renderer, renderer, z, commands);
        }
        WidgetKind::Button { text, .. } => {
            render_button(text, node, rect, widget_state, screen_size, anim.alpha, text_renderer, renderer, z, commands);
        }
        WidgetKind::ProgressBar { value } => {
            render_progress_bar(*value, node, rect, screen_size, anim.alpha, z, commands);
        }
        WidgetKind::Icon { texture } => {
            render_icon(*texture, rect, screen_size, anim.alpha, z, commands);
        }
        // Column, Row, Spacer — no visual output (children rendered below)
        WidgetKind::Column | WidgetKind::Row | WidgetKind::Spacer { .. } => {}
    }

    // Recurse into children
    for (i, child) in node.children.iter().enumerate() {
        render_node(
            child,
            &key.child(i as u16),
            layout,
            ui_state,
            screen_size,
            text_renderer,
            renderer,
            rect_idx,
            z,
            commands,
        );
    }
}

/// Render a Panel: background rect + optional border, or 9-slice texture.
fn render_panel<E>(
    node: &UiNode<E>,
    rect: &LayoutRect,
    screen_size: Vec2,
    alpha: f32,
    z: &mut i32,
    commands: &mut Vec<OverlayCommand>,
) {
    // If a 9-slice texture is set, use it instead of solid color
    if let Some(ref nine) = node.nine_slice {
        let tint = node.panel_style.as_ref()
            .map(|s| s.background)
            .unwrap_or(Color::WHITE);
        let tint = with_alpha(tint, alpha);
        let sprites = crate::nine_slice::render_nine_slice(
            nine, rect.x, rect.y, rect.width, rect.height, tint,
        );
        for sprite in sprites {
            let overlay = pixel_sprite_to_overlay(&sprite, screen_size);
            commands.push(OverlayCommand {
                command: RenderCommand::Sprite(overlay),
                z_order: *z,
            });
        }
        *z += 1;
        return;
    }

    let style = node.panel_style.as_ref().cloned().unwrap_or_default();

    // Border (slightly larger rect behind the background)
    if style.border_width > 0.0 && style.border_color.a > 0.0 {
        let bw = style.border_width;
        let (pos, size) = pixel_rect_to_overlay(
            rect.x - bw, rect.y - bw,
            rect.width + 2.0 * bw, rect.height + 2.0 * bw,
            screen_size,
        );
        commands.push(OverlayCommand {
            command: RenderCommand::Rect { position: pos, size, color: with_alpha(style.border_color, alpha) },
            z_order: *z,
        });
        *z += 1;
    }

    // Background
    let (pos, size) = pixel_rect_to_overlay(rect.x, rect.y, rect.width, rect.height, screen_size);
    commands.push(OverlayCommand {
        command: RenderCommand::Rect { position: pos, size, color: with_alpha(style.background, alpha) },
        z_order: *z,
    });
    *z += 1;
}

/// Render a Label: series of glyph sprites.
fn render_label<E>(
    text: &str,
    node: &UiNode<E>,
    rect: &LayoutRect,
    screen_size: Vec2,
    alpha: f32,
    text_renderer: &mut TextRenderer,
    renderer: &mut dyn Renderer<Error = String>,
    z: &mut i32,
    commands: &mut Vec<OverlayCommand>,
) {
    let style = node.text_style.as_ref().cloned().unwrap_or_default();
    let color = with_alpha(style.color, alpha);
    let glyph_cmds = text_renderer.render_text(text, Vec2::new(rect.x, rect.y), style.font_size, color, renderer);

    for cmd in glyph_cmds {
        if let RenderCommand::Sprite(sprite) = cmd {
            let overlay_sprite = pixel_sprite_to_overlay(&sprite, screen_size);
            commands.push(OverlayCommand {
                command: RenderCommand::Sprite(overlay_sprite),
                z_order: *z,
            });
        }
    }
    *z += 1;
}

/// Render a Button: background rect + text glyphs, with hover/press color variation.
fn render_button<E>(
    text: &str,
    node: &UiNode<E>,
    rect: &LayoutRect,
    widget_state: Option<&WidgetState>,
    screen_size: Vec2,
    alpha: f32,
    text_renderer: &mut TextRenderer,
    renderer: &mut dyn Renderer<Error = String>,
    z: &mut i32,
    commands: &mut Vec<OverlayCommand>,
) {
    // Determine button background color based on state
    let bg_color = match widget_state {
        Some(ws) if ws.pressed => BUTTON_PRESS,
        Some(ws) if ws.hovered => {
            lerp_color(BUTTON_NORMAL, BUTTON_HOVER, ws.hover_time)
        }
        _ => BUTTON_NORMAL,
    };

    // Button background (with animation alpha)
    let (pos, size) = pixel_rect_to_overlay(rect.x, rect.y, rect.width, rect.height, screen_size);
    commands.push(OverlayCommand {
        command: RenderCommand::Rect { position: pos, size, color: with_alpha(bg_color, alpha) },
        z_order: *z,
    });
    *z += 1;

    // Button text (centered within the button rect)
    let style = node.text_style.as_ref().cloned().unwrap_or_default();
    let text_size = text_renderer.measure(text, style.font_size, renderer);

    // Center the text within the button
    let text_x = rect.x + (rect.width - text_size.x) / 2.0;
    let text_y = rect.y + (rect.height - text_size.y) / 2.0;

    let color = with_alpha(style.color, alpha);
    let glyph_cmds = text_renderer.render_text(text, Vec2::new(text_x, text_y), style.font_size, color, renderer);

    for cmd in glyph_cmds {
        if let RenderCommand::Sprite(sprite) = cmd {
            let overlay_sprite = pixel_sprite_to_overlay(&sprite, screen_size);
            commands.push(OverlayCommand {
                command: RenderCommand::Sprite(overlay_sprite),
                z_order: *z,
            });
        }
    }
    *z += 1;
}

/// Render a ProgressBar: background rect + foreground fill rect.
fn render_progress_bar<E>(
    value: f32,
    _node: &UiNode<E>,
    rect: &LayoutRect,
    screen_size: Vec2,
    alpha: f32,
    z: &mut i32,
    commands: &mut Vec<OverlayCommand>,
) {
    // Background
    let (pos, size) = pixel_rect_to_overlay(rect.x, rect.y, rect.width, rect.height, screen_size);
    commands.push(OverlayCommand {
        command: RenderCommand::Rect { position: pos, size, color: with_alpha(PROGRESS_BG, alpha) },
        z_order: *z,
    });
    *z += 1;

    // Foreground fill
    if value > 0.0 {
        let fill_width = rect.width * value;
        let (fpos, fsize) = pixel_rect_to_overlay(rect.x, rect.y, fill_width, rect.height, screen_size);
        commands.push(OverlayCommand {
            command: RenderCommand::Rect { position: fpos, size: fsize, color: with_alpha(PROGRESS_FG, alpha) },
            z_order: *z,
        });
    }
    *z += 1;
}

/// Render an Icon: single sprite with the given texture.
fn render_icon(
    texture: TextureId,
    rect: &LayoutRect,
    screen_size: Vec2,
    alpha: f32,
    z: &mut i32,
    commands: &mut Vec<OverlayCommand>,
) {
    let sprite = DrawSprite {
        texture,
        position: [rect.x + rect.width / 2.0, rect.y + rect.height / 2.0],
        size: [rect.width, rect.height],
        rotation: 0.0,
        uv: [0.0, 0.0, 1.0, 1.0],
        color: with_alpha(Color::WHITE, alpha),
    };
    let overlay_sprite = pixel_sprite_to_overlay(&sprite, screen_size);
    commands.push(OverlayCommand {
        command: RenderCommand::Sprite(overlay_sprite),
        z_order: *z,
    });
    *z += 1;
}

// ── Coordinate conversion helpers ──

/// Convert a pixel-space rect (top-left origin, Y-down) to overlay coordinates
/// (bottom-left origin, Y-up). Returns (position, size) where position is bottom-left.
///
/// For `RenderCommand::Rect`, position is the bottom-left corner.
fn pixel_rect_to_overlay(
    px: f32, py: f32,
    pw: f32, ph: f32,
    screen_size: Vec2,
) -> ([f32; 2], [f32; 2]) {
    let ox = px / screen_size.x;
    // In pixel space, py is the top edge. The bottom edge is py + ph.
    // In overlay space (Y-up), the bottom-left y = 1.0 - (py + ph) / screen_h
    let oy = 1.0 - (py + ph) / screen_size.y;
    let ow = pw / screen_size.x;
    let oh = ph / screen_size.y;
    ([ox, oy], [ow, oh])
}

/// Convert a pixel-space sprite (center position, Y-down) to overlay coordinates.
fn pixel_sprite_to_overlay(sprite: &DrawSprite, screen_size: Vec2) -> DrawSprite {
    let cx = sprite.position[0] / screen_size.x;
    // Sprite position is center. In pixel space, center_y is distance from top.
    // In overlay space, center_y = 1.0 - center_pixel_y / screen_h
    let cy = 1.0 - sprite.position[1] / screen_size.y;
    let sw = sprite.size[0] / screen_size.x;
    let sh = sprite.size[1] / screen_size.y;

    DrawSprite {
        texture: sprite.texture,
        position: [cx, cy],
        size: [sw, sh],
        rotation: sprite.rotation,
        uv: sprite.uv,
        color: sprite.color,
    }
}

/// Linearly interpolate between two colors.
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Apply an alpha multiplier to a color.
fn with_alpha(c: Color, alpha: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * alpha)
}

/// Skip index counting for a hidden subtree.
fn skip_count<E>(node: &UiNode<E>, idx: &mut usize) {
    *idx += 1;
    for child in &node.children {
        skip_count(child, idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffOp, NodeKey};
    use crate::layout::{compute_layout, TextMeasurer};
    use crate::node::{UiNode, UiTree};
    use crate::state::UiState;
    use crate::style::{Anchor, PanelStyle};
    use unison_core::Vec2;

    struct FixedMeasurer;
    impl TextMeasurer for FixedMeasurer {
        fn measure(&mut self, text: &str, _font_size: f32) -> Vec2 {
            Vec2::new(text.len() as f32 * 8.0, 16.0)
        }
    }

    fn screen() -> Vec2 {
        Vec2::new(960.0, 540.0)
    }

    // ── Coordinate conversion tests ──

    #[test]
    fn pixel_rect_top_left() {
        // Pixel rect at (0, 0, 96, 54) → overlay bottom-left at (0.0, 0.9), size (0.1, 0.1)
        let (pos, size) = pixel_rect_to_overlay(0.0, 0.0, 96.0, 54.0, screen());
        assert!((pos[0] - 0.0).abs() < 0.001);
        assert!((pos[1] - 0.9).abs() < 0.001); // 1.0 - 54/540 = 0.9
        assert!((size[0] - 0.1).abs() < 0.001); // 96/960
        assert!((size[1] - 0.1).abs() < 0.001); // 54/540
    }

    #[test]
    fn pixel_rect_bottom_right() {
        // Pixel rect at (864, 486, 96, 54) → overlay at (0.9, 0.0)
        let (pos, size) = pixel_rect_to_overlay(864.0, 486.0, 96.0, 54.0, screen());
        assert!((pos[0] - 0.9).abs() < 0.001);
        assert!((pos[1] - 0.0).abs() < 0.001); // 1.0 - (486+54)/540 = 0.0
        assert!((size[0] - 0.1).abs() < 0.001);
        assert!((size[1] - 0.1).abs() < 0.001);
    }

    #[test]
    fn pixel_rect_center() {
        // Pixel rect at (430, 245, 100, 50) → center area
        let (pos, _size) = pixel_rect_to_overlay(430.0, 245.0, 100.0, 50.0, screen());
        let ox = 430.0 / 960.0;
        let oy = 1.0 - (245.0 + 50.0) / 540.0;
        assert!((pos[0] - ox).abs() < 0.001);
        assert!((pos[1] - oy).abs() < 0.001);
    }

    #[test]
    fn sprite_conversion() {
        // Sprite centered at pixel (480, 270) on 960x540
        let sprite = DrawSprite {
            texture: TextureId::NONE,
            position: [480.0, 270.0],
            size: [96.0, 54.0],
            rotation: 0.0,
            uv: [0.0, 0.0, 1.0, 1.0],
            color: Color::WHITE,
        };
        let overlay = pixel_sprite_to_overlay(&sprite, screen());
        assert!((overlay.position[0] - 0.5).abs() < 0.001); // 480/960
        assert!((overlay.position[1] - 0.5).abs() < 0.001); // 1 - 270/540
        assert!((overlay.size[0] - 0.1).abs() < 0.001);
        assert!((overlay.size[1] - 0.1).abs() < 0.001);
    }

    // ── Widget rendering tests (no TextRenderer needed for non-text widgets) ──

    #[test]
    fn panel_background_emits_rect() {
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::panel()
                .with_anchor(Anchor::TopLeft)
                .with_width(100.0)
                .with_height(50.0),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        // Panel should emit at least 1 Rect (background)
        let rects: Vec<_> = cmds.iter().filter(|c| matches!(c.command, RenderCommand::Rect { .. })).collect();
        assert_eq!(rects.len(), 1, "panel should emit 1 background rect");
    }

    #[test]
    fn panel_with_border_emits_two_rects() {
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::panel()
                .with_anchor(Anchor::TopLeft)
                .with_width(100.0)
                .with_height(50.0)
                .with_panel_style(PanelStyle::new().border(Color::WHITE, 2.0)),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        let rects: Vec<_> = cmds.iter().filter(|c| matches!(c.command, RenderCommand::Rect { .. })).collect();
        assert_eq!(rects.len(), 2, "panel with border should emit border + background");
        // Border should be drawn first (lower z)
        assert!(rects[0].z_order < rects[1].z_order);
    }

    #[test]
    fn progress_bar_emits_two_rects() {
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::progress_bar(0.5).with_anchor(Anchor::TopLeft),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        let rects: Vec<_> = cmds.iter().filter(|c| matches!(c.command, RenderCommand::Rect { .. })).collect();
        assert_eq!(rects.len(), 2, "progress bar should emit bg + fg rects");

        // Foreground should be narrower than background (50% fill)
        if let (RenderCommand::Rect { size: bg_size, .. }, RenderCommand::Rect { size: fg_size, .. }) =
            (&rects[0].command, &rects[1].command)
        {
            assert!((fg_size[0] - bg_size[0] * 0.5).abs() < 0.001);
        }
    }

    #[test]
    fn progress_bar_zero_emits_one_rect() {
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::progress_bar(0.0).with_anchor(Anchor::TopLeft),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        let rects: Vec<_> = cmds.iter().filter(|c| matches!(c.command, RenderCommand::Rect { .. })).collect();
        assert_eq!(rects.len(), 1, "0% progress bar should only emit background");
    }

    #[test]
    fn icon_emits_sprite() {
        let tex = TextureId(42);
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::icon(tex).with_anchor(Anchor::TopLeft),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        let sprites: Vec<_> = cmds.iter().filter(|c| matches!(c.command, RenderCommand::Sprite(_))).collect();
        assert_eq!(sprites.len(), 1, "icon should emit 1 sprite");

        if let RenderCommand::Sprite(ref s) = sprites[0].command {
            assert_eq!(s.texture, tex);
        }
    }

    #[test]
    fn invisible_widget_no_commands() {
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::panel()
                .with_anchor(Anchor::TopLeft)
                .with_width(100.0)
                .with_height(50.0)
                .with_visible(false),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        assert!(cmds.is_empty(), "invisible widget should produce no commands");
    }

    #[test]
    fn z_order_increases() {
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::panel()
                .with_anchor(Anchor::TopLeft)
                .with_width(200.0)
                .with_height(100.0)
                .with_children(vec![
                    UiNode::progress_bar(0.5),
                ]),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0)), DiffOp::Added(NodeKey::root(0).child(0))]);

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        // Verify z-order is monotonically non-decreasing
        for pair in cmds.windows(2) {
            assert!(pair[0].z_order <= pair[1].z_order, "z-order should increase");
        }
    }

    #[test]
    fn empty_tree_no_commands() {
        let tree: UiTree<()> = UiTree::empty();
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let state = UiState::new();

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        assert!(cmds.is_empty());
    }

    #[test]
    fn column_container_no_visual() {
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::column().with_anchor(Anchor::TopLeft),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

        let cmds = render_ui_no_text(&tree, &layout, &state, screen());
        assert!(cmds.is_empty(), "empty column should produce no visual commands");
    }

    #[test]
    fn overlay_positions_in_valid_range() {
        // Panel at each corner should produce overlay coords in [0, 1]
        for anchor in &[Anchor::TopLeft, Anchor::TopRight, Anchor::BottomLeft, Anchor::BottomRight, Anchor::Center] {
            let tree: UiTree<()> = UiTree::new(vec![
                UiNode::panel()
                    .with_anchor(*anchor)
                    .with_width(100.0)
                    .with_height(50.0),
            ]);
            let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
            let mut state = UiState::new();
            state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

            let cmds = render_ui_no_text(&tree, &layout, &state, screen());
            for cmd in &cmds {
                match &cmd.command {
                    RenderCommand::Rect { position, size, .. } => {
                        assert!(position[0] >= -0.001, "rect x out of range: {}", position[0]);
                        assert!(position[1] >= -0.001, "rect y out of range: {}", position[1]);
                        assert!(position[0] + size[0] <= 1.001, "rect right edge out of range");
                        assert!(position[1] + size[1] <= 1.001, "rect top edge out of range");
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn lerp_color_test() {
        let a = Color::new(0.0, 0.0, 0.0, 1.0);
        let b = Color::new(1.0, 1.0, 1.0, 1.0);
        let mid = lerp_color(a, b, 0.5);
        assert!((mid.r - 0.5).abs() < 0.001);
        assert!((mid.g - 0.5).abs() < 0.001);
        assert!((mid.b - 0.5).abs() < 0.001);
    }

    // Helper: render without text renderer (for non-text widget tests)
    fn render_ui_no_text(
        tree: &UiTree<()>,
        layout: &Layout,
        ui_state: &UiState,
        screen_size: Vec2,
    ) -> Vec<OverlayCommand> {
        let mut commands = Vec::new();
        let mut rect_idx = 0;
        let mut z = 0i32;

        for (root_i, root) in tree.roots.iter().enumerate() {
            let key = NodeKey::root(root_i as u16);
            render_node_no_text(root, &key, layout, ui_state, screen_size, &mut rect_idx, &mut z, &mut commands);
        }

        commands
    }

    /// Render without text support — skips label/button text, only emits rects/sprites.
    fn render_node_no_text(
        node: &UiNode<()>,
        key: &NodeKey,
        layout: &Layout,
        ui_state: &UiState,
        screen_size: Vec2,
        rect_idx: &mut usize,
        z: &mut i32,
        commands: &mut Vec<OverlayCommand>,
    ) {
        if !node.visible {
            skip_count(node, rect_idx);
            return;
        }

        let my_idx = *rect_idx;
        *rect_idx += 1;

        let rect = match layout.rects.get(my_idx) {
            Some(r) => r,
            None => return,
        };

        let widget_state = ui_state.get(key);
        let anim = compute_animation(widget_state);

        match &node.kind {
            WidgetKind::Panel => {
                render_panel(node, rect, screen_size, anim.alpha, z, commands);
            }
            WidgetKind::Button { .. } => {
                // Button background only (no text without TextRenderer)
                let bg_color = match widget_state {
                    Some(ws) if ws.pressed => BUTTON_PRESS,
                    Some(ws) if ws.hovered => lerp_color(BUTTON_NORMAL, BUTTON_HOVER, ws.hover_time),
                    _ => BUTTON_NORMAL,
                };
                let (pos, size) = pixel_rect_to_overlay(rect.x, rect.y, rect.width, rect.height, screen_size);
                commands.push(OverlayCommand {
                    command: RenderCommand::Rect { position: pos, size, color: with_alpha(bg_color, anim.alpha) },
                    z_order: *z,
                });
                *z += 1;
            }
            WidgetKind::ProgressBar { value } => {
                render_progress_bar(*value, node, rect, screen_size, anim.alpha, z, commands);
            }
            WidgetKind::Icon { texture } => {
                render_icon(*texture, rect, screen_size, anim.alpha, z, commands);
            }
            // Label, Column, Row, Spacer — no visual in this test mode
            _ => {}
        }

        for (i, child) in node.children.iter().enumerate() {
            render_node_no_text(
                child,
                &key.child(i as u16),
                layout,
                ui_state,
                screen_size,
                rect_idx,
                z,
                commands,
            );
        }
    }
}
