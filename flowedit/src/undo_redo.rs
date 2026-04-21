//! Undo/redo operations for the flow editor.

use crate::history::EditAction;
use crate::initializer;
use crate::WindowState;

/// Record an edit action in the history and increment the unsaved edit count.
pub(crate) fn record_edit(win: &mut WindowState, action: EditAction) {
    win.history.record(action);
    win.unsaved_edits += 1;
    win.compiled_manifest = None; // Invalidate compilation on any edit
}

/// Apply an undo action -- reverse the last edit.
pub(crate) fn apply_undo(win: &mut WindowState) {
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
pub(crate) fn apply_redo(win: &mut WindowState) {
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
    apply_undo(win);
    win.unsaved_edits = (win.unsaved_edits - 1).max(0);
}

/// Handle redo message -- applies redo and increments unsaved edit count.
pub(crate) fn handle_redo(win: &mut WindowState) {
    apply_redo(win);
    win.unsaved_edits += 1;
}
