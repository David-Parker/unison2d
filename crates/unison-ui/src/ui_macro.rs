//! The `ui!` declarative macro for building UI trees.
//!
//! # Syntax
//!
//! ```ignore
//! use unison_ui::ui;
//!
//! let tree = ui! {
//!     column(anchor = Anchor::TopLeft, padding = 8.0, gap = 4.0) [
//!         label("Score: {}", score),
//!         row(gap = 8.0) [
//!             icon(heart_texture, width = 16.0),
//!             label("x3"),
//!         ],
//!     ]
//!     if paused {
//!         panel(anchor = Anchor::Center, style = menu_style) [
//!             label("PAUSED"),
//!             button("Resume", on_click = Action::Resume),
//!         ]
//!     }
//! };
//! ```
//!
//! The macro produces a `UiTree<E>` containing one or more root nodes.

/// Build a declarative UI tree.
///
/// See the [module docs](crate::ui_macro) for syntax reference.
#[macro_export]
macro_rules! ui {
    // Entry point: parse root-level items into a UiTree
    ( $($body:tt)* ) => {{
        let mut roots: Vec<$crate::node::UiNode<_>> = Vec::new();
        $crate::ui_items!(roots; $($body)*);
        $crate::node::UiTree::new(roots)
    }};
}

/// Internal macro: parse a sequence of items (nodes + conditionals) and push them into a vec.
#[macro_export]
#[doc(hidden)]
macro_rules! ui_items {
    // Base case: nothing left
    ($out:ident;) => {};

    // Conditional: if ident { ... } or if ident.field { ... }
    // We use tt+ to capture the condition since expr can't be followed by {
    ($out:ident; if $($cond:ident).+ { $($body:tt)* } $($rest:tt)*) => {
        if $($cond).+ {
            $crate::ui_items!($out; $($body)*);
        }
        $crate::ui_items!($out; $($rest)*);
    };

    // Conditional with let pattern: if let Some(x) = expr { ... }
    ($out:ident; if let $pat:pat = $($cond:ident).+ { $($body:tt)* } $($rest:tt)*) => {
        if let $pat = $($cond).+ {
            $crate::ui_items!($out; $($body)*);
        }
        $crate::ui_items!($out; $($rest)*);
    };

    // Container widget with children: widget(props...) [ children... ]
    // Followed by comma or nothing
    ($out:ident; $widget:ident ( $($props:tt)* ) [ $($children:tt)* ] , $($rest:tt)*) => {
        $out.push($crate::ui_node!($widget ( $($props)* ) [ $($children)* ]));
        $crate::ui_items!($out; $($rest)*);
    };
    ($out:ident; $widget:ident ( $($props:tt)* ) [ $($children:tt)* ] $($rest:tt)*) => {
        $out.push($crate::ui_node!($widget ( $($props)* ) [ $($children)* ]));
        $crate::ui_items!($out; $($rest)*);
    };

    // Leaf widget without children: widget(props...)
    // Followed by comma or nothing
    ($out:ident; $widget:ident ( $($props:tt)* ) , $($rest:tt)*) => {
        $out.push($crate::ui_node!($widget ( $($props)* )));
        $crate::ui_items!($out; $($rest)*);
    };
    ($out:ident; $widget:ident ( $($props:tt)* ) $($rest:tt)*) => {
        $out.push($crate::ui_node!($widget ( $($props)* )));
        $crate::ui_items!($out; $($rest)*);
    };
}

