//! Initializer editor logic for applying and syncing initializer edits.

use flowcore::model::input::InputInitializer;
use flowcore::model::process_reference::ProcessReference;

use crate::canvas_view::derive_short_name;
use crate::{InitializerEditor, WindowState};
use crate::history::EditAction;

/// Apply an initializer edit to the flow definition and update the node display.
pub(crate) fn apply_initializer_edit(win: &mut WindowState, editor: &InitializerEditor) {
    let alias = win
        .nodes
        .get(editor.node_index)
        .map(|n| n.alias.clone())
        .unwrap_or_default();

    // Capture old state for undo
    let old_init = win
        .flow_definition
        .process_refs
        .iter()
        .find(|pr| {
            let pr_alias = if pr.alias.is_empty() {
                derive_short_name(&pr.source)
            } else {
                pr.alias.to_string()
            };
            pr_alias == alias
        })
        .and_then(|pr| pr.initializations.get(&editor.port_name).cloned());
    let old_display = win
        .nodes
        .get(editor.node_index)
        .and_then(|n| n.initializers.get(&editor.port_name).cloned());

    // Compute new initializer and display
    let (new_init, new_display) = match editor.init_type.as_str() {
        "none" => (None, None),
        "once" | "always" => {
            let value = serde_json::from_str(&editor.value_text)
                .unwrap_or_else(|_| serde_json::Value::String(editor.value_text.clone()));
            let init = if editor.init_type == "once" {
                InputInitializer::Once(value)
            } else {
                InputInitializer::Always(value)
            };
            let display = format!("{}: {}", editor.init_type, editor.value_text);
            (Some(init), Some(display))
        }
        _ => return,
    };

    // Apply to model
    if let Some(pref) = win.flow_definition.process_refs.iter_mut().find(|pr| {
        let pr_alias = if pr.alias.is_empty() {
            derive_short_name(&pr.source)
        } else {
            pr.alias.to_string()
        };
        pr_alias == alias
    }) {
        match &new_init {
            Some(init) => {
                pref.initializations
                    .insert(editor.port_name.clone(), init.clone());
            }
            None => {
                pref.initializations.remove(&editor.port_name);
            }
        }
    }

    // Apply to display
    if let Some(node) = win.nodes.get_mut(editor.node_index) {
        match &new_display {
            Some(display) => {
                node.initializers
                    .insert(editor.port_name.clone(), display.clone());
            }
            None => {
                node.initializers.remove(&editor.port_name);
            }
        }
    }

    win.history.record(EditAction::EditInitializer {
        node_index: editor.node_index,
        port_name: editor.port_name.clone(),
        old_init,
        old_display,
        new_init,
        new_display,
    });
    win.unsaved_edits += 1;
    win.compiled_manifest = None;
    win.canvas_state.request_redraw();
    win.status = format!("Initializer updated on {}/{}", alias, editor.port_name);
}

/// Synchronize the in-memory `FlowDefinition` with the current editor state
/// so that process references and the flow name are up to date.
/// Connections are handled separately via `EdgeLayout` during save.
pub(crate) fn sync_flow_definition(win: &mut WindowState) {
    // Update or rebuild process_refs from current NodeLayout data
    let mut new_refs: Vec<ProcessReference> = Vec::with_capacity(win.nodes.len());
    for node in &win.nodes {
        // Try to find the original ProcessReference by alias to preserve initializations
        let original = win
            .flow_definition
            .process_refs
            .iter()
            .find(|pr| {
                let alias = if pr.alias.is_empty() {
                    derive_short_name(&pr.source)
                } else {
                    pr.alias.to_string()
                };
                alias == node.alias
            })
            .cloned();

        let pref = if let Some(mut orig) = original {
            orig.x = Some(node.x);
            orig.y = Some(node.y);
            orig.width = Some(node.width);
            orig.height = Some(node.height);
            orig
        } else {
            // New node without an original -- build from scratch
            ProcessReference {
                alias: node.alias.clone(),
                source: node.source.clone(),
                initializations: std::collections::BTreeMap::new(),
                x: Some(node.x),
                y: Some(node.y),
                width: Some(node.width),
                height: Some(node.height),
            }
        };
        new_refs.push(pref);
    }
    win.flow_definition.process_refs = new_refs;

    // Update the flow name
    win.flow_definition.name = win.flow_name.clone();
}

/// Apply an initializer state to both the model and display.
pub(crate) fn apply_initializer_state(
    win: &mut WindowState,
    node_index: usize,
    port_name: &str,
    init: Option<&InputInitializer>,
    display: Option<&String>,
) {
    let alias = win
        .nodes
        .get(node_index)
        .map(|n| n.alias.clone())
        .unwrap_or_default();

    if let Some(pref) = win.flow_definition.process_refs.iter_mut().find(|pr| {
        let pr_alias = if pr.alias.is_empty() {
            derive_short_name(&pr.source)
        } else {
            pr.alias.to_string()
        };
        pr_alias == alias
    }) {
        match init {
            Some(i) => {
                pref.initializations
                    .insert(port_name.to_string(), i.clone());
            }
            None => {
                pref.initializations.remove(port_name);
            }
        }
    }

    if let Some(node) = win.nodes.get_mut(node_index) {
        match display {
            Some(d) => {
                node.initializers
                    .insert(port_name.to_string(), d.clone());
            }
            None => {
                node.initializers.remove(port_name);
            }
        }
    }
}
