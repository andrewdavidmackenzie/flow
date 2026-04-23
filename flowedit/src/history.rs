//! Undo/redo history for flow editing operations.
//!
//! Each editing action is recorded as an [`EditAction`] that stores enough
//! information to both undo and redo the operation. The history is lost
//! when the application exits.

use flowcore::model::connection::Connection;
use flowcore::model::input::InputInitializer;
use flowcore::model::name::Name;
use flowcore::model::process::Process;
use flowcore::model::process_reference::ProcessReference;

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
    /// A node was deleted. Stores the process reference and subprocess for restoration.
    DeleteNode {
        /// Index where the node was
        index: usize,
        /// The deleted process reference
        process_ref: ProcessReference,
        /// The removed subprocess definition, if any
        subprocess: Option<(Name, Process)>,
        /// Connections that were removed with the node
        removed_connections: Vec<Connection>,
    },
    /// A connection was created.
    CreateConnection {
        /// The new connection
        connection: Connection,
    },
    /// A connection was deleted.
    DeleteConnection {
        /// Index where the connection was
        index: usize,
        /// The deleted connection
        connection: Connection,
    },
    /// An input initializer was changed.
    EditInitializer {
        /// Node index
        node_index: usize,
        /// Port name
        port_name: String,
        /// Previous initializer (None if there was none)
        old_init: Option<InputInitializer>,
        /// New initializer (None if removed)
        new_init: Option<InputInitializer>,
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
                if let Some(pref) = win.flow_definition.process_refs.get_mut(index) {
                    pref.x = Some(old_x);
                    pref.y = Some(old_y);
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
                if let Some(pref) = win.flow_definition.process_refs.get_mut(index) {
                    pref.x = Some(old_x);
                    pref.y = Some(old_y);
                    pref.width = Some(old_w);
                    pref.height = Some(old_h);
                }
                win.status = String::from("Undo: resize");
            }
            EditAction::DeleteNode {
                index,
                process_ref,
                subprocess,
                removed_connections,
            } => {
                win.flow_definition.process_refs.insert(index, process_ref);
                if let Some((name, proc)) = subprocess {
                    win.flow_definition.subprocesses.insert(name, proc);
                }
                win.flow_definition.connections.extend(removed_connections);
                win.status = String::from("Undo: delete node");
            }
            EditAction::CreateConnection { connection } => {
                let from_str = connection.from().to_string();
                let to_strs: Vec<String> =
                    connection.to().iter().map(ToString::to_string).collect();
                win.flow_definition.connections.retain(|c| {
                    c.from().to_string() != from_str
                        || c.to().iter().map(ToString::to_string).collect::<Vec<_>>() != to_strs
                });
                win.status = String::from("Undo: create connection");
            }
            EditAction::DeleteConnection { index, connection } => {
                win.flow_definition.connections.insert(index, connection);
                win.status = String::from("Undo: delete connection");
            }
            EditAction::EditInitializer {
                node_index,
                ref port_name,
                ref old_init,
                ..
            } => {
                initializer::apply_initializer_state(win, node_index, port_name, old_init.as_ref());
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
                if let Some(pref) = win.flow_definition.process_refs.get_mut(index) {
                    pref.x = Some(new_x);
                    pref.y = Some(new_y);
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
                if let Some(pref) = win.flow_definition.process_refs.get_mut(index) {
                    pref.x = Some(new_x);
                    pref.y = Some(new_y);
                    pref.width = Some(new_w);
                    pref.height = Some(new_h);
                }
                win.status = String::from("Redo: resize");
            }
            EditAction::DeleteNode {
                index,
                subprocess,
                removed_connections,
                ..
            } => {
                if index < win.flow_definition.process_refs.len() {
                    let removed = win.flow_definition.process_refs.remove(index);
                    let alias = if removed.alias.is_empty() {
                        crate::canvas_view::derive_short_name(&removed.source)
                    } else {
                        removed.alias.clone()
                    };
                    win.flow_definition.subprocesses.remove(&alias);
                }
                // Also remove re-inserted subprocess if it was restored during undo
                if let Some((ref name, _)) = subprocess {
                    win.flow_definition.subprocesses.remove(name);
                }
                for conn in &removed_connections {
                    let from_str = conn.from().to_string();
                    let to_strs: Vec<String> = conn.to().iter().map(ToString::to_string).collect();
                    win.flow_definition.connections.retain(|c| {
                        c.from().to_string() != from_str
                            || c.to().iter().map(ToString::to_string).collect::<Vec<_>>() != to_strs
                    });
                }
                win.status = String::from("Redo: delete node");
            }
            EditAction::CreateConnection { connection } => {
                win.flow_definition.connections.push(connection);
                win.status = String::from("Redo: create connection");
            }
            EditAction::DeleteConnection { index, .. } => {
                if index < win.flow_definition.connections.len() {
                    win.flow_definition.connections.remove(index);
                }
                win.status = String::from("Redo: delete connection");
            }
            EditAction::EditInitializer {
                node_index,
                ref port_name,
                ref new_init,
                ..
            } => {
                initializer::apply_initializer_state(win, node_index, port_name, new_init.as_ref());
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
    use std::collections::BTreeMap;

    fn test_pref() -> ProcessReference {
        ProcessReference {
            alias: "test".into(),
            source: "lib://test".into(),
            initializations: BTreeMap::new(),
            x: Some(100.0),
            y: Some(100.0),
            width: Some(180.0),
            height: Some(120.0),
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
        let pref = test_pref();
        history.record(EditAction::DeleteNode {
            index: 0,
            process_ref: pref.clone(),
            subprocess: None,
            removed_connections: vec![],
        });
        let action = history.undo().expect("Should have action");
        match action {
            EditAction::DeleteNode {
                index,
                process_ref: pr,
                ..
            } => {
                assert_eq!(index, 0);
                assert_eq!(pr.alias, pref.alias);
            }
            _ => panic!("Expected DeleteNode"),
        }
    }

    #[test]
    fn create_connection_roundtrip() {
        use crate::canvas_view::split_route;
        let mut history = EditHistory::default();
        let connection = Connection::new("a/out", "b/in");
        history.record(EditAction::CreateConnection {
            connection: connection.clone(),
        });
        let action = history.undo().expect("Should have action");
        match action {
            EditAction::CreateConnection { connection: c } => {
                let (from_node, _) = split_route(c.from().as_ref());
                let (to_node, _) =
                    split_route(c.to().first().expect("should have to route").as_ref());
                assert_eq!(from_node, "a");
                assert_eq!(to_node, "b");
            }
            _ => panic!("Expected CreateConnection"),
        }
    }

    // --- Tests moved from ui_test.rs (direct function calls, no message routing) ---

    fn test_win_state() -> WindowState {
        use flowcore::model::flow_definition::FlowDefinition;
        use flowcore::model::name::Name;

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

        WindowState {
            flow_definition: flow,
            is_root: true,
            ..Default::default()
        }
    }

    #[test]
    fn record_and_undo_edit() {
        let mut flow_def = flowcore::model::flow_definition::FlowDefinition {
            name: String::from("test"),
            ..flowcore::model::flow_definition::FlowDefinition::default()
        };
        flow_def.process_refs.push(ProcessReference {
            alias: "a".into(),
            source: "lib://test".into(),
            initializations: BTreeMap::new(),
            x: Some(100.0),
            y: Some(100.0),
            width: Some(180.0),
            height: Some(120.0),
        });
        let mut win = WindowState {
            flow_definition: flow_def,
            is_root: true,
            ..Default::default()
        };

        // Move node
        if let Some(pref) = win.flow_definition.process_refs.first_mut() {
            pref.x = Some(200.0);
            pref.y = Some(300.0);
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
        let pref = win.flow_definition.process_refs.first();
        assert!(pref.is_some_and(|p| (p.x.unwrap_or(0.0) - 100.0).abs() < 0.01));
        assert!(pref.is_some_and(|p| (p.y.unwrap_or(0.0) - 100.0).abs() < 0.01));

        // Redo
        handle_redo(&mut win);
        let pref = win.flow_definition.process_refs.first();
        assert!(pref.is_some_and(|p| (p.x.unwrap_or(0.0) - 200.0).abs() < 0.01));
        assert!(pref.is_some_and(|p| (p.y.unwrap_or(0.0) - 300.0).abs() < 0.01));
    }

    #[test]
    fn undo_redo_resize_node() {
        let mut win = test_win_state();
        // Simulate resize
        if let Some(pref) = win.flow_definition.process_refs.first_mut() {
            pref.width = Some(250.0);
            pref.height = Some(180.0);
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
        let pref = win.flow_definition.process_refs.first();
        assert!(pref.is_some_and(|p| (p.width.unwrap_or(0.0) - 180.0).abs() < 0.01));
        assert!(pref.is_some_and(|p| (p.height.unwrap_or(0.0) - 120.0).abs() < 0.01));

        // Redo
        handle_redo(&mut win);
        let pref = win.flow_definition.process_refs.first();
        assert!(pref.is_some_and(|p| (p.width.unwrap_or(0.0) - 250.0).abs() < 0.01));
        assert!(pref.is_some_and(|p| (p.height.unwrap_or(0.0) - 180.0).abs() < 0.01));
    }

    #[test]
    fn undo_redo_delete_node() {
        let mut win = test_win_state();
        assert_eq!(win.flow_definition.process_refs.len(), 2);
        let removed_pref = win.flow_definition.process_refs.remove(0);
        record_edit(
            &mut win,
            EditAction::DeleteNode {
                index: 0,
                process_ref: removed_pref,
                subprocess: None,
                removed_connections: Vec::new(),
            },
        );
        assert_eq!(win.flow_definition.process_refs.len(), 1);

        // Undo restores
        handle_undo(&mut win);
        assert_eq!(win.flow_definition.process_refs.len(), 2);

        // Redo removes again
        handle_redo(&mut win);
        assert_eq!(win.flow_definition.process_refs.len(), 1);
    }

    #[test]
    fn undo_redo_create_connection() {
        let mut win = test_win_state();
        let connection = Connection::new("add/out", "stdout/in");
        win.flow_definition.connections.push(connection.clone());
        record_edit(&mut win, EditAction::CreateConnection { connection });
        assert_eq!(win.flow_definition.connections.len(), 1);

        // Undo removes
        handle_undo(&mut win);
        assert_eq!(win.flow_definition.connections.len(), 0);

        // Redo re-adds
        handle_redo(&mut win);
        assert_eq!(win.flow_definition.connections.len(), 1);
    }

    #[test]
    fn undo_redo_delete_connection() {
        let mut win = test_win_state();
        win.flow_definition
            .connections
            .push(Connection::new("add/out", "stdout/in"));
        let removed_conn = win.flow_definition.connections.remove(0);
        record_edit(
            &mut win,
            EditAction::DeleteConnection {
                index: 0,
                connection: removed_conn,
            },
        );
        assert_eq!(win.flow_definition.connections.len(), 0);

        // Undo restores
        handle_undo(&mut win);
        assert_eq!(win.flow_definition.connections.len(), 1);

        // Redo removes again
        handle_redo(&mut win);
        assert_eq!(win.flow_definition.connections.len(), 0);
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
                new_init: Some(InputInitializer::Once(serde_json::json!(42))),
            },
        );

        // Apply the new state manually (record_edit only records, doesn't apply)
        initializer::apply_initializer_state(
            &mut win,
            0,
            "input",
            Some(&InputInitializer::Once(serde_json::json!(42))),
        );
        assert!(win
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_some());

        // Undo
        handle_undo(&mut win);
        assert!(win
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_none());

        // Redo
        handle_redo(&mut win);
        assert!(win
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_some());
    }
}
