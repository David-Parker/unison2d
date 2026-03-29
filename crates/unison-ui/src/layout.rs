//! Layout engine — converts a UiNode tree into positioned, sized rectangles.
//!
//! Two-pass algorithm:
//! 1. **Measure (bottom-up):** Compute intrinsic sizes for each node.
//! 2. **Position (top-down):** Place nodes based on anchor and container direction.

use unison_core::Vec2;

use crate::node::{UiNode, UiTree, WidgetKind};
use crate::style::Anchor;

/// Default sizes for widgets that don't have explicit dimensions.
const DEFAULT_PROGRESS_BAR_WIDTH: f32 = 120.0;
const DEFAULT_PROGRESS_BAR_HEIGHT: f32 = 16.0;
const DEFAULT_ICON_SIZE: f32 = 24.0;
const DEFAULT_BUTTON_MIN_HEIGHT: f32 = 32.0;
const DEFAULT_BUTTON_PADDING_X: f32 = 12.0;
const DEFAULT_BUTTON_PADDING_Y: f32 = 6.0;

/// A laid-out widget with computed position and size in pixel coordinates.
///
/// Origin is **top-left** of the screen. Y increases downward.
#[derive(Clone, Debug)]
pub struct LayoutRect {
    /// Index of this node in the flattened pre-order list.
    pub node_index: usize,
    /// Position of the top-left corner (pixels, screen-space).
    pub x: f32,
    pub y: f32,
    /// Size in pixels.
    pub width: f32,
    pub height: f32,
}

impl LayoutRect {
    /// Check if a point (in pixel coords, origin top-left) is inside this rect.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width
            && py >= self.y && py <= self.y + self.height
    }
}

/// Trait for measuring text, so layout can be tested with mocks.
pub trait TextMeasurer {
    /// Measure the pixel size of a text string at the given font size.
    fn measure(&mut self, text: &str, font_size: f32) -> Vec2;
}

/// Computed layout for an entire UI tree.
#[derive(Clone, Debug)]
pub struct Layout {
    pub rects: Vec<LayoutRect>,
}

/// Compute layout for a UI tree.
pub fn compute_layout<E>(
    tree: &UiTree<E>,
    screen_size: Vec2,
    measurer: &mut dyn TextMeasurer,
) -> Layout {
    let mut rects = Vec::new();
    let mut index = 0;

    for root in &tree.roots {
        if !root.visible {
            skip_subtree(root, &mut index);
            continue;
        }

        // Measure intrinsic size
        let size = measure_node(root, measurer);

        // Position based on anchor
        let anchor = root.anchor.unwrap_or(Anchor::TopLeft);
        let pos = anchor_position(anchor, size, screen_size);

        // Lay out recursively
        layout_node(root, pos.x, pos.y, size.x, size.y, measurer, &mut rects, &mut index);
    }

    Layout { rects }
}

/// Measure the intrinsic size of a node (bottom-up).
fn measure_node<E>(node: &UiNode<E>, measurer: &mut dyn TextMeasurer) -> Vec2 {
    let content_size = match &node.kind {
        WidgetKind::Label { text } => {
            let font_size = node.text_style.as_ref().map_or(16.0, |s| s.font_size);
            measurer.measure(text, font_size)
        }
        WidgetKind::Button { text, .. } => {
            let font_size = node.text_style.as_ref().map_or(16.0, |s| s.font_size);
            let text_size = measurer.measure(text, font_size);
            Vec2::new(
                text_size.x + DEFAULT_BUTTON_PADDING_X * 2.0,
                (text_size.y + DEFAULT_BUTTON_PADDING_Y * 2.0).max(DEFAULT_BUTTON_MIN_HEIGHT),
            )
        }
        WidgetKind::Icon { .. } => {
            Vec2::new(DEFAULT_ICON_SIZE, DEFAULT_ICON_SIZE)
        }
        WidgetKind::ProgressBar { .. } => {
            Vec2::new(DEFAULT_PROGRESS_BAR_WIDTH, DEFAULT_PROGRESS_BAR_HEIGHT)
        }
        WidgetKind::Spacer { size } => {
            // Spacer takes up space in the parent's direction.
            // We don't know direction here, so report it as both dimensions.
            Vec2::new(*size, *size)
        }
        WidgetKind::Column | WidgetKind::Row | WidgetKind::Panel => {
            measure_container(node, measurer)
        }
    };

    // Apply explicit overrides
    let w = node.width.unwrap_or(content_size.x + node.padding.horizontal());
    let h = node.height.unwrap_or(content_size.y + node.padding.vertical());

    Vec2::new(w, h)
}

