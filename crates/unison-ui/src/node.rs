//! UiNode — the declarative UI tree type.
//!
//! Game code builds a `UiNode<E>` tree each frame (via the `ui!` macro or
//! builder methods). The UI system diffs it against last frame's tree.

use unison_render::TextureId;

use crate::style::{Anchor, EdgeInsets, NineSlice, PanelStyle, TextStyle};

/// What kind of widget this node represents.
#[derive(Clone, Debug)]
pub enum WidgetKind<E> {
    /// Vertical layout container.
    Column,
    /// Horizontal layout container.
    Row,
    /// Visual container with background.
    Panel,
    /// Text label.
    Label { text: String },
    /// Clickable button with text.
    Button { text: String, on_click: Option<E> },
    /// Texture icon.
    Icon { texture: TextureId },
    /// Progress bar.
    ProgressBar { value: f32 },
    /// Fixed-size spacer.
    Spacer { size: f32 },
}

/// A node in the declarative UI tree.
#[derive(Clone, Debug)]
pub struct UiNode<E> {
    /// Widget type and type-specific data.
    pub kind: WidgetKind<E>,
    /// Child nodes.
    pub children: Vec<UiNode<E>>,

    // Layout props
    /// Screen anchor (only meaningful on root-level nodes).
    pub anchor: Option<Anchor>,
    /// Internal padding.
    pub padding: EdgeInsets,
    /// Spacing between children (Column/Row).
    pub gap: f32,
    /// Explicit width override (pixels).
    pub width: Option<f32>,
    /// Explicit height override (pixels).
    pub height: Option<f32>,

    // Style props
    /// Text style (for Label/Button text).
    pub text_style: Option<TextStyle>,
    /// Panel visual style.
    pub panel_style: Option<PanelStyle>,
    /// 9-slice texture background.
    pub nine_slice: Option<NineSlice>,

    // Identity
    /// Explicit key for diffing (overrides positional identity).
    pub key: Option<u64>,
    /// Whether this node is visible (invisible nodes are skipped entirely).
    pub visible: bool,
}

impl<E> UiNode<E> {
    /// Create a new node with the given widget kind.
    fn new(kind: WidgetKind<E>) -> Self {
        Self {
            kind,
            children: Vec::new(),
            anchor: None,
            padding: EdgeInsets::default(),
            gap: 0.0,
            width: None,
            height: None,
            text_style: None,
            panel_style: None,
            nine_slice: None,
            key: None,
            visible: true,
        }
    }

    // ── Constructors ──

    pub fn column() -> Self {
        Self::new(WidgetKind::Column)
    }

    pub fn row() -> Self {
        Self::new(WidgetKind::Row)
    }

    pub fn panel() -> Self {
        let mut node = Self::new(WidgetKind::Panel);
        node.panel_style = Some(PanelStyle::default());
        node
    }

    pub fn label(text: impl Into<String>) -> Self {
        Self::new(WidgetKind::Label { text: text.into() })
    }

    pub fn button(text: impl Into<String>) -> Self {
        Self::new(WidgetKind::Button { text: text.into(), on_click: None })
    }

    pub fn icon(texture: TextureId) -> Self {
        Self::new(WidgetKind::Icon { texture })
    }

    pub fn progress_bar(value: f32) -> Self {
        Self::new(WidgetKind::ProgressBar { value: value.clamp(0.0, 1.0) })
    }

    pub fn spacer(size: f32) -> Self {
        Self::new(WidgetKind::Spacer { size })
    }

    // ── Builder methods ──

