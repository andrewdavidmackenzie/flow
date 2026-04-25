//! Initializer editor tests.

#[cfg(test)]
mod test {
    use crate::{InitializerEditor, WindowState};
    use flowcore::model::flow_definition::FlowDefinition;
    use flowcore::model::input::InputInitializer;
    use flowcore::model::name::Name;
    use flowcore::model::process_reference::ProcessReference;
    use std::collections::BTreeMap;

    fn test_flow_def() -> FlowDefinition {
        FlowDefinition {
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
        }
    }

    #[test]
    fn initializer_apply_once() {
        let mut flow_def = test_flow_def();
        let mut win = WindowState::default();
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "once".into(),
            value_text: "42".into(),
        };
        win.apply_initializer_edit(&mut flow_def, &editor);
        assert!(!win.history.is_empty());
        assert!(flow_def
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_some());
    }

    #[test]
    fn initializer_apply_always() {
        let mut flow_def = test_flow_def();
        let mut win = WindowState::default();
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "always".into(),
            value_text: "\"hello\"".into(),
        };
        win.apply_initializer_edit(&mut flow_def, &editor);
        assert!(flow_def
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_some());
    }

    #[test]
    fn initializer_apply_none_removes() {
        let mut flow_def = test_flow_def();
        let mut win = WindowState::default();
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "once".into(),
            value_text: "42".into(),
        };
        win.apply_initializer_edit(&mut flow_def, &editor);
        assert!(flow_def
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
        win.apply_initializer_edit(&mut flow_def, &editor);
        assert!(flow_def
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("input"))
            .is_none());
    }

    #[test]
    fn initializer_apply_state_set_and_remove() {
        let mut flow_def = test_flow_def();
        let mut win = WindowState::default();
        let init = InputInitializer::Once(serde_json::json!(99));
        win.apply_initializer_state(&mut flow_def, 0, "port", Some(&init));
        assert!(flow_def
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("port"))
            .is_some());

        win.apply_initializer_state(&mut flow_def, 0, "port", None);
        assert!(flow_def
            .process_refs
            .first()
            .and_then(|p| p.initializations.get("port"))
            .is_none());
    }

    #[test]
    fn initializer_apply_invalid_type_no_change() {
        let mut flow_def = test_flow_def();
        let mut win = WindowState::default();
        let empty_before = win.history.is_empty();
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "bogus".into(),
            value_text: "42".into(),
        };
        win.apply_initializer_edit(&mut flow_def, &editor);
        assert_eq!(
            win.history.is_empty(),
            empty_before,
            "Invalid type should not create an edit"
        );
    }
}
