//! `Ui<E>` facade — the public API that wires all UI subsystems together.
//!
//! Game code creates a `Ui<E>` once and calls it each frame:
//!
//! ```ignore
//! // In update():
//! let ui_input = self.ui.begin_frame(ctx.input, screen_size, ctx.dt);
//! self.ui.describe(ui! { ... }, &mut ctx.renderer);
//! for event in self.ui.drain_events() { ... }
//! if !ui_input.consumed_click { /* game input */ }
//!
//! // In render():
//! self.ui.render(&mut self.world, &mut ctx.renderer);
//! ```

use unison_input::InputState;
use unison_math::Vec2;
use unison_render::Renderer;

use crate::diff::diff_trees;
use crate::input::{process_input, UiInputResult};
use crate::layout::{compute_layout, Layout, TextMeasurer};
use crate::node::UiTree;
use crate::render::render_ui;
use crate::state::UiState;
use crate::text::TextRenderer;

/// The main UI facade. Generic over the game's event/action type `E`.
pub struct Ui<E: Clone> {
    /// Previous frame's tree (for diffing).
    prev_tree: UiTree<E>,
    /// Current frame's tree (set by `describe`).
    curr_tree: UiTree<E>,
    /// Current layout (computed by `describe`).
    layout: Layout,
    /// Persistent widget state (hover, press, animation timers).
    state: UiState,
    /// Text renderer (font + glyph atlas).
    text_renderer: TextRenderer,
    /// Events triggered this frame (click actions etc.).
    events: Vec<E>,
    /// Screen size (cached from last begin_frame).
    screen_size: Vec2,
    /// Whether `describe` has been called this frame.
    described: bool,
}

impl<E: Clone> Ui<E> {
    /// Create a new UI system with the given font.
    ///
    /// `font_bytes` should be raw TTF/OTF data. The device scale factor is
    /// derived from the renderer's `drawable_size / screen_size`.
    pub fn new(
        font_bytes: Vec<u8>,
        renderer: &mut dyn Renderer<Error = String>,
    ) -> Result<Self, String> {
        let scale_factor = compute_scale_factor(renderer);
        let text_renderer = TextRenderer::new(font_bytes, scale_factor, renderer)?;
        Ok(Self {
            prev_tree: UiTree::empty(),
            curr_tree: UiTree::empty(),
            layout: Layout { rects: Vec::new() },
            state: UiState::new(),
            text_renderer,
            events: Vec::new(),
            screen_size: Vec2::new(960.0, 540.0),
            described: false,
        })
    }

    /// Start a new frame. Processes input against the *previous* frame's layout,
    /// advances animation timers, and returns input consumption info.
    ///
    /// Call this at the beginning of your update function.
    pub fn begin_frame(
        &mut self,
        input: &InputState,
        screen_size: Vec2,
        dt: f32,
    ) -> UiInputResult {
        self.screen_size = screen_size;
        self.described = false;
        self.events.clear();

        // Advance animation timers
        self.state.update(dt);

        // Process input against previous frame's layout
        let (result, events) = process_input(
            &self.prev_tree,
            &self.layout,
            &mut self.state,
            input,
            screen_size,
        );

        self.events = events;
        result
    }

    /// Describe the UI tree for this frame.
    ///
    /// Diffs against the previous frame's tree, updates widget state,
    /// and computes layout. Call this after `begin_frame`.
    pub fn describe(
        &mut self,
        tree: UiTree<E>,
        renderer: &mut dyn Renderer<Error = String>,
    ) {
        // Update scale factor in case it changed (e.g., window moved between displays)
        let scale_factor = compute_scale_factor(renderer);
        let _ = self.text_renderer.set_scale_factor(scale_factor, renderer);

        // Diff against previous tree
        let ops = diff_trees(&self.prev_tree, &tree);
        self.state.apply_diff(&ops);

        // Compute layout using the text renderer as measurer
        let mut measurer = TextRendererMeasurer {
            text_renderer: &mut self.text_renderer,
            renderer,
        };
        self.layout = compute_layout(&tree, self.screen_size, &mut measurer);

        // Store current tree for next frame's diff
        self.prev_tree = self.curr_tree.clone();
        self.curr_tree = tree;
        self.described = true;
    }

    /// Drain triggered events (button clicks, etc.).
    ///
    /// Returns all events accumulated since `begin_frame`. Calling this
    /// clears the event buffer — a second call returns an empty vec.
    pub fn drain_events(&mut self) -> Vec<E> {
        std::mem::take(&mut self.events)
    }

    /// Render the UI into the world's overlay system.
    ///
    /// Call this in your render function, before `world.auto_render()`.
    pub fn render(
        &mut self,
        world: &mut dyn OverlayTarget,
        renderer: &mut dyn Renderer<Error = String>,
    ) {
        let tree = if self.described { &self.curr_tree } else { &self.prev_tree };

        let commands = render_ui(
            tree,
            &self.layout,
            &self.state,
            self.screen_size,
            &mut self.text_renderer,
            renderer,
        );

        for cmd in commands {
            world.draw_overlay(cmd.command, cmd.z_order);
        }
    }

