//! Undo/redo history for flow editing operations.
//!
//! Each editing action is recorded as an [`EditAction`] that stores enough
//! information to both undo and redo the operation. The history is lost
//! when the application exits.

use flowcore::model::input::InputInitializer;

use crate::canvas_view::{EdgeLayout, NodeLayout};
use crate::initializer;
use crate::WindowState;

/// An editing action that can be undone and redone.
#[derive(Debug, Clone)]
pub(crate) enum EditAction {
    /// A node was moved from (`old_x`, `old_y`) to (`new_x`, `new_y`).
    MoveNode {
        /// Node index
        index: usize,
        /// Previous position
        old_x: f32,
        /// Previous position
        old_y: f32,
        /// New position
        new_x: f32,
        /// New position
        new_y: f32,
    },
    /// A node was resized.
    ResizeNode {
        /// Node index
        index: usize,
        /// Previous geometry
        old_x: f32,
        /// Previous geometry
        old_y: f32,
        /// Previous geometry
        old_w: f32,
        /// Previous geometry
        old_h: f32,
        /// New geometry
        new_x: f32,
        /// New geometry
        new_y: f32,
        /// New geometry
        new_w: f32,
        /// New geometry
        new_h: f32,
    },
    /// A node was deleted. Stores the node and its connected edges for restoration.
    DeleteNode {
        /// Index where the node was
        index: usize,
        /// The deleted node
        node: NodeLayout,
        /// Edges that were removed with the node
        removed_edges: Vec<EdgeLayout>,
    },
    /// A connection was created.
    CreateConnection {
        /// The new edge
        edge: EdgeLayout,
    },
    /// A connection was deleted.
    DeleteConnection {
        /// Index where the edge was
        index: usize,
        /// The deleted edge
        edge: EdgeLayout,
    },
    /// An input initializer was changed.
    EditInitializer {
        /// Node index
        node_index: usize,
        /// Port name
        port_name: String,
        /// Previous initializer (None if there was none)
        old_init: Option<InputInitializer>,
        /// Previous display string (None if there was none)
        old_display: Option<String>,
        /// New initializer (None if removed)
        new_init: Option<InputInitializer>,
        /// New display string (None if removed)
        new_display: Option<String>,
    },
}

/// Edit history supporting undo and redo.
#[derive(Default)]
pub(crate) struct EditHistory {
    /// Stack of actions that can be undone (most recent last)
    undo_stack: Vec<EditAction>,
    /// Stack of actions that can be redone (most recent last)
    redo_stack: Vec<EditAction>,
}

impl EditHistory {
    /// Record a new action. Clears the redo stack since the history has diverged.
    pub(crate) fn record(&mut self, action: EditAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }

    /// Pop the most recent action for undoing. Returns `None` if nothing to undo.
    pub(crate) fn undo(&mut self) -> Option<EditAction> {
        let action = self.undo_stack.pop()?;
        self.redo_stack.push(action.clone());
        Some(action)
    }

    /// Pop the most recent undone action for redoing. Returns `None` if nothing to redo.
    pub(crate) fn redo(&mut self) -> Option<EditAction> {
        let action = self.redo_stack.pop()?;
        self.undo_stack.push(action.clone());
        Some(action)
    }

