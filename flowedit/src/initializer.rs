//! Initializer editor logic for applying and syncing initializer edits.

use flowcore::model::input::InputInitializer;

use crate::canvas_view::derive_short_name;
use crate::history::EditAction;
use crate::{InitializerEditor, WindowState};

/// Apply an initializer edit to the flow definition.
pub(crate) fn apply_initializer_edit(win: &mut WindowState, editor: &InitializerEditor) {
    let alias = win
        .flow_definition
        .process_refs
        .get(editor.node_index)
        .map(|pr| {
            if pr.alias.is_empty() {
                derive_short_name(&pr.source)
            } else {
                pr.alias.clone()
            }
        })
        .unwrap_or_default();

    // Capture old state for undo
    let old_init = win
        .flow_definition
        .process_refs
        .get(editor.node_index)
        .and_then(|pr| pr.initializations.get(&editor.port_name).cloned());

    // Compute new initializer
    let new_init = match editor.init_type.as_str() {
        "none" => None,
        "once" | "always" => {
            let value = serde_json::from_str(&editor.value_text)
                .unwrap_or_else(|_| serde_json::Value::String(editor.value_text.clone()));
            let init = if editor.init_type == "once" {
                InputInitializer::Once(value)
            } else {
                InputInitializer::Always(value)
            };
            Some(init)
        }
        _ => return,
    };

    // Apply to model
    if let Some(pref) = win.flow_definition.process_refs.get_mut(editor.node_index) {
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

    win.history.record(EditAction::EditInitializer {
        node_index: editor.node_index,
        port_name: editor.port_name.clone(),
        old_init,
        new_init,
    });
    win.unsaved_edits += 1;
    win.compiled_manifest = None;
    win.canvas_state.request_redraw();
    win.status = format!("Initializer updated on {}/{}", alias, editor.port_name);
}

/// Apply an initializer state to the model (`process_refs`).
pub(crate) fn apply_initializer_state(
    win: &mut WindowState,
    node_index: usize,
    port_name: &str,
    init: Option<&InputInitializer>,
) {
    if let Some(pref) = win.flow_definition.process_refs.get_mut(node_index) {
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
    use flowcore::model::process_reference::ProcessReference;
    use std::collections::BTreeMap;

    fn test_win_state() -> WindowState {
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
        // Check model was updated
        assert!(win
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_some());
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
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_some());
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
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
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
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_none());
    }

    #[test]
    fn initializer_apply_state_set_and_remove() {
        let mut win = test_win_state();
        let init = InputInitializer::Once(serde_json::json!(99));
        apply_initializer_state(&mut win, 0, "port", Some(&init));
        assert!(win
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("port"))
            .is_some());

        // Remove
        apply_initializer_state(&mut win, 0, "port", None);
        assert!(win
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("port"))
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
