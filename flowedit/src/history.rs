//! Undo/redo history for flow editing operations.
//!
//! Each editing action is recorded as an [`EditAction`] that stores enough
//! information to both undo and redo the operation. The history is lost
//! when the application exits.

use flowcore::model::input::InputInitializer;

use crate::canvas_view::{EdgeLayout, NodeLayout};

/// An editing action that can be undone and redone.
#[derive(Debug, Clone)]
pub(crate) enum EditAction {
    /// A node was moved from (old_x, old_y) to (new_x, new_y).
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

    #[cfg(test)]
    fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    #[cfg(test)]
    fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
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
}
