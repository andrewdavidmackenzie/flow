//! Initializer editor logic for applying and syncing initializer edits.

use flowcore::model::input::InputInitializer;

use crate::history::EditAction;
use crate::utils::derive_short_name;
use crate::{InitializerEditor, WindowState};

impl WindowState {
    pub(crate) fn apply_initializer_edit(&mut self, editor: &InitializerEditor) {
        let alias = self
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

        let old_init = self
            .flow_definition
            .process_refs
            .get(editor.node_index)
            .and_then(|pr| pr.initializations.get(&editor.port_name).cloned());

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

        let Some(pref) = self.flow_definition.process_refs.get_mut(editor.node_index) else {
            return;
        };
        match &new_init {
            Some(init) => {
                pref.initializations
                    .insert(editor.port_name.clone(), init.clone());
            }
            None => {
                pref.initializations.remove(&editor.port_name);
            }
        }

        self.history.record(EditAction::EditInitializer {
            node_index: editor.node_index,
            port_name: editor.port_name.clone(),
            old_init,
            new_init,
        });
        self.canvas_state.request_redraw();
        self.status = format!("Initializer updated on {}/{}", alias, editor.port_name);
    }

    pub(crate) fn apply_initializer_state(
        &mut self,
        node_index: usize,
        port_name: &str,
        init: Option<&InputInitializer>,
    ) {
        if let Some(pref) = self.flow_definition.process_refs.get_mut(node_index) {
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

    pub(crate) fn handle_initializer_type_changed(&mut self, new_type: String) {
        if let Some(ref mut editor) = self.initializer_editor {
            editor.init_type = new_type;
        }
    }

    pub(crate) fn handle_initializer_value_changed(&mut self, new_value: String) {
        if let Some(ref mut editor) = self.initializer_editor {
            editor.value_text = new_value;
        }
    }

    pub(crate) fn handle_initializer_apply(&mut self) {
        if let Some(editor) = self.initializer_editor.take() {
            self.apply_initializer_edit(&editor);
        }
    }

    pub(crate) fn handle_initializer_cancel(&mut self) {
        self.initializer_editor = None;
    }
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
        win.apply_initializer_edit(&editor);
        assert!(!win.history.is_empty());
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
        win.apply_initializer_edit(&editor);
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
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "once".into(),
            value_text: "42".into(),
        };
        win.apply_initializer_edit(&editor);
        assert!(win
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_some());

        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "none".into(),
            value_text: String::new(),
        };
        win.apply_initializer_edit(&editor);
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
        win.apply_initializer_state(0, "port", Some(&init));
        assert!(win
            .flow_definition
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("port"))
            .is_some());

        win.apply_initializer_state(0, "port", None);
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
        let empty_before = win.history.is_empty();
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "bogus".into(),
            value_text: "42".into(),
        };
        win.apply_initializer_edit(&editor);
        assert_eq!(
            win.history.is_empty(),
            empty_before,
            "Invalid type should not create an edit"
        );
    }
}