/// Measure a container node by measuring its children.
fn measure_container<E>(node: &UiNode<E>, measurer: &mut dyn TextMeasurer) -> Vec2 {
    let is_column = matches!(node.kind, WidgetKind::Column | WidgetKind::Panel);
    let gap = node.gap;

    let mut total_main: f32 = 0.0;
    let mut max_cross: f32 = 0.0;
    let mut visible_count: usize = 0;

    for child in &node.children {
        if !child.visible {
            continue;
        }
        let child_size = measure_node(child, measurer);
        if is_column {
            total_main += child_size.y;
            max_cross = max_cross.max(child_size.x);
        } else {
            total_main += child_size.x;
            max_cross = max_cross.max(child_size.y);
        }
        visible_count += 1;
    }

    // Add gaps between visible children
    if visible_count > 1 {
        total_main += gap * (visible_count - 1) as f32;
    }

    if is_column {
        Vec2::new(max_cross, total_main)
    } else {
        Vec2::new(total_main, max_cross)
    }
}

/// Compute the top-left position for an anchored root node.
fn anchor_position(anchor: Anchor, size: Vec2, screen: Vec2) -> Vec2 {
    let x = match anchor {
        Anchor::TopLeft | Anchor::CenterLeft | Anchor::BottomLeft => 0.0,
        Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => (screen.x - size.x) / 2.0,
        Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => screen.x - size.x,
    };
    let y = match anchor {
        Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => 0.0,
        Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => (screen.y - size.y) / 2.0,
        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => screen.y - size.y,
    };
    Vec2::new(x, y)
}

/// Recursively lay out a node and its children.
fn layout_node<E>(
    node: &UiNode<E>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    measurer: &mut dyn TextMeasurer,
    rects: &mut Vec<LayoutRect>,
    index: &mut usize,
) {
    // Record this node's layout
    let my_index = *index;
    rects.push(LayoutRect {
        node_index: my_index,
        x,
        y,
        width,
        height,
    });
    *index += 1;

    // Lay out children
    let content_x = x + node.padding.left;
    let content_y = y + node.padding.top;

    let is_column = matches!(node.kind, WidgetKind::Column | WidgetKind::Panel);
    let is_row = matches!(node.kind, WidgetKind::Row);

    if !is_column && !is_row {
        // Leaf nodes — still need to advance index for any children (shouldn't have any)
        for child in &node.children {
            skip_subtree(child, index);
        }
        return;
    }

    let gap = node.gap;
    let mut cursor = if is_column { content_y } else { content_x };

    for child in &node.children {
        if !child.visible {
            skip_subtree(child, index);
            continue;
        }

        let child_size = measure_node(child, measurer);

        let (cx, cy, cw, ch) = if is_column {
            let cx = content_x;
            let cy = cursor;
            let cw = child_size.x;
            let ch = child_size.y;
            cursor += ch + gap;
            (cx, cy, cw, ch)
        } else {
            let cx = cursor;
            let cy = content_y;
            let cw = child_size.x;
            let ch = child_size.y;
            cursor += cw + gap;
            (cx, cy, cw, ch)
        };

        layout_node(child, cx, cy, cw, ch, measurer, rects, index);
    }
}