    pub fn with_children(mut self, children: Vec<UiNode<E>>) -> Self {
        self.children = children;
        self
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    pub fn with_padding(mut self, padding: impl Into<EdgeInsets>) -> Self {
        self.padding = padding.into();
        self
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn with_text_style(mut self, style: TextStyle) -> Self {
        self.text_style = Some(style);
        self
    }

    pub fn with_panel_style(mut self, style: PanelStyle) -> Self {
        self.panel_style = Some(style);
        self
    }

    pub fn with_nine_slice(mut self, nine_slice: NineSlice) -> Self {
        self.nine_slice = Some(nine_slice);
        self
    }

    pub fn with_key(mut self, key: u64) -> Self {
        self.key = Some(key);
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn with_on_click(mut self, event: E) -> Self {
        if let WidgetKind::Button { ref mut on_click, .. } = self.kind {
            *on_click = Some(event);
        }
        self
    }
}

impl<E> WidgetKind<E> {
    /// Returns a string tag for this widget kind (for diffing).
    pub fn tag(&self) -> &'static str {
        match self {
            WidgetKind::Column => "column",
            WidgetKind::Row => "row",
            WidgetKind::Panel => "panel",
            WidgetKind::Label { .. } => "label",
            WidgetKind::Button { .. } => "button",
            WidgetKind::Icon { .. } => "icon",
            WidgetKind::ProgressBar { .. } => "progress_bar",
            WidgetKind::Spacer { .. } => "spacer",
        }
    }
}

/// A root-level UI description: one or more anchored node trees.
#[derive(Clone, Debug)]
pub struct UiTree<E> {
    pub roots: Vec<UiNode<E>>,
}

impl<E> UiTree<E> {
    pub fn new(roots: Vec<UiNode<E>>) -> Self {
        Self { roots }
    }

    pub fn empty() -> Self {
        Self { roots: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Anchor;
    use crate::ui;

    #[derive(Clone, Debug, PartialEq)]
    enum TestAction {
        Resume,
        Quit,
    }

    #[test]
    fn builder_label() {
        let node: UiNode<TestAction> = UiNode::label("Hello");
        assert!(matches!(node.kind, WidgetKind::Label { ref text } if text == "Hello"));
        assert!(node.children.is_empty());
        assert!(node.visible);
    }

    #[test]
    fn builder_button_with_event() {
        let node = UiNode::button("Click me").with_on_click(TestAction::Resume);
        match &node.kind {
            WidgetKind::Button { text, on_click } => {
                assert_eq!(text, "Click me");
                assert_eq!(on_click.as_ref(), Some(&TestAction::Resume));
            }
            _ => panic!("expected Button"),
        }
    }

    #[test]
    fn builder_column_with_children() {
        let node: UiNode<TestAction> = UiNode::column()
            .with_anchor(Anchor::TopLeft)
            .with_padding(8.0)
            .with_gap(4.0)
            .with_children(vec![
                UiNode::label("A"),
                UiNode::label("B"),
            ]);
        assert!(matches!(node.kind, WidgetKind::Column));
        assert_eq!(node.anchor, Some(Anchor::TopLeft));
        assert_eq!(node.padding, crate::style::EdgeInsets::all(8.0));
        assert_eq!(node.gap, 4.0);
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn builder_progress_bar_clamps() {
        let node: UiNode<TestAction> = UiNode::progress_bar(1.5);
        match &node.kind {
            WidgetKind::ProgressBar { value } => assert_eq!(*value, 1.0),
            _ => panic!("expected ProgressBar"),
        }
        let node2: UiNode<TestAction> = UiNode::progress_bar(-0.5);
        match &node2.kind {
            WidgetKind::ProgressBar { value } => assert_eq!(*value, 0.0),
            _ => panic!("expected ProgressBar"),
        }
    }

    #[test]
    fn widget_kind_tag() {
        assert_eq!(WidgetKind::<TestAction>::Column.tag(), "column");
        assert_eq!(WidgetKind::<TestAction>::Row.tag(), "row");
        assert_eq!(WidgetKind::<TestAction>::Label { text: "x".into() }.tag(), "label");
    }

    // ── Macro tests ──

    #[test]
    fn macro_single_label() {
        let tree: crate::node::UiTree<TestAction> = ui! {
            label("hi")
        };
        assert_eq!(tree.roots.len(), 1);
        assert!(matches!(&tree.roots[0].kind, WidgetKind::Label { text } if text == "hi"));
    }

    #[test]
    fn macro_label_format() {
        let score = 42;
        let tree: crate::node::UiTree<TestAction> = ui! {
            label("Score: {}", score)
        };
        match &tree.roots[0].kind {
            WidgetKind::Label { text } => assert_eq!(text, "Score: 42"),
            _ => panic!("expected Label"),
        }
    }

    #[test]
    fn macro_column_with_children() {
        let tree: crate::node::UiTree<TestAction> = ui! {
            column(anchor = Anchor::TopLeft, padding = 8.0) [
                label("A"),
                label("B"),
            ]
        };
        assert_eq!(tree.roots.len(), 1);
        assert!(matches!(tree.roots[0].kind, WidgetKind::Column));
        assert_eq!(tree.roots[0].children.len(), 2);
        assert_eq!(tree.roots[0].anchor, Some(Anchor::TopLeft));
    }

    #[test]
    fn macro_nested() {
        let tree: crate::node::UiTree<TestAction> = ui! {
            column() [
                row(gap = 4.0) [
                    label("a"),
                    label("b"),
                ],
            ]
        };
        assert_eq!(tree.roots[0].children.len(), 1);
        let row = &tree.roots[0].children[0];
        assert!(matches!(row.kind, WidgetKind::Row));
        assert_eq!(row.children.len(), 2);
        assert_eq!(row.gap, 4.0);
    }

    #[test]
    fn macro_button_on_click() {
        let tree = ui! {
            button("Resume", on_click = TestAction::Resume)
        };
        match &tree.roots[0].kind {
            WidgetKind::Button { text, on_click } => {
                assert_eq!(text, "Resume");
                assert_eq!(on_click.as_ref(), Some(&TestAction::Resume));
            }
            _ => panic!("expected Button"),
        }
    }

    #[test]
    fn macro_conditional() {
        let paused = true;
        let tree: crate::node::UiTree<TestAction> = ui! {
            if paused {
                label("PAUSED")
            }
        };
        assert_eq!(tree.roots.len(), 1);

        let not_paused = false;
        let tree2: crate::node::UiTree<TestAction> = ui! {
            if not_paused {
                label("PAUSED")
            }
        };
        assert_eq!(tree2.roots.len(), 0);
    }

    #[test]
    fn macro_empty_column() {
        let tree: crate::node::UiTree<TestAction> = ui! {
            column() []
        };
        assert_eq!(tree.roots.len(), 1);
        assert!(tree.roots[0].children.is_empty());
    }

    #[test]
    fn macro_multiple_roots() {
        let tree: crate::node::UiTree<TestAction> = ui! {
            column(anchor = Anchor::TopLeft) [
                label("HUD"),
            ]
            column(anchor = Anchor::TopRight) [
                label("Score"),
            ]
        };
        assert_eq!(tree.roots.len(), 2);
    }
}
