//! Initializer editor logic for applying and syncing initializer edits.

use flowcore::model::input::InputInitializer;
use flowcore::model::process_reference::ProcessReference;

use crate::canvas_view::derive_short_name;
use crate::history::EditAction;
use crate::{InitializerEditor, WindowState};

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
                pr.alias.clone()
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
            pr.alias.clone()
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
/// Connections are stored in `flow_definition.connections` and serialized during save.
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
                    pr.alias.clone()
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
            pr.alias.clone()
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
                node.initializers.insert(port_name.to_string(), d.clone());
            }
            None => {
                node.initializers.remove(port_name);
            }
        }
    }
}

/// Handle initializer type change message.
pub(crate) fn handle_type_changed(win: &mut WindowState, new_type: String) {
    if let Some(ref mut editor) = win.initializer_editor {
        editor.init_type = new_type;
    }
}

/// Handle initializer value change message.
pub(crate) fn handle_value_changed(win: &mut WindowState, new_value: String) {
    if let Some(ref mut editor) = win.initializer_editor {
        editor.value_text = new_value;
    }
}

/// Handle initializer apply message.
pub(crate) fn handle_apply(win: &mut WindowState) {
    if let Some(editor) = win.initializer_editor.take() {
        apply_initializer_edit(win, &editor);
    }
}

/// Handle initializer cancel message.
pub(crate) fn handle_cancel(win: &mut WindowState) {
    win.initializer_editor = None;
}

#[cfg(test)]
mod test {
    use super::*;
    use flowcore::model::flow_definition::FlowDefinition;
    use flowcore::model::name::Name;

    use crate::canvas_view::NodeLayout;

    fn test_node(alias: &str, source: &str) -> NodeLayout {
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
            flow_definition: flow,
            is_root: true,
            ..Default::default()
        }
    }

    #[test]
    fn sync_flow_definition_preserves_nodes() {
        let flow_def = FlowDefinition {
            name: Name::from("test"),
            ..FlowDefinition::default()
        };
        let mut win = WindowState {
            nodes: vec![
                test_node("add", "lib://flowstdlib/math/add"),
                test_node("stdout", "context://stdio/stdout"),
            ],
            flow_definition: flow_def,
            is_root: true,
            ..Default::default()
        };

        sync_flow_definition(&mut win);
        assert_eq!(win.flow_definition.process_refs.len(), 2);
        assert_eq!(win.flow_definition.name, "test");
    }

    #[test]
    fn initializer_apply_once() {
        let mut win = test_win_state();
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "once".into(),
            value_text: "42".into(),
        };
        apply_initializer_edit(&mut win, &editor);
        assert!(win.unsaved_edits > 0);
        // Check display was updated
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_some_and(|d| d.contains("once")));
    }

    #[test]
    fn initializer_apply_always() {
        let mut win = test_win_state();
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "always".into(),
            value_text: "\"hello\"".into(),
        };
        apply_initializer_edit(&mut win, &editor);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_some_and(|d| d.contains("always")));
    }

    #[test]
    fn initializer_apply_none_removes() {
        let mut win = test_win_state();
        // First set one
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "once".into(),
            value_text: "42".into(),
        };
        apply_initializer_edit(&mut win, &editor);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_some());

        // Then remove it
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "none".into(),
            value_text: String::new(),
        };
        apply_initializer_edit(&mut win, &editor);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_none());
    }

    #[test]
    fn initializer_apply_state_set_and_remove() {
        let mut win = test_win_state();
        let init = InputInitializer::Once(serde_json::json!(99));
        let display = "once: 99".to_string();
        apply_initializer_state(&mut win, 0, "port", Some(&init), Some(&display));
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("port"))
            .is_some());

        // Remove
        apply_initializer_state(&mut win, 0, "port", None, None);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("port"))
            .is_none());
    }

    #[test]
    fn initializer_apply_invalid_type_no_change() {
        let mut win = test_win_state();
        let edits_before = win.unsaved_edits;
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "bogus".into(),
            value_text: "42".into(),
        };
        apply_initializer_edit(&mut win, &editor);
        assert_eq!(
            win.unsaved_edits, edits_before,
            "Invalid type should not create an edit"
        );
    }
}