    /// Access the text renderer (e.g., for custom text measurement).
    pub fn text_renderer(&mut self) -> &mut TextRenderer {
        &mut self.text_renderer
    }

    /// Access the current layout (for debugging or custom hit testing).
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// Access widget state (for debugging or custom rendering).
    pub fn state(&self) -> &UiState {
        &self.state
    }
}

/// Trait for targets that can receive overlay commands.
///
/// `World` implements this, but tests can use a mock.
pub trait OverlayTarget {
    fn draw_overlay(&mut self, command: unison_render::RenderCommand, z_order: i32);
}

/// Adapter that lets TextRenderer implement TextMeasurer.
struct TextRendererMeasurer<'a> {
    text_renderer: &'a mut TextRenderer,
    renderer: &'a mut dyn Renderer<Error = String>,
}

impl<'a> TextMeasurer for TextRendererMeasurer<'a> {
    fn measure(&mut self, text: &str, font_size: f32) -> Vec2 {
        self.text_renderer.measure(text, font_size, self.renderer)
    }
}

/// Derive the device scale factor from the renderer's drawable vs logical size.
fn compute_scale_factor(renderer: &dyn Renderer<Error = String>) -> f32 {
    let (sw, _) = renderer.screen_size();
    if sw > 0.0 {
        let (dw, _) = renderer.drawable_size();
        (dw / sw).max(1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffOp, NodeKey};
    use crate::node::UiNode;
    use crate::style::Anchor;
    use unison_math::Color;
    use unison_render::RenderCommand;

    // ── Mock Overlay Target ──

    struct MockOverlay {
        commands: Vec<(RenderCommand, i32)>,
    }

    impl MockOverlay {
        fn new() -> Self {
            Self { commands: Vec::new() }
        }
    }

    impl OverlayTarget for MockOverlay {
        fn draw_overlay(&mut self, command: RenderCommand, z_order: i32) {
            self.commands.push((command, z_order));
        }
    }

    // full_frame_cycle_no_panic, conditional_ui_adds_and_removes, and
    // ten_frame_lifecycle moved to unison-tests/tests/ui_lifecycle.rs

    #[test]
    fn diff_across_frames() {
        let tree1: UiTree<()> = UiTree::new(vec![
            UiNode::panel()
                .with_anchor(Anchor::TopLeft)
                .with_width(100.0)
                .with_height(50.0),
        ]);

        let tree2: UiTree<()> = UiTree::new(vec![
            UiNode::panel()
                .with_anchor(Anchor::TopLeft)
                .with_width(200.0)
                .with_height(100.0),
        ]);

        let mut state = UiState::new();

        // Frame 1
        let ops1 = diff_trees(&UiTree::empty(), &tree1);
        state.apply_diff(&ops1);
        assert_eq!(state.len(), 1);

        // Frame 2 — same widget type, different size → Updated
        let ops2 = diff_trees(&tree1, &tree2);
        assert!(ops2.iter().all(|op| matches!(op, DiffOp::Updated(_) | DiffOp::Unchanged(_))));
        state.apply_diff(&ops2);
        assert_eq!(state.len(), 1); // Same widget, just updated
    }

    // conditional_ui_adds_and_removes moved to unison-tests/tests/ui_lifecycle.rs

    #[test]
    fn no_op_frame_all_unchanged() {
        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::panel()
                .with_anchor(Anchor::TopLeft)
                .with_width(100.0)
                .with_height(50.0),
        ]);

        let ops = diff_trees(&tree, &tree);
        assert!(ops.iter().all(|op| matches!(op, DiffOp::Unchanged(_))));
    }

    #[test]
    fn multiple_roots_create_separate_state() {
        let mut state = UiState::new();

        let tree: UiTree<()> = UiTree::new(vec![
            UiNode::panel()
                .with_anchor(Anchor::TopLeft)
                .with_width(100.0)
                .with_height(50.0),
            UiNode::panel()
                .with_anchor(Anchor::TopRight)
                .with_width(100.0)
                .with_height(50.0),
        ]);

        let ops = diff_trees(&UiTree::empty(), &tree);
        state.apply_diff(&ops);
        assert_eq!(state.len(), 2);
        assert!(state.get(&NodeKey::root(0)).is_some());
        assert!(state.get(&NodeKey::root(1)).is_some());
    }

    // ten_frame_lifecycle moved to unison-tests/tests/ui_lifecycle.rs

    #[test]
    fn overlay_target_trait_works() {
        let mut overlay = MockOverlay::new();

        // Directly submit some commands
        overlay.draw_overlay(RenderCommand::Rect {
            position: [0.0, 0.9],
            size: [0.1, 0.1],
            color: Color::WHITE,
        }, 0);

        overlay.draw_overlay(RenderCommand::Rect {
            position: [0.1, 0.8],
            size: [0.2, 0.1],
            color: Color::RED,
        }, 1);

        assert_eq!(overlay.commands.len(), 2);
        assert_eq!(overlay.commands[0].1, 0);
        assert_eq!(overlay.commands[1].1, 1);
    }
}
