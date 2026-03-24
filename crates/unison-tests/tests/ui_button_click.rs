//! Integration tests for UI button click handling.
//!
//! Reproduces the exact frame lifecycle used by the game:
//!   update(): ui.begin_frame → drain_events
//!   render(): ui.describe(tree) → drain_events
//!
//! Also simulates the game_loop's fixed-timestep + input-swap mechanism
//! to catch timing-dependent click bugs.

use unison_input::{InputBuffer, InputState, MouseButton};
use unison_math::Vec2;
use unison_ui::diff::diff_trees;
use unison_ui::input::process_input;
use unison_ui::layout::{compute_layout, Layout, TextMeasurer};
use unison_ui::node::{UiNode, UiTree};
use unison_ui::state::UiState;
use unison_ui::style::Anchor;

const DT: f32 = 1.0 / 60.0;

fn screen() -> Vec2 {
    Vec2::new(960.0, 540.0)
}

// ── Helpers ──

struct FixedMeasurer;
impl TextMeasurer for FixedMeasurer {
    fn measure(&mut self, text: &str, _font_size: f32) -> Vec2 {
        Vec2::new(text.len() as f32 * 8.0, 16.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum MenuAction {
    SelectLevel(usize),
}

/// Build a menu tree identical to the game's menu.
fn menu_tree() -> UiTree<MenuAction> {
    UiTree::new(vec![
        UiNode::panel()
            .with_anchor(Anchor::TopLeft)
            .with_width(300.0)
            .with_height(300.0)
            .with_children(vec![
                UiNode::column().with_children(vec![
                    UiNode::label("Menu"),
                    UiNode::button("Level 1")
                        .with_on_click(MenuAction::SelectLevel(0))
                        .with_width(280.0)
                        .with_height(48.0),
                    UiNode::button("Level 2")
                        .with_on_click(MenuAction::SelectLevel(1))
                        .with_width(280.0)
                        .with_height(48.0),
                ]),
            ]),
    ])
}

/// Simulates one frame the way menu_level does it:
///   1. begin_frame (update phase) — processes input against prev layout
///   2. describe(tree) (render phase) — diffs, computes layout
/// Returns events collected from the update phase.
fn simulate_frame(
    prev_tree: &mut UiTree<MenuAction>,
    layout: &mut Layout,
    state: &mut UiState,
    input: &InputState,
    tree: UiTree<MenuAction>,
) -> Vec<MenuAction> {
    // ── update phase ──
    state.update(DT);
    // clear_input_state resets hovered (but not pressed)
    state.clear_input_state();
    let (_result, events) = process_input(
        prev_tree,
        layout,
        state,
        input,
        screen(),
    );

    // ── render phase ──
    let ops = diff_trees(prev_tree, &tree);
    state.apply_diff(&ops);
    *layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
    *prev_tree = tree;

    events
}

/// Get the center of the first button in the layout.
/// Tree structure: panel(0) > column(1) > [label(2), button1(3), button2(4)]
fn first_button_center(layout: &Layout) -> (f32, f32) {
    let btn = &layout.rects[3];
    (btn.x + btn.width / 2.0, btn.y + btn.height / 2.0)
}

// ── Tests ──

#[test]
fn click_after_one_idle_frame() {
    let mut prev_tree = UiTree::empty();
    let mut layout = Layout { rects: Vec::new() };
    let mut state = UiState::new();
    let mut input = InputState::new();

    // Frame 1: UI appears
    let events = simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
    assert!(events.is_empty());

    let (bx, by) = first_button_center(&layout);
    input.mouse_moved(bx, by);

    // Frame 2: press
    input.mouse_button_pressed(MouseButton::Left);
    let events = simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
    assert!(events.is_empty(), "press alone should not fire");

    // Frame 3: release
    input.begin_frame();
    input.mouse_button_released(MouseButton::Left);
    let events = simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
    assert_eq!(events, vec![MenuAction::SelectLevel(0)]);
}

#[test]
fn click_after_stable_ui() {
    let mut prev_tree = UiTree::empty();
    let mut layout = Layout { rects: Vec::new() };
    let mut state = UiState::new();
    let mut input = InputState::new();

    // Run 5 idle frames
    for _ in 0..5 {
        simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
        input.begin_frame();
    }

    let (bx, by) = first_button_center(&layout);
    input.mouse_moved(bx, by);

    // Press
    input.mouse_button_pressed(MouseButton::Left);
    let events = simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
    assert!(events.is_empty());

    // Release
    input.begin_frame();
    input.mouse_button_released(MouseButton::Left);
    let events = simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
    assert_eq!(events, vec![MenuAction::SelectLevel(0)]);
}

#[test]
fn fast_click_press_and_release_same_frame() {
    let mut prev_tree = UiTree::empty();
    let mut layout = Layout { rects: Vec::new() };
    let mut state = UiState::new();
    let mut input = InputState::new();

    // Stabilize
    for _ in 0..3 {
        simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
        input.begin_frame();
    }

    let (bx, by) = first_button_center(&layout);
    input.mouse_moved(bx, by);

    // Both press and release in the same frame
    input.mouse_button_pressed(MouseButton::Left);
    input.mouse_button_released(MouseButton::Left);
    let events = simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
    assert_eq!(events, vec![MenuAction::SelectLevel(0)]);
}

#[test]
fn three_consecutive_clicks_all_fire() {
    let mut prev_tree = UiTree::empty();
    let mut layout = Layout { rects: Vec::new() };
    let mut state = UiState::new();
    let mut input = InputState::new();

    // Stabilize
    for _ in 0..3 {
        simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
        input.begin_frame();
    }

    let (bx, by) = first_button_center(&layout);
    input.mouse_moved(bx, by);

    let mut all_events = Vec::new();
    for click in 1..=3 {
        // Press
        input.mouse_button_pressed(MouseButton::Left);
        let events = simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
        all_events.extend(events);

        // Release
        input.begin_frame();
        input.mouse_button_released(MouseButton::Left);
        let events = simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
        all_events.extend(events);

        assert_eq!(
            all_events.len(), click,
            "after click {click}, expected {click} events but got {}",
            all_events.len()
        );

        // Idle frame between clicks
        input.begin_frame();
        simulate_frame(&mut prev_tree, &mut layout, &mut state, &input, menu_tree());
        input.begin_frame();
    }
}

/// Simulates the game_loop's fixed-timestep behavior using InputBuffer.
///
/// This catches the bug where a click's press or release event is swapped into
/// the engine on a RAF frame where no update runs, causing the event to be
/// silently discarded on the next swap. InputBuffer::transfer() prevents this
/// by only swapping when an update tick will actually run.
#[test]
fn click_survives_skipped_update_frame() {
    let mut prev_tree = UiTree::empty();
    let mut layout = Layout { rects: Vec::new() };
    let mut state = UiState::new();
    let mut input = InputBuffer::new();
    let mut engine_input = InputState::new();
    let mut accumulator: f32 = 0.0;

    /// Simulate a RAF callback with the given dt.
    fn raf_frame(
        dt: f32,
        accumulator: &mut f32,
        input: &mut InputBuffer,
        engine_input: &mut InputState,
        prev_tree: &mut UiTree<MenuAction>,
        layout: &mut Layout,
        state: &mut UiState,
    ) -> Vec<MenuAction> {
        *accumulator += dt.min(0.1);

        let will_update = *accumulator >= DT;
        input.transfer(will_update);
        if will_update {
            input.swap_into(engine_input);
        }

        let mut all_events = Vec::new();
        let mut first_tick = true;
        while *accumulator >= DT {
            if !first_tick {
                engine_input.begin_frame();
            }
            first_tick = false;
            state.update(DT);
            state.clear_input_state();
            let (_result, events) = process_input(
                prev_tree, layout, state, engine_input, screen(),
            );
            all_events.extend(events);
            *accumulator -= DT;
        }

        let tree = menu_tree();
        let ops = diff_trees(prev_tree, &tree);
        state.apply_diff(&ops);
        *layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        *prev_tree = tree;
        all_events
    }

    // Frame 1: normal frame to set up UI
    let events = raf_frame(DT, &mut accumulator, &mut input, &mut engine_input,
        &mut prev_tree, &mut layout, &mut state);
    assert!(events.is_empty());

    let (bx, by) = first_button_center(&layout);

    // Mouse move (DOM event goes into shared buffer)
    input.shared_mut().mouse_moved(bx, by);

    // Frame 2: normal frame, mouse is hovering
    let events = raf_frame(DT, &mut accumulator, &mut input, &mut engine_input,
        &mut prev_tree, &mut layout, &mut state);
    assert!(events.is_empty());

    // DOM mousedown event (between RAF frames)
    input.shared_mut().mouse_button_pressed(MouseButton::Left);

    // Frame 3: SHORT dt — accumulator < FIXED_DT so transfer is a no-op.
    // The press event stays in shared, safe from being discarded.
    let events = raf_frame(0.005, &mut accumulator, &mut input, &mut engine_input,
        &mut prev_tree, &mut layout, &mut state);
    assert!(events.is_empty(), "no transfer means no update, no events");

    // DOM mouseup event (between RAF frames, accumulates in shared)
    input.shared_mut().mouse_button_released(MouseButton::Left);

    // Frame 4: normal dt — transfer finally happens. Engine gets both press
    // and release. process_input sees just_pressed → pressed=true, then
    // just_released + pressed → fires the click event.
    let events = raf_frame(DT, &mut accumulator, &mut input, &mut engine_input,
        &mut prev_tree, &mut layout, &mut state);
    assert_eq!(
        events, vec![MenuAction::SelectLevel(0)],
        "click must fire even when press happened during a deferred-swap RAF frame"
    );
}

/// Same scenario but the release (not press) happens during a skipped-update frame.
#[test]
fn click_survives_release_on_skipped_update_frame() {
    let mut prev_tree = UiTree::empty();
    let mut layout = Layout { rects: Vec::new() };
    let mut state = UiState::new();
    let mut input = InputBuffer::new();
    let mut engine_input = InputState::new();
    let mut accumulator: f32 = 0.0;

    fn raf_frame(
        dt: f32,
        accumulator: &mut f32,
        input: &mut InputBuffer,
        engine_input: &mut InputState,
        prev_tree: &mut UiTree<MenuAction>,
        layout: &mut Layout,
        state: &mut UiState,
    ) -> Vec<MenuAction> {
        *accumulator += dt.min(0.1);

        let will_update = *accumulator >= DT;
        input.transfer(will_update);
        if will_update {
            input.swap_into(engine_input);
        }

        let mut all_events = Vec::new();
        let mut first_tick = true;
        while *accumulator >= DT {
            if !first_tick { engine_input.begin_frame(); }
            first_tick = false;
            state.update(DT);
            state.clear_input_state();
            let (_result, events) = process_input(
                prev_tree, layout, state, engine_input, screen(),
            );
            all_events.extend(events);
            *accumulator -= DT;
        }

        let tree = menu_tree();
        let ops = diff_trees(prev_tree, &tree);
        state.apply_diff(&ops);
        *layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        *prev_tree = tree;
        all_events
    }

    // Stabilize
    for _ in 0..3 {
        raf_frame(DT, &mut accumulator, &mut input, &mut engine_input,
            &mut prev_tree, &mut layout, &mut state);
    }

    let (bx, by) = first_button_center(&layout);
    input.shared_mut().mouse_moved(bx, by);

    // Normal frame to pick up mouse position
    raf_frame(DT, &mut accumulator, &mut input, &mut engine_input,
        &mut prev_tree, &mut layout, &mut state);

    // Press (normal frame, update runs)
    input.shared_mut().mouse_button_pressed(MouseButton::Left);
    let events = raf_frame(DT, &mut accumulator, &mut input, &mut engine_input,
        &mut prev_tree, &mut layout, &mut state);
    assert!(events.is_empty(), "press alone should not fire");

    // Release happens, but next RAF has short dt — transfer is a no-op.
    input.shared_mut().mouse_button_released(MouseButton::Left);
    let events = raf_frame(0.005, &mut accumulator, &mut input, &mut engine_input,
        &mut prev_tree, &mut layout, &mut state);
    assert!(events.is_empty(), "no transfer on short-dt frame");

    // Next normal frame — transfer happens, engine gets the release,
    // pressed=true from earlier frame → fires click.
    let events = raf_frame(DT, &mut accumulator, &mut input, &mut engine_input,
        &mut prev_tree, &mut layout, &mut state);
    assert_eq!(
        events, vec![MenuAction::SelectLevel(0)],
        "click must fire even when release happened during a deferred-swap RAF frame"
    );
}