    /// Returns true if there are actions that can be undone.
    pub(crate) fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are actions that can be redone.
    pub(crate) fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

/// Record an edit action in the history and increment the unsaved edit count.
pub(crate) fn record_edit(win: &mut WindowState, action: EditAction) {
    win.history.record(action);
    win.unsaved_edits += 1;
    win.compiled_manifest = None; // Invalidate compilation on any edit
}

/// Apply an undo action -- reverse the last edit.
fn apply_undo(win: &mut WindowState) {
    if let Some(action) = win.history.undo() {
        match action {
            EditAction::MoveNode {
                index,
                old_x,
                old_y,
                ..
            } => {
                if let Some(node) = win.nodes.get_mut(index) {
                    node.x = old_x;
                    node.y = old_y;
                }
                win.status = String::from("Undo: move");
            }
            EditAction::ResizeNode {
                index,
                old_x,
                old_y,
                old_w,
                old_h,
                ..
            } => {
                if let Some(node) = win.nodes.get_mut(index) {
                    node.x = old_x;
                    node.y = old_y;
                    node.width = old_w;
                    node.height = old_h;
                }
                win.status = String::from("Undo: resize");
            }
            EditAction::DeleteNode {
                index,
                node,
                removed_edges,
            } => {
                win.nodes.insert(index, node);
                win.edges.extend(removed_edges);
                win.status = String::from("Undo: delete node");
            }
            EditAction::CreateConnection { edge } => {
                win.edges.retain(|e| {
                    e.from_node != edge.from_node
                        || e.from_port != edge.from_port
                        || e.to_node != edge.to_node
                        || e.to_port != edge.to_port
                });
                win.status = String::from("Undo: create connection");
            }
            EditAction::DeleteConnection { index, edge } => {
                win.edges.insert(index, edge);
                win.status = String::from("Undo: delete connection");
            }
            EditAction::EditInitializer {
                node_index,
                ref port_name,
                ref old_init,
                ref old_display,
                ..
            } => {
                initializer::apply_initializer_state(
                    win,
                    node_index,
                    port_name,
                    old_init.as_ref(),
                    old_display.as_ref(),
                );
                win.status = String::from("Undo: initializer");
            }
        }
        win.canvas_state.request_redraw();
    }
}

/// Apply a redo action -- re-apply the last undone edit.
fn apply_redo(win: &mut WindowState) {
    if let Some(action) = win.history.redo() {
        match action {
            EditAction::MoveNode {
                index,
                new_x,
                new_y,
                ..
            } => {
                if let Some(node) = win.nodes.get_mut(index) {
                    node.x = new_x;
                    node.y = new_y;
                }
                win.status = String::from("Redo: move");
            }
            EditAction::ResizeNode {
                index,
                new_x,
                new_y,
                new_w,
                new_h,
                ..
            } => {
                if let Some(node) = win.nodes.get_mut(index) {
                    node.x = new_x;
                    node.y = new_y;
                    node.width = new_w;
                    node.height = new_h;
                }
                win.status = String::from("Redo: resize");
            }
            EditAction::DeleteNode {
                index,
                removed_edges,
                ..
            } => {
                if index < win.nodes.len() {
                    win.nodes.remove(index);
                }
                for edge in &removed_edges {
                    win.edges.retain(|e| {
                        e.from_node != edge.from_node
                            || e.from_port != edge.from_port
                            || e.to_node != edge.to_node
                            || e.to_port != edge.to_port
                    });
                }
                win.status = String::from("Redo: delete node");
            }
            EditAction::CreateConnection { edge } => {
                win.edges.push(edge);
                win.status = String::from("Redo: create connection");
            }
            EditAction::DeleteConnection { index, .. } => {
                if index < win.edges.len() {
                    win.edges.remove(index);
                }
                win.status = String::from("Redo: delete connection");
            }
            EditAction::EditInitializer {
                node_index,
                ref port_name,
                ref new_init,
                ref new_display,
                ..
            } => {
                initializer::apply_initializer_state(
                    win,
                    node_index,
                    port_name,
                    new_init.as_ref(),
                    new_display.as_ref(),
                );
                win.status = String::from("Redo: initializer");
            }
        }
        win.canvas_state.request_redraw();
    }
}

/// Handle undo message -- applies undo and decrements unsaved edit count.
pub(crate) fn handle_undo(win: &mut WindowState) {
    if win.history.can_undo() {
        apply_undo(win);
        win.unsaved_edits = (win.unsaved_edits - 1).max(0);
    }
}

/// Handle redo message -- applies redo and increments unsaved edit count.
pub(crate) fn handle_redo(win: &mut WindowState) {
    if win.history.can_redo() {
        apply_redo(win);
        win.unsaved_edits += 1;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    fn test_node() -> NodeLayout {
        NodeLayout {
            alias: "test".into(),
            source: "lib://test".into(),
            description: String::new(),
            x: 100.0,
            y: 100.0,
            width: 180.0,
            height: 120.0,
            inputs: Vec::new(),
            outputs: Vec::new(),
            initializers: HashMap::new(),
        }
    }

    #[test]
    fn record_and_undo() {
        let mut history = EditHistory::default();
        history.record(EditAction::MoveNode {
            index: 0,
            old_x: 0.0,
            old_y: 0.0,
            new_x: 100.0,
            new_y: 100.0,
        });
        assert!(history.can_undo());
        assert!(!history.can_redo());

        let action = history.undo();
        assert!(action.is_some());
        assert!(!history.can_undo());
        assert!(history.can_redo());
    }

    #[test]
    fn redo_after_undo() {
        let mut history = EditHistory::default();
        history.record(EditAction::MoveNode {
            index: 0,
            old_x: 0.0,
            old_y: 0.0,
            new_x: 100.0,
            new_y: 100.0,
        });
        history.undo();
        let action = history.redo();
        assert!(action.is_some());
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn new_action_clears_redo() {
        let mut history = EditHistory::default();
        history.record(EditAction::MoveNode {
            index: 0,
            old_x: 0.0,
            old_y: 0.0,
            new_x: 100.0,
            new_y: 100.0,
        });
        history.undo();
        assert!(history.can_redo());

        history.record(EditAction::MoveNode {
            index: 1,
            old_x: 0.0,
            old_y: 0.0,
            new_x: 50.0,
            new_y: 50.0,
        });
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_empty() {
        let mut history = EditHistory::default();
        assert!(history.undo().is_none());
    }

    #[test]
    fn redo_empty() {
        let mut history = EditHistory::default();
        assert!(history.redo().is_none());
    }

    #[test]
    fn delete_node_roundtrip() {
        let mut history = EditHistory::default();
        let node = test_node();
        history.record(EditAction::DeleteNode {
            index: 0,
            node: node.clone(),
            removed_edges: vec![],
        });
        let action = history.undo().expect("Should have action");
        match action {
            EditAction::DeleteNode { index, node: n, .. } => {
                assert_eq!(index, 0);
                assert_eq!(n.alias, node.alias);
            }
            _ => panic!("Expected DeleteNode"),
        }
    }

    #[test]
    fn create_connection_roundtrip() {
        let mut history = EditHistory::default();
        let edge = EdgeLayout::new("a".into(), "out".into(), "b".into(), "in".into());
        history.record(EditAction::CreateConnection { edge: edge.clone() });
        let action = history.undo().expect("Should have action");
        match action {
            EditAction::CreateConnection { edge: e } => {
                assert_eq!(e.from_node, "a");
                assert_eq!(e.to_node, "b");
            }
            _ => panic!("Expected CreateConnection"),
        }
    }

    // --- Tests moved from ui_test.rs (direct function calls, no message routing) ---

    fn test_win_node(alias: &str, source: &str) -> NodeLayout {
        NodeLayout {
            alias: alias.into(),
            source: source.into(),
            ..Default::default()
        }
    }

    fn test_win_state() -> WindowState {
        use flowcore::model::flow_definition::FlowDefinition;
        use flowcore::model::name::Name;
        use flowcore::model::process_reference::ProcessReference;
        use std::collections::BTreeMap;

        let flow = FlowDefinition {
            name: Name::from("test"),
            process_refs: vec![
                ProcessReference {
                    alias: Name::from("add"),
                    source: "lib://flowstdlib/math/add".into(),
                    initializations: BTreeMap::new(),
                    x: Some(100.0),
                    y: Some(100.0),
                    width: Some(180.0),
                    height: Some(120.0),
                },
                ProcessReference {
                    alias: Name::from("stdout"),
                    source: "context://stdio/stdout".into(),
                    initializations: BTreeMap::new(),
                    x: Some(400.0),
                    y: Some(100.0),
                    width: Some(180.0),
                    height: Some(120.0),
                },
            ],
            ..FlowDefinition::default()
        };

        let nodes: Vec<NodeLayout> = flow
            .process_refs
            .iter()
            .map(|pref| NodeLayout {
                alias: pref.alias.clone(),
                source: pref.source.clone(),
                x: pref.x.unwrap_or(0.0),
                y: pref.y.unwrap_or(0.0),
                width: pref.width.unwrap_or(180.0),
                height: pref.height.unwrap_or(120.0),
                ..Default::default()
            })
            .collect();

        WindowState {
            nodes,
            edges: Vec::new(),
            flow_definition: flow,
            is_root: true,
            ..Default::default()
        }
    }

    #[test]
    fn record_and_undo_edit() {
        let flow_def = flowcore::model::flow_definition::FlowDefinition {
            name: String::from("test"),
            ..flowcore::model::flow_definition::FlowDefinition::default()
        };
        let mut win = WindowState {
            nodes: vec![test_win_node("a", "lib://test")],
            flow_definition: flow_def,
            is_root: true,
            ..Default::default()
        };

        // Move node
        if let Some(n) = win.nodes.first_mut() {
            n.x = 200.0;
            n.y = 300.0;
        }
        record_edit(
            &mut win,
            EditAction::MoveNode {
                index: 0,
                old_x: 100.0,
                old_y: 100.0,
                new_x: 200.0,
                new_y: 300.0,
            },
        );
        assert_eq!(win.unsaved_edits, 1);

        // Undo
        handle_undo(&mut win);
        assert!(win
            .nodes
            .first()
            .is_some_and(|n| (n.x - 100.0).abs() < 0.01));
        assert!(win
            .nodes
            .first()
            .is_some_and(|n| (n.y - 100.0).abs() < 0.01));

        // Redo
        handle_redo(&mut win);
        assert!(win
            .nodes
            .first()
            .is_some_and(|n| (n.x - 200.0).abs() < 0.01));
        assert!(win
            .nodes
            .first()
            .is_some_and(|n| (n.y - 300.0).abs() < 0.01));
    }

    #[test]
    fn undo_redo_resize_node() {
        let mut win = test_win_state();
        // Simulate resize
        if let Some(n) = win.nodes.first_mut() {
            n.width = 250.0;
            n.height = 180.0;
        }
        record_edit(
            &mut win,
            EditAction::ResizeNode {
                index: 0,
                old_x: 100.0,
                old_y: 100.0,
                old_w: 180.0,
                old_h: 120.0,
                new_x: 100.0,
                new_y: 100.0,
                new_w: 250.0,
                new_h: 180.0,
            },
        );

        // Undo
        handle_undo(&mut win);
        assert!(win
            .nodes
            .first()
            .is_some_and(|n| (n.width - 180.0).abs() < 0.01));
        assert!(win
            .nodes
            .first()
            .is_some_and(|n| (n.height - 120.0).abs() < 0.01));

        // Redo
        handle_redo(&mut win);
        assert!(win
            .nodes
            .first()
            .is_some_and(|n| (n.width - 250.0).abs() < 0.01));
        assert!(win
            .nodes
            .first()
            .is_some_and(|n| (n.height - 180.0).abs() < 0.01));
    }

    #[test]
    fn undo_redo_delete_node() {
        let mut win = test_win_state();
        assert_eq!(win.nodes.len(), 2);
        let removed_node = win.nodes.remove(0);
        record_edit(
            &mut win,
            EditAction::DeleteNode {
                index: 0,
                node: removed_node,
                removed_edges: Vec::new(),
            },
        );
        assert_eq!(win.nodes.len(), 1);

        // Undo restores
        handle_undo(&mut win);
        assert_eq!(win.nodes.len(), 2);

        // Redo removes again
        handle_redo(&mut win);
        assert_eq!(win.nodes.len(), 1);
    }

    #[test]
    fn undo_redo_create_connection() {
        let mut win = test_win_state();
        let edge = EdgeLayout::new("add".into(), "out".into(), "stdout".into(), "in".into());
        win.edges.push(edge.clone());
        record_edit(&mut win, EditAction::CreateConnection { edge });
        assert_eq!(win.edges.len(), 1);

        // Undo removes
        handle_undo(&mut win);
        assert_eq!(win.edges.len(), 0);

        // Redo re-adds
        handle_redo(&mut win);
        assert_eq!(win.edges.len(), 1);
    }

    #[test]
    fn undo_redo_delete_connection() {
        let mut win = test_win_state();
        win.edges.push(EdgeLayout::new(
            "add".into(),
            "out".into(),
            "stdout".into(),
            "in".into(),
        ));
        let removed_edge = win.edges.remove(0);
        record_edit(
            &mut win,
            EditAction::DeleteConnection {
                index: 0,
                edge: removed_edge,
            },
        );
        assert_eq!(win.edges.len(), 0);

        // Undo restores
        handle_undo(&mut win);
        assert_eq!(win.edges.len(), 1);

        // Redo removes again
        handle_redo(&mut win);
        assert_eq!(win.edges.len(), 0);
    }

    #[test]
    fn undo_redo_edit_initializer() {
        use flowcore::model::input::InputInitializer;

        let mut win = test_win_state();
        // Record an initializer edit
        record_edit(
            &mut win,
            EditAction::EditInitializer {
                node_index: 0,
                port_name: "input".into(),
                old_init: None,
                old_display: None,
                new_init: Some(InputInitializer::Once(serde_json::json!(42))),
                new_display: Some("once: 42".into()),
            },
        );

        // Apply the new state manually (record_edit only records, doesn't apply)
        initializer::apply_initializer_state(
            &mut win,
            0,
            "input",
            Some(&InputInitializer::Once(serde_json::json!(42))),
            Some(&"once: 42".to_string()),
        );
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_some());

        // Undo
        handle_undo(&mut win);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_none());

        // Redo
        handle_redo(&mut win);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_some());
    }
}
