//! Input handling — hit testing, hover detection, click events, and input consumption.

use unison_input::{InputState, MouseButton};
use unison_math::Vec2;

use crate::diff::NodeKey;
use crate::layout::Layout;
use crate::node::{UiNode, UiTree, WidgetKind};
use crate::state::UiState;

/// Result of UI input processing for one frame.
#[derive(Clone, Debug, Default)]
pub struct UiInputResult {
    /// Whether the UI consumed a mouse click this frame.
    pub consumed_click: bool,
    /// Whether the mouse is hovering over any interactive UI element.
    pub consumed_hover: bool,
}

/// Process input against the laid-out UI.
///
/// Updates widget hover/press state and returns triggered events + consumption info.
///
/// `mouse_pos` should be in CSS pixel coordinates (origin top-left), matching
/// the layout coordinate system.
pub fn process_input<E: Clone>(
    tree: &UiTree<E>,
    layout: &Layout,
    ui_state: &mut UiState,
    input: &InputState,
    _screen_size: Vec2,
) -> (UiInputResult, Vec<E>) {
    let mouse = input.mouse_position();
    let mouse_just_pressed = input.is_mouse_just_pressed(MouseButton::Left);
    let mouse_just_released = input.is_mouse_just_released(MouseButton::Left);

    let mut result = UiInputResult::default();
    let mut events: Vec<E> = Vec::new();

    // Reset hover state for all widgets
    ui_state.clear_input_state();

    // Walk nodes and layout rects together to find interactive widgets.
    // We collect all hit interactive widgets, then pick the topmost (last in list = highest z).
    let mut hits: Vec<(NodeKey, usize)> = Vec::new(); // (key, rect_index)

    let mut rect_idx = 0;
    for (root_i, root) in tree.roots.iter().enumerate() {
        let key = NodeKey::root(root_i as u16);
        collect_interactive_hits(
            root,
            &key,
            layout,
            &mut rect_idx,
            mouse,
            &mut hits,
        );
    }

    // The last hit in the list has the highest z-order (drawn on top)
    if let Some((ref hit_key, _hit_rect_idx)) = hits.last() {
        result.consumed_hover = true;

        // Set hover on the hit widget
        if let Some(state) = ui_state.get_mut(hit_key) {
            state.hovered = true;
        }

        // Handle click
        if mouse_just_pressed {
            if let Some(state) = ui_state.get_mut(hit_key) {
                state.pressed = true;
            }
            result.consumed_click = true;
        }

        if mouse_just_released {
            if let Some(state) = ui_state.get_mut(hit_key) {
                if state.pressed {
                    // Trigger the click event
                    if let Some(event) = get_on_click_event(tree, hit_key) {
                        events.push(event);
                    }
                    state.pressed = false;
                }
            }
        }
    }

    // Clear pressed state for widgets where mouse was released outside
    if mouse_just_released {
        // We need to clear pressed for all widgets except the one that just fired
        clear_stale_presses(tree, layout, ui_state, mouse);
    }

    (result, events)
}

/// Walk the tree and collect interactive widgets that the mouse is over.
fn collect_interactive_hits<E>(
    node: &UiNode<E>,
    key: &NodeKey,
    layout: &Layout,
    rect_idx: &mut usize,
    mouse: Vec2,
    hits: &mut Vec<(NodeKey, usize)>,
) {
    if !node.visible {
        skip_count(node, rect_idx);
        return;
    }

    let my_idx = *rect_idx;
    *rect_idx += 1;

    // Check if this node is interactive and the mouse is inside
    if is_interactive(&node.kind) {
        if let Some(rect) = layout.rects.get(my_idx) {
            if rect.contains(mouse.x, mouse.y) {
                hits.push((key.clone(), my_idx));
            }
        }
    }

    // Recurse into children
    for (i, child) in node.children.iter().enumerate() {
        collect_interactive_hits(
            child,
            &key.child(i as u16),
            layout,
            rect_idx,
            mouse,
            hits,
        );
    }
}

