//! Integration tests for UI lifecycle and multi-frame interactions.
//!
//! These tests exercise multiple unison-ui subsystems together:
//! input processing, tree diffing, state management, layout, and animation timers.
//! They were extracted from unison-ui's per-module unit tests because they test
//! cross-cutting behaviour rather than a single module.

use unison_input::{InputState, MouseButton};
use unison_core::Vec2;
use unison_ui::diff::{diff_trees, DiffOp, NodeKey};
use unison_ui::input::process_input;
use unison_ui::layout::{compute_layout, Layout, TextMeasurer};
use unison_ui::node::{UiNode, UiTree};
use unison_ui::state::UiState;
use unison_ui::style::Anchor;

fn screen() -> Vec2 {
    Vec2::new(960.0, 540.0)
}

struct FixedMeasurer;
impl TextMeasurer for FixedMeasurer {
    fn measure(&mut self, text: &str, _font_size: f32) -> Vec2 {
        Vec2::new(text.len() as f32 * 8.0, 16.0)
    }
}

// ── Helpers ──

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Click,
}

fn setup_button_test() -> (UiTree<Action>, Layout, UiState) {
    let tree = UiTree::new(vec![
        UiNode::button("Click")
            .with_on_click(Action::Click)
            .with_anchor(Anchor::TopLeft),
    ]);
    let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
    let mut state = UiState::new();
    state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);
    (tree, layout, state)
}

// ── Multi-frame click (from input.rs) ──

/// Two-frame press→release sequence across InputState + process_input + UiState.
#[test]
fn click_triggers_event() {
    let (tree, layout, mut state) = setup_button_test();
    let btn = &layout.rects[0];

    let mut input = InputState::new();
    input.mouse_moved(btn.x + 5.0, btn.y + 5.0);

    // Frame 1: press inside
    input.mouse_button_pressed(MouseButton::Left);
    let (result, events) = process_input(&tree, &layout, &mut state, &input, screen());
    assert!(result.consumed_click);
    assert!(events.is_empty()); // No event on press, only on release

    // Frame 2: release inside (begin_frame clears just_pressed, then release)
    input.begin_frame();
    input.mouse_button_released(MouseButton::Left);
    let (_, events) = process_input(&tree, &layout, &mut state, &input, screen());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], Action::Click);
}

// ── Full frame cycle (from facade.rs) ──

/// Wires input processing → diff → state application for the first frame bootstrap.
#[test]
fn full_frame_cycle_no_panic() {
    let tree: UiTree<()> = UiTree::new(vec![
        UiNode::panel()
            .with_anchor(Anchor::TopLeft)
            .with_width(200.0)
            .with_height(100.0)
            .with_children(vec![
                UiNode::progress_bar(0.75),
            ]),
    ]);

    let mut state = UiState::new();
    let input = InputState::new();

    // Step 1: Process input against empty previous layout
    let prev_tree: UiTree<()> = UiTree::empty();
    let prev_layout = Layout { rects: Vec::new() };
    let (result, _events) = process_input(&prev_tree, &prev_layout, &mut state, &input, screen());
    assert!(!result.consumed_click);

    // Step 2: Diff + layout
    let ops = diff_trees(&prev_tree, &tree);
    state.apply_diff(&ops);

    // Verify state was created
    assert!(state.get(&NodeKey::root(0)).is_some());
}

// ── Conditional add/remove with exit animation (from facade.rs) ──

/// Multi-frame test: add widget → remove widget → exit animation completes → state purged.
#[test]
fn conditional_ui_adds_and_removes() {
    let mut state = UiState::new();

    // Frame 1: Panel visible
    let tree1: UiTree<()> = UiTree::new(vec![
        UiNode::panel()
            .with_anchor(Anchor::Center)
            .with_width(200.0)
            .with_height(100.0),
    ]);
    let ops1 = diff_trees(&UiTree::empty(), &tree1);
    state.apply_diff(&ops1);
    assert_eq!(state.len(), 1);

    // Frame 2: Panel removed
    let tree2: UiTree<()> = UiTree::empty();
    let ops2 = diff_trees(&tree1, &tree2);
    state.apply_diff(&ops2);
    // Widget should be exiting, not yet removed
    let ws = state.get(&NodeKey::root(0)).unwrap();
    assert!(ws.is_exiting());

    // After enough time, exit completes
    state.update(0.2);
    assert!(state.is_empty());
}

// ── 10-frame lifecycle (from facade.rs) ──

/// Complex multi-frame simulation: show menu → hide menu → exit animation → show different menu.
#[test]
fn ten_frame_lifecycle() {
    let mut state = UiState::new();
    let dt = 0.016;

    let menu_tree: UiTree<()> = UiTree::new(vec![
        UiNode::panel()
            .with_anchor(Anchor::Center)
            .with_width(200.0)
            .with_height(100.0)
            .with_children(vec![
                UiNode::progress_bar(0.5),
            ]),
    ]);
    let empty_tree: UiTree<()> = UiTree::empty();
    let alt_menu: UiTree<()> = UiTree::new(vec![
        UiNode::panel()
            .with_anchor(Anchor::TopLeft)
            .with_width(150.0)
            .with_height(80.0),
    ]);

    let mut prev = UiTree::empty();

    // Frames 1-3: show menu
    for _ in 0..3 {
        let ops = diff_trees(&prev, &menu_tree);
        state.apply_diff(&ops);
        state.update(dt);
        prev = menu_tree.clone();
    }
    assert_eq!(state.len(), 2); // panel + progress_bar

    // Frames 4-6: hide menu
    for _ in 0..3 {
        let ops = diff_trees(&prev, &empty_tree);
        state.apply_diff(&ops);
        state.update(dt);
        prev = empty_tree.clone();
    }

    // After ~0.048s of hiding, exit animations should still be in progress
    // (EXIT_DURATION = 0.12). But after frame 6 we've done 3 * 0.016 = 0.048s of exit.
    // Need more time for full removal.
    state.update(0.1); // Push past exit duration
    assert!(state.is_empty(), "all widgets should be purged after exit");

    // Frames 7-10: show different menu
    for _ in 0..4 {
        let ops = diff_trees(&prev, &alt_menu);
        state.apply_diff(&ops);
        state.update(dt);
        prev = alt_menu.clone();
    }
    assert_eq!(state.len(), 1); // just the new panel
    assert!(state.get(&NodeKey::root(0)).is_some());
}