/// Skip a subtree without laying it out, but still advance the index counter.
fn skip_subtree<E>(node: &UiNode<E>, index: &mut usize) {
    *index += 1;
    for child in &node.children {
        skip_subtree(child, index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{UiNode, UiTree};
    use crate::style::Anchor;

    /// Mock text measurer: 8px wide per char, 16px tall.
    struct FixedMeasurer;
    impl TextMeasurer for FixedMeasurer {
        fn measure(&mut self, text: &str, _font_size: f32) -> Vec2 {
            Vec2::new(text.len() as f32 * 8.0, 16.0)
        }
    }

    type Node = UiNode<()>;

    fn screen() -> Vec2 {
        Vec2::new(960.0, 540.0)
    }

    #[test]
    fn single_label_top_left() {
        let tree = UiTree::new(vec![
            Node::label("Hello").with_anchor(Anchor::TopLeft),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        assert_eq!(layout.rects.len(), 1);
        let r = &layout.rects[0];
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.width, 40.0);  // 5 chars * 8
        assert_eq!(r.height, 16.0);
    }

    #[test]
    fn column_stacking() {
        let tree = UiTree::new(vec![
            Node::column()
                .with_anchor(Anchor::TopLeft)
                .with_gap(4.0)
                .with_children(vec![
                    Node::label("A"),
                    Node::label("BB"),
                    Node::label("CCC"),
                ]),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        // Column + 3 children = 4 rects
        assert_eq!(layout.rects.len(), 4);
        // Children stacked vertically
        let c0 = &layout.rects[1]; // "A"
        let c1 = &layout.rects[2]; // "BB"
        let c2 = &layout.rects[3]; // "CCC"
        assert_eq!(c0.y, 0.0);
        assert_eq!(c1.y, 20.0);  // 16 + 4 gap
        assert_eq!(c2.y, 40.0);  // 16 + 4 + 16 + 4
    }

    #[test]
    fn row_stacking() {
        let tree = UiTree::new(vec![
            Node::row()
                .with_anchor(Anchor::TopLeft)
                .with_gap(4.0)
                .with_children(vec![
                    Node::label("A"),   // 8px
                    Node::label("BB"),  // 16px
                    Node::label("C"),   // 8px
                ]),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let c0 = &layout.rects[1];
        let c1 = &layout.rects[2];
        let c2 = &layout.rects[3];
        assert_eq!(c0.x, 0.0);
        assert_eq!(c1.x, 12.0);  // 8 + 4 gap
        assert_eq!(c2.x, 32.0);  // 8 + 4 + 16 + 4
    }

    #[test]
    fn padding() {
        let tree = UiTree::new(vec![
            Node::column()
                .with_anchor(Anchor::TopLeft)
                .with_padding(10.0)
                .with_children(vec![
                    Node::label("Hi"),
                ]),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let child = &layout.rects[1];
        assert_eq!(child.x, 10.0);
        assert_eq!(child.y, 10.0);
    }

    #[test]
    fn nested_containers() {
        let tree = UiTree::new(vec![
            Node::column()
                .with_anchor(Anchor::TopLeft)
                .with_children(vec![
                    Node::row().with_children(vec![
                        Node::label("A"),
                        Node::label("B"),
                    ]),
                    Node::label("C"),
                ]),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        // Column(0), Row(1), Label A(2), Label B(3), Label C(4)
        assert_eq!(layout.rects.len(), 5);
        let label_a = &layout.rects[2];
        let label_b = &layout.rects[3];
        assert_eq!(label_a.x, 0.0);
        assert_eq!(label_b.x, 8.0);  // After "A" (8px wide)
    }

    #[test]
    fn all_anchors() {
        let s = screen();
        let label_w = 40.0; // "Hello" = 5*8
        let label_h = 16.0;

        for (anchor, expected_x, expected_y) in [
            (Anchor::TopLeft, 0.0, 0.0),
            (Anchor::TopCenter, (s.x - label_w) / 2.0, 0.0),
            (Anchor::TopRight, s.x - label_w, 0.0),
            (Anchor::CenterLeft, 0.0, (s.y - label_h) / 2.0),
            (Anchor::Center, (s.x - label_w) / 2.0, (s.y - label_h) / 2.0),
            (Anchor::CenterRight, s.x - label_w, (s.y - label_h) / 2.0),
            (Anchor::BottomLeft, 0.0, s.y - label_h),
            (Anchor::BottomCenter, (s.x - label_w) / 2.0, s.y - label_h),
            (Anchor::BottomRight, s.x - label_w, s.y - label_h),
        ] {
            let tree = UiTree::new(vec![
                Node::label("Hello").with_anchor(anchor),
            ]);
            let layout = compute_layout(&tree, s, &mut FixedMeasurer);
            let r = &layout.rects[0];
            assert_eq!(r.x, expected_x, "anchor={:?} x", anchor);
            assert_eq!(r.y, expected_y, "anchor={:?} y", anchor);
        }
    }

    #[test]
    fn explicit_width_height() {
        let tree = UiTree::new(vec![
            Node::label("Hi")
                .with_anchor(Anchor::TopLeft)
                .with_width(200.0)
                .with_height(50.0),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let r = &layout.rects[0];
        assert_eq!(r.width, 200.0);
        assert_eq!(r.height, 50.0);
    }

    #[test]
    fn spacer_in_column() {
        let tree = UiTree::new(vec![
            Node::column()
                .with_anchor(Anchor::TopLeft)
                .with_children(vec![
                    Node::label("A"),
                    Node::spacer(20.0),
                    Node::label("B"),
                ]),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let label_b = &layout.rects[3];
        // A is 16px, spacer is 20px, so B starts at 36
        assert_eq!(label_b.y, 36.0);
    }

    #[test]
    fn layout_stability() {
        let tree = UiTree::new(vec![
            Node::column()
                .with_anchor(Anchor::Center)
                .with_gap(4.0)
                .with_padding(8.0)
                .with_children(vec![
                    Node::label("One"),
                    Node::label("Two"),
                    Node::label("Three"),
                ]),
        ]);
        let l1 = compute_layout(&tree, screen(), &mut FixedMeasurer);
        let l2 = compute_layout(&tree, screen(), &mut FixedMeasurer);
        for (a, b) in l1.rects.iter().zip(l2.rects.iter()) {
            assert_eq!(a.x, b.x);
            assert_eq!(a.y, b.y);
            assert_eq!(a.width, b.width);
            assert_eq!(a.height, b.height);
        }
    }

    #[test]
    fn invisible_node_skipped() {
        let tree = UiTree::new(vec![
            Node::column()
                .with_anchor(Anchor::TopLeft)
                .with_children(vec![
                    Node::label("A"),
                    Node::label("B").with_visible(false),
                    Node::label("C"),
                ]),
        ]);
        let layout = compute_layout(&tree, screen(), &mut FixedMeasurer);
        // Column + A + C = 3 visible rects (B is skipped)
        // But index still advances for B
        assert_eq!(layout.rects.len(), 3);
        let label_c = &layout.rects[2];
        // A=16px, B invisible so no gap, C starts at 16
        assert_eq!(label_c.y, 16.0);
    }
}