/// Whether a widget kind is interactive (can be clicked/hovered).
fn is_interactive<E>(kind: &WidgetKind<E>) -> bool {
    matches!(kind, WidgetKind::Button { .. })
}

/// Get the on_click event from a button node at the given key path.
fn get_on_click_event<E: Clone>(tree: &UiTree<E>, key: &NodeKey) -> Option<E> {
    let node = resolve_node(tree, key)?;
    match &node.kind {
        WidgetKind::Button { on_click, .. } => on_click.clone(),
        _ => None,
    }
}

/// Resolve a NodeKey to the actual UiNode in the tree.
fn resolve_node<'a, E>(tree: &'a UiTree<E>, key: &NodeKey) -> Option<&'a UiNode<E>> {
    if key.path.is_empty() {
        return None;
    }
    let mut node = tree.roots.get(key.path[0] as usize)?;
    for &idx in &key.path[1..] {
        node = node.children.get(idx as usize)?;
    }
    Some(node)
}

/// Clear pressed state for widgets where mouse was released outside their bounds.
fn clear_stale_presses<E>(
    _tree: &UiTree<E>,
    _layout: &Layout,
    ui_state: &mut UiState,
    _mouse: Vec2,
) {
    // Simply clear all pressed states — the hit widget already fired its event above.
    // This is safe because we only track one pressed widget at a time.
    // A more sophisticated approach would track which widget is "captured" during press.
    // For now, clearing all pressed on release works correctly.
    for key in ui_state.all_keys() {
        if let Some(state) = ui_state.get_mut(&key) {
            state.pressed = false;
        }
    }
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
    use crate::style::Anchor;
    use unison_input::InputState;
    use unison_math::Vec2;

    #[derive(Clone, Debug, PartialEq)]
    enum Action {
        Click,
        Other,
    }

    struct FixedMeasurer;
    impl TextMeasurer for FixedMeasurer {
        fn measure(&mut self, text: &str, _font_size: f32) -> Vec2 {
            Vec2::new(text.len() as f32 * 8.0, 16.0)
        }
    }

    fn screen() -> Vec2 {
        Vec2::new(960.0, 540.0)
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

    #[test]
    fn hover_detection() {
        let (tree, layout, mut state) = setup_button_test();
        let btn = &layout.rects[0];

        // Mouse inside button
        let mut input = InputState::new();
        input.mouse_moved(btn.x + 5.0, btn.y + 5.0);
        let (result, _) = process_input(&tree, &layout, &mut state, &input, screen());
        assert!(result.consumed_hover);
        assert!(state.get(&NodeKey::root(0)).unwrap().hovered);
    }

    #[test]
    fn hover_miss() {
        let (tree, layout, mut state) = setup_button_test();

        // Mouse outside button
        let mut input = InputState::new();
        input.mouse_moved(900.0, 500.0);
        let (result, _) = process_input(&tree, &layout, &mut state, &input, screen());
        assert!(!result.consumed_hover);
        assert!(!state.get(&NodeKey::root(0)).unwrap().hovered);
    }

    #[test]
    fn click_triggers_event() {
        let (tree, layout, mut state) = setup_button_test();
        let btn = &layout.rects[0];

        // Use a single InputState across frames (like the real engine does)
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

    #[test]
    fn non_interactive_passthrough() {
        let tree: UiTree<Action> = UiTree::new(vec![
            UiNode::label("Just text").with_anchor(Anchor::TopLeft),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let mut state = UiState::new();
        state.apply_diff(&[DiffOp::Added(NodeKey::root(0))]);

        let mut input = InputState::new();
        input.mouse_moved(5.0, 5.0);
        let (result, _) = process_input(&tree, &layout, &mut state, &input, screen());
        assert!(!result.consumed_hover); // Labels are not interactive
    }

    #[test]
    fn click_empty_space_not_consumed() {
        let (tree, layout, mut state) = setup_button_test();

        let mut input = InputState::new();
        input.mouse_moved(900.0, 500.0);
        input.mouse_button_pressed(MouseButton::Left);
        let (result, _) = process_input(&tree, &layout, &mut state, &input, screen());
        assert!(!result.consumed_click);
    }
}
