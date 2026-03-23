//! Tree diffing engine — compares two UI trees to detect changes.

use crate::node::{UiNode, UiTree, WidgetKind};

/// Identifies a node across frames by its position in the tree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey {
    /// Path from root: each element is the child index at that depth.
    pub path: Vec<u16>,
}

impl NodeKey {
    pub fn root(index: u16) -> Self {
        Self { path: vec![index] }
    }

    pub fn child(&self, index: u16) -> Self {
        let mut path = self.path.clone();
        path.push(index);
        Self { path }
    }
}

/// What changed for a node between frames.
#[derive(Clone, Debug, PartialEq)]
pub enum DiffOp {
    /// Node was just added (exists in new tree but not old).
    Added(NodeKey),
    /// Node was removed (exists in old tree but not new).
    Removed(NodeKey),
    /// Node exists in both trees but its content changed.
    Updated(NodeKey),
    /// Node exists in both trees and is identical.
    Unchanged(NodeKey),
}

/// Diff two UI trees, producing a list of diff operations.
pub fn diff_trees<E>(prev: &UiTree<E>, next: &UiTree<E>) -> Vec<DiffOp> {
    let mut ops = Vec::new();
    let max_roots = prev.roots.len().max(next.roots.len());

    for i in 0..max_roots {
        let key = NodeKey::root(i as u16);
        match (prev.roots.get(i), next.roots.get(i)) {
            (Some(old), Some(new)) => diff_node(old, new, &key, &mut ops),
            (None, Some(new)) => add_subtree(new, &key, &mut ops),
            (Some(old), None) => remove_subtree(old, &key, &mut ops),
            (None, None) => {}
        }
    }

    ops
}

/// Diff two individual nodes.
fn diff_node<E>(old: &UiNode<E>, new: &UiNode<E>, key: &NodeKey, ops: &mut Vec<DiffOp>) {
    // Different widget kind → remove old, add new
    if old.kind.tag() != new.kind.tag() {
        remove_subtree(old, key, ops);
        add_subtree(new, key, ops);
        return;
    }

    // Same kind — check if content changed
    if content_changed(old, new) {
        ops.push(DiffOp::Updated(key.clone()));
    } else {
        ops.push(DiffOp::Unchanged(key.clone()));
    }

    // Diff children
    let max_children = old.children.len().max(new.children.len());
    for i in 0..max_children {
        let child_key = key.child(i as u16);
        match (old.children.get(i), new.children.get(i)) {
            (Some(old_child), Some(new_child)) => diff_node(old_child, new_child, &child_key, ops),
            (None, Some(new_child)) => add_subtree(new_child, &child_key, ops),
            (Some(old_child), None) => remove_subtree(old_child, &child_key, ops),
            (None, None) => {}
        }
    }
}

/// Mark an entire subtree as Added.
fn add_subtree<E>(node: &UiNode<E>, key: &NodeKey, ops: &mut Vec<DiffOp>) {
    ops.push(DiffOp::Added(key.clone()));
    for (i, child) in node.children.iter().enumerate() {
        add_subtree(child, &key.child(i as u16), ops);
    }
}

/// Mark an entire subtree as Removed.
fn remove_subtree<E>(node: &UiNode<E>, key: &NodeKey, ops: &mut Vec<DiffOp>) {
    ops.push(DiffOp::Removed(key.clone()));
    for (i, child) in node.children.iter().enumerate() {
        remove_subtree(child, &key.child(i as u16), ops);
    }
}

/// Check if the content of two same-kind nodes differs.
fn content_changed<E>(old: &UiNode<E>, new: &UiNode<E>) -> bool {
    match (&old.kind, &new.kind) {
        (WidgetKind::Label { text: a }, WidgetKind::Label { text: b }) => a != b,
        (WidgetKind::Button { text: a, .. }, WidgetKind::Button { text: b, .. }) => a != b,
        (WidgetKind::ProgressBar { value: a }, WidgetKind::ProgressBar { value: b }) => {
            (a - b).abs() > f32::EPSILON
        }
        (WidgetKind::Icon { texture: a }, WidgetKind::Icon { texture: b }) => a != b,
        (WidgetKind::Spacer { size: a }, WidgetKind::Spacer { size: b }) => {
            (a - b).abs() > f32::EPSILON
        }
        // Containers: content is the same (children are diffed separately)
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{UiNode, UiTree};

    type Node = UiNode<()>;

    #[test]
    fn identical_trees() {
        let tree = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::label("A"),
                Node::label("B"),
            ]),
        ]);
        let ops = diff_trees(&tree, &tree);
        assert!(ops.iter().all(|op| matches!(op, DiffOp::Unchanged(_))));
        assert_eq!(ops.len(), 3); // column + 2 labels
    }

    #[test]
    fn add_child() {
        let tree1 = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::label("A"),
                Node::label("B"),
            ]),
        ]);
        let tree2 = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::label("A"),
                Node::label("B"),
                Node::label("C"),
            ]),
        ]);
        let ops = diff_trees(&tree1, &tree2);
        let added: Vec<_> = ops.iter().filter(|op| matches!(op, DiffOp::Added(_))).collect();
        assert_eq!(added.len(), 1);
    }

    #[test]
    fn remove_child() {
        let tree1 = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::label("A"),
                Node::label("B"),
                Node::label("C"),
            ]),
        ]);
        let tree2 = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::label("A"),
                Node::label("B"),
            ]),
        ]);
        let ops = diff_trees(&tree1, &tree2);
        let removed: Vec<_> = ops.iter().filter(|op| matches!(op, DiffOp::Removed(_))).collect();
        assert_eq!(removed.len(), 1);
    }

    #[test]
    fn change_text() {
        let tree1 = UiTree::new(vec![Node::label("old")]);
        let tree2 = UiTree::new(vec![Node::label("new")]);
        let ops = diff_trees(&tree1, &tree2);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], DiffOp::Updated(_)));
    }

    #[test]
    fn change_widget_kind() {
        let tree1 = UiTree::new(vec![Node::label("x")]);
        let tree2 = UiTree::new(vec![Node::button("x")]);
        let ops = diff_trees(&tree1, &tree2);
        assert!(ops.iter().any(|op| matches!(op, DiffOp::Removed(_))));
        assert!(ops.iter().any(|op| matches!(op, DiffOp::Added(_))));
    }

    #[test]
    fn deep_nesting_diff() {
        let tree1 = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::row().with_children(vec![
                    Node::label("old"),
                ]),
            ]),
        ]);
        let tree2 = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::row().with_children(vec![
                    Node::label("new"),
                ]),
            ]),
        ]);
        let ops = diff_trees(&tree1, &tree2);
        // column=unchanged, row=unchanged, label=updated
        assert_eq!(ops.len(), 3);
        assert!(matches!(&ops[2], DiffOp::Updated(_)));
    }

    #[test]
    fn conditional_subtree_removed() {
        let tree1 = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::panel().with_children(vec![
                    Node::label("Inner"),
                ]),
                Node::label("Always"),
            ]),
        ]);
        let tree2 = UiTree::new(vec![
            Node::column().with_children(vec![
                Node::label("Always"),
            ]),
        ]);
        let ops = diff_trees(&tree1, &tree2);
        // Panel and its children should be removed or replaced
        let removed: Vec<_> = ops.iter().filter(|op| matches!(op, DiffOp::Removed(_))).collect();
        assert!(removed.len() >= 1, "panel subtree should produce removals");
    }
}