/// Internal macro: construct a single UiNode from widget syntax.
#[macro_export]
#[doc(hidden)]
macro_rules! ui_node {
    // ── Container with children ──

    (column ( $($props:tt)* ) [ $($children:tt)* ]) => {{
        #[allow(unused_mut)]
        let mut node = $crate::node::UiNode::column();
        $crate::ui_props!(node; $($props)*);
        #[allow(unused_mut)]
        let mut kids: Vec<$crate::node::UiNode<_>> = Vec::new();
        $crate::ui_items!(kids; $($children)*);
        node.children = kids;
        node
    }};

    (row ( $($props:tt)* ) [ $($children:tt)* ]) => {{
        #[allow(unused_mut)]
        let mut node = $crate::node::UiNode::row();
        $crate::ui_props!(node; $($props)*);
        #[allow(unused_mut)]
        let mut kids: Vec<$crate::node::UiNode<_>> = Vec::new();
        $crate::ui_items!(kids; $($children)*);
        node.children = kids;
        node
    }};

    (panel ( $($props:tt)* ) [ $($children:tt)* ]) => {{
        #[allow(unused_mut)]
        let mut node = $crate::node::UiNode::panel();
        $crate::ui_props!(node; $($props)*);
        #[allow(unused_mut)]
        let mut kids: Vec<$crate::node::UiNode<_>> = Vec::new();
        $crate::ui_items!(kids; $($children)*);
        node.children = kids;
        node
    }};

    // ── Leaf widgets ──

    // label("format", args...)
    (label ( $fmt:literal $(, $arg:expr)* )) => {{
        $crate::node::UiNode::label(format!($fmt $(, $arg)*))
    }};

    // label("format", args..., prop = val, ...)
    // We need to separate format args from props. Use a sentinel: style = ...
    (label ( $fmt:literal $(, $arg:expr)* , style = $style:expr )) => {{
        let node = $crate::node::UiNode::label(format!($fmt $(, $arg)*));
        $crate::node::UiNode {
            text_style: Some($style),
            ..node
        }
    }};

    // button("text", on_click = expr) with optional style
    (button ( $text:literal , on_click = $event:expr )) => {{
        let node = $crate::node::UiNode::button($text);
        $crate::node::UiNode {
            kind: $crate::node::WidgetKind::Button {
                text: $text.to_string(),
                on_click: Some($event),
            },
            ..node
        }
    }};

    (button ( $text:literal , on_click = $event:expr , style = $style:expr )) => {{
        let node = $crate::node::UiNode::button($text);
        $crate::node::UiNode {
            kind: $crate::node::WidgetKind::Button {
                text: $text.to_string(),
                on_click: Some($event),
            },
            text_style: Some($style),
            ..node
        }
    }};

    (button ( $text:literal )) => {{
        $crate::node::UiNode::button($text)
    }};

    // icon(texture) or icon(texture, prop = val)
    (icon ( $texture:expr )) => {{
        $crate::node::UiNode::icon($texture)
    }};

    (icon ( $texture:expr , $($props:tt)* )) => {{
        let mut node = $crate::node::UiNode::icon($texture);
        $crate::ui_props!(node; $($props)*);
        node
    }};

    // progress_bar(value)
    (progress_bar ( $value:expr )) => {{
        $crate::node::UiNode::progress_bar($value)
    }};

    (progress_bar ( $value:expr , $($props:tt)* )) => {{
        let mut node = $crate::node::UiNode::progress_bar($value);
        $crate::ui_props!(node; $($props)*);
        node
    }};

    // spacer(size)
    (spacer ( $size:expr )) => {{
        $crate::node::UiNode::spacer($size)
    }};
}

/// Internal macro: apply property assignments to a node.
#[macro_export]
#[doc(hidden)]
macro_rules! ui_props {
    // Base case
    ($node:ident;) => {};

    // anchor = expr
    ($node:ident; anchor = $val:expr $(, $($rest:tt)*)?) => {
        $node.anchor = Some($val);
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // padding = expr
    ($node:ident; padding = $val:expr $(, $($rest:tt)*)?) => {
        $node.padding = $crate::style::EdgeInsets::from($val);
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // gap = expr
    ($node:ident; gap = $val:expr $(, $($rest:tt)*)?) => {
        $node.gap = $val;
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // width = expr
    ($node:ident; width = $val:expr $(, $($rest:tt)*)?) => {
        $node.width = Some($val);
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // height = expr
    ($node:ident; height = $val:expr $(, $($rest:tt)*)?) => {
        $node.height = Some($val);
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // style = expr (panel style)
    ($node:ident; style = $val:expr $(, $($rest:tt)*)?) => {
        $node.panel_style = Some($val);
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // text_style = expr
    ($node:ident; text_style = $val:expr $(, $($rest:tt)*)?) => {
        $node.text_style = Some($val);
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // nine_slice = expr
    ($node:ident; nine_slice = $val:expr $(, $($rest:tt)*)?) => {
        $node.nine_slice = Some($val);
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // key = expr
    ($node:ident; key = $val:expr $(, $($rest:tt)*)?) => {
        $node.key = Some($val);
        $( $crate::ui_props!($node; $($rest)*); )?
    };

    // visible = expr
    ($node:ident; visible = $val:expr $(, $($rest:tt)*)?) => {
        $node.visible = $val;
        $( $crate::ui_props!($node; $($rest)*); )?
    };
}
