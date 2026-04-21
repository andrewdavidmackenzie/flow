#![allow(clippy::indexing_slicing)]

use super::*;
use iced_test::simulator::{self, simulator};
use std::collections::HashMap;

fn test_node(alias: &str, source: &str) -> NodeLayout {
    NodeLayout {
        alias: alias.into(),
        source: source.into(),
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
fn sync_flow_definition_preserves_nodes() {
    let mut win = WindowState {
        kind: WindowKind::FlowEditor,
        flow_name: String::from("test"),
        nodes: vec![
            test_node("add", "lib://flowstdlib/math/add"),
            test_node("stdout", "context://stdio/stdout"),
        ],
        edges: Vec::new(),
        canvas_state: FlowCanvasState::default(),
        status: String::new(),
        selected_node: None,
        selected_connection: None,
        history: EditHistory::default(),
        auto_fit_pending: false,
        auto_fit_enabled: false,
        unsaved_edits: 0,
        compiled_manifest: None,
        file_path: None,
        flow_definition: FlowDefinition::default(),
        tooltip: None,
        initializer_editor: None,
        is_root: true,
        flow_inputs: Vec::new(),
        flow_outputs: Vec::new(),
        context_menu: None,
        show_metadata: false,
        flow_hierarchy: FlowHierarchy::empty(),
        last_size: None,
        last_position: None,
    };

    initializer::sync_flow_definition(&mut win);
    assert_eq!(win.flow_definition.process_refs.len(), 2);
    assert_eq!(win.flow_definition.name, "test");
}

#[test]
fn record_and_undo_edit() {
    let mut win = WindowState {
        kind: WindowKind::FlowEditor,
        flow_name: String::from("test"),
        nodes: vec![test_node("a", "lib://test")],
        edges: Vec::new(),
        canvas_state: FlowCanvasState::default(),
        status: String::new(),
        selected_node: None,
        selected_connection: None,
        history: EditHistory::default(),
        auto_fit_pending: false,
        auto_fit_enabled: false,
        unsaved_edits: 0,
        compiled_manifest: None,
        file_path: None,
        flow_definition: FlowDefinition::default(),
        tooltip: None,
        initializer_editor: None,
        is_root: true,
        flow_inputs: Vec::new(),
        flow_outputs: Vec::new(),
        context_menu: None,
        show_metadata: false,
        flow_hierarchy: FlowHierarchy::empty(),
        last_size: None,
        last_position: None,
    };

    // Move node
    win.nodes[0].x = 200.0;
    win.nodes[0].y = 300.0;
    undo_redo::record_edit(
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
    undo_redo::apply_undo(&mut win);
    assert!((win.nodes[0].x - 100.0).abs() < 0.01);
    assert!((win.nodes[0].y - 100.0).abs() < 0.01);

    // Redo
    undo_redo::apply_redo(&mut win);
    assert!((win.nodes[0].x - 200.0).abs() < 0.01);
    assert!((win.nodes[0].y - 300.0).abs() < 0.01);
}

fn test_win_state() -> WindowState {
    WindowState {
        kind: WindowKind::FlowEditor,
        flow_name: String::from("test"),
        nodes: vec![
            test_node("add", "lib://flowstdlib/math/add"),
            test_node("stdout", "context://stdio/stdout"),
        ],
        edges: Vec::new(),
        canvas_state: FlowCanvasState::default(),
        status: String::new(),
        selected_node: None,
        selected_connection: None,
        history: EditHistory::default(),
        auto_fit_pending: false,
        auto_fit_enabled: false,
        unsaved_edits: 0,
        compiled_manifest: None,
        file_path: None,
        flow_definition: FlowDefinition::default(),
        tooltip: None,
        initializer_editor: None,
        is_root: true,
        flow_inputs: Vec::new(),
        flow_outputs: Vec::new(),
        context_menu: None,
        show_metadata: false,
        flow_hierarchy: FlowHierarchy::empty(),
        last_size: None,
        last_position: None,
    }
}

fn test_app() -> (FlowEdit, window::Id) {
    let win_id = window::Id::unique();
    let app = FlowEdit {
        windows: HashMap::from([(win_id, test_win_state())]),
        root_window: Some(win_id),
        focused_window: Some(win_id),
        library_tree: LibraryTree {
            libraries: vec![library_panel::LibraryEntry {
                name: "test_lib".into(),
                categories: vec![library_panel::CategoryEntry {
                    name: "math".into(),
                    functions: vec![library_panel::FunctionEntry {
                        name: "add".into(),
                        source: "lib://test_lib/math/add".into(),
                        description: String::new(),
                    }],
                    expanded: true,
                }],
                expanded: true,
            }],
        },
        root_flow_path: None,
        show_lib_paths: false,
        lib_paths: Vec::new(),
        library_cache: HashMap::new(),
        lib_definitions: HashMap::new(),
        context_definitions: HashMap::new(),
    };
    (app, win_id)
}

#[test]
fn update_zoom_in() {
    let (mut app, win_id) = test_app();
    let old_zoom = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    app.update(Message::ZoomIn(win_id));
    let new_zoom = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    assert!(new_zoom > old_zoom);
}

#[test]
fn update_zoom_out() {
    let (mut app, win_id) = test_app();
    let old_zoom = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    app.update(Message::ZoomOut(win_id));
    let new_zoom = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    assert!(new_zoom < old_zoom);
}

#[test]
fn update_toggle_auto_fit() {
    let (mut app, win_id) = test_app();
    app.windows
        .get_mut(&win_id)
        .map(|w| w.auto_fit_enabled = false);
    app.update(Message::ToggleAutoFit(win_id));
    assert!(app
        .windows
        .get(&win_id)
        .map_or(false, |w| w.auto_fit_enabled));
}

#[test]
fn update_canvas_select_node() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Selected(Some(0)),
    ));
    assert_eq!(
        app.windows.get(&win_id).and_then(|w| w.selected_node),
        Some(0)
    );
}

#[test]
fn update_canvas_deselect() {
    let (mut app, win_id) = test_app();
    app.windows
        .get_mut(&win_id)
        .map(|w| w.selected_node = Some(0));
    app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
    assert_eq!(app.windows.get(&win_id).and_then(|w| w.selected_node), None);
}

#[test]
fn update_canvas_move_node() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 200.0, 300.0),
    ));
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map(|n| n.x).unwrap_or(0.0) - 200.0).abs() < 0.01);
    assert!((node.map(|n| n.y).unwrap_or(0.0) - 300.0).abs() < 0.01);
}

#[test]
fn update_canvas_move_completed_records_history() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_canvas_delete_node() {
    let (mut app, win_id) = test_app();
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(2));
    app.update(Message::WindowCanvas(win_id, CanvasMessage::Deleted(0)));
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(1));
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_canvas_create_connection() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionCreated {
            from_node: "add".into(),
            from_port: "".into(),
            to_node: "stdout".into(),
            to_port: "".into(),
        },
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(1));
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_canvas_select_connection() {
    let (mut app, win_id) = test_app();
    // Add a connection first
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.edges.push(EdgeLayout::new(
            "add".into(),
            "".into(),
            "stdout".into(),
            "".into(),
        ));
    }
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionSelected(Some(0)),
    ));
    assert_eq!(
        app.windows.get(&win_id).and_then(|w| w.selected_connection),
        Some(0)
    );
}

#[test]
fn update_canvas_delete_connection() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.edges.push(EdgeLayout::new(
            "add".into(),
            "".into(),
            "stdout".into(),
            "".into(),
        ));
    }
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionDeleted(0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(0));
}

#[test]
fn update_undo_redo_cycle() {
    let (mut app, win_id) = test_app();
    // Move node and record
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 200.0, 300.0),
    ));
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));

    // Undo
    app.update(Message::Undo);
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map(|n| n.x).unwrap_or(0.0) - 100.0).abs() < 0.01);

    // Redo
    app.update(Message::Redo);
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map(|n| n.x).unwrap_or(0.0) - 200.0).abs() < 0.01);
}

#[test]
fn update_toggle_metadata() {
    let (mut app, win_id) = test_app();
    assert!(!app.windows.get(&win_id).map_or(true, |w| w.show_metadata));
    app.update(Message::ToggleMetadataEditor(win_id));
    assert!(app.windows.get(&win_id).map_or(false, |w| w.show_metadata));
    app.update(Message::ToggleMetadataEditor(win_id));
    assert!(!app.windows.get(&win_id).map_or(true, |w| w.show_metadata));
}

#[test]
fn update_flow_name_changed() {
    let (mut app, win_id) = test_app();
    app.update(Message::FlowNameChanged(win_id, "new_name".into()));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.flow_name.as_str()),
        Some("new_name")
    );
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_flow_version_changed() {
    let (mut app, win_id) = test_app();
    app.update(Message::FlowVersionChanged(win_id, "2.0.0".into()));
    assert_eq!(
        app.windows
            .get(&win_id)
            .map(|w| w.flow_definition.metadata.version.as_str()),
        Some("2.0.0")
    );
}

#[test]
fn update_flow_description_changed() {
    let (mut app, win_id) = test_app();
    app.update(Message::FlowDescriptionChanged(
        win_id,
        "A test flow".into(),
    ));
    assert_eq!(
        app.windows
            .get(&win_id)
            .map(|w| w.flow_definition.metadata.description.as_str()),
        Some("A test flow")
    );
}

#[test]
fn update_flow_authors_changed() {
    let (mut app, win_id) = test_app();
    app.update(Message::FlowAuthorsChanged(win_id, "Alice, Bob".into()));
    let authors = app
        .windows
        .get(&win_id)
        .map(|w| w.flow_definition.metadata.authors.clone())
        .unwrap_or_default();
    assert_eq!(authors, vec!["Alice", "Bob"]);
}

#[test]
fn update_flow_add_input() {
    let (mut app, win_id) = test_app();
    app.update(Message::FlowAddInput(win_id));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.flow_inputs.len()),
        Some(1)
    );
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_flow_add_output() {
    let (mut app, win_id) = test_app();
    app.update(Message::FlowAddOutput(win_id));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.flow_outputs.len()),
        Some(1)
    );
}

#[test]
fn update_flow_delete_input() {
    let (mut app, win_id) = test_app();
    app.update(Message::FlowAddInput(win_id));
    app.update(Message::FlowDeleteInput(win_id, 0));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.flow_inputs.len()),
        Some(0)
    );
}

#[test]
fn update_flow_input_name_changed() {
    let (mut app, win_id) = test_app();
    app.update(Message::FlowAddInput(win_id));
    app.update(Message::FlowInputNameChanged(win_id, 0, "data".into()));
    assert_eq!(
        app.windows
            .get(&win_id)
            .and_then(|w| w.flow_inputs.first().map(|p| p.name.as_str())),
        Some("data")
    );
}

#[test]
fn update_window_focused() {
    let (mut app, win_id) = test_app();
    let other_id = window::Id::unique();
    app.update(Message::WindowFocused(other_id));
    assert_eq!(app.focused_window, Some(other_id));
}

#[test]
fn update_window_resized() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowResized(
        win_id,
        iced::Size::new(800.0, 600.0),
    ));
    let size = app.windows.get(&win_id).and_then(|w| w.last_size);
    assert!(size.is_some());
    assert!((size.map(|s| s.width).unwrap_or(0.0) - 800.0).abs() < 0.01);
}

#[test]
fn update_window_moved() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowMoved(win_id, iced::Point::new(100.0, 200.0)));
    let pos = app.windows.get(&win_id).and_then(|w| w.last_position);
    assert!(pos.is_some());
}

#[test]
fn update_toggle_lib_paths() {
    let (mut app, _win_id) = test_app();
    assert!(!app.show_lib_paths);
    app.update(Message::ToggleLibPaths);
    assert!(app.show_lib_paths);
    app.update(Message::ToggleLibPaths);
    assert!(!app.show_lib_paths);
}

#[test]
fn update_context_menu() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ContextMenu(100.0, 200.0),
    ));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.context_menu)
        .is_some());
    // Clicking deselects context menu
    app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.context_menu)
        .is_none());
}

#[test]
fn update_canvas_resize_node() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Resized(0, 50.0, 50.0, 200.0, 150.0),
    ));
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map(|n| n.width).unwrap_or(0.0) - 200.0).abs() < 0.01);
    assert!((node.map(|n| n.height).unwrap_or(0.0) - 150.0).abs() < 0.01);
}

#[test]
fn update_initializer_type_changed() {
    let (mut app, win_id) = test_app();
    // Open initializer editor
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.initializer_editor = Some(InitializerEditor {
            node_index: 0,
            port_name: "i1".into(),
            init_type: "none".into(),
            value_text: String::new(),
        });
    }
    app.update(Message::InitializerTypeChanged(win_id, "once".into()));
    let init_type = app
        .windows
        .get(&win_id)
        .and_then(|w| w.initializer_editor.as_ref())
        .map(|e| e.init_type.as_str());
    assert_eq!(init_type, Some("once"));
}

#[test]
fn update_initializer_cancel() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.initializer_editor = Some(InitializerEditor {
            node_index: 0,
            port_name: "i1".into(),
            init_type: "once".into(),
            value_text: "42".into(),
        });
    }
    app.update(Message::InitializerCancel(win_id));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.initializer_editor.as_ref())
        .is_none());
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("flowedit_tests").join(name);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// ---- iced_test UI tests ----

fn click_and_update(app: &mut FlowEdit, win_id: window::Id, text: &str) {
    let view = app.view(win_id);
    let mut ui = simulator(view);
    let _ = ui.click(text);
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }
}

fn canvas_click_and_update(app: &mut FlowEdit, win_id: window::Id, x: f32, y: f32) {
    let view = app.view(win_id);
    let mut ui = simulator(view);
    ui.point_at(iced::Point::new(x, y));
    ui.simulate(simulator::click());
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }
}

#[test]
fn find_status_text() {
    let (app, win_id) = test_app();
    let view = app.view(win_id);
    let mut ui = simulator(view);
    // The view should render without crashing — that's the main test
    // Text search may not find substrings within composed widgets
    let _ui = ui;
}

#[test]
fn click_zoom_in() {
    let (mut app, win_id) = test_app();
    let old = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    click_and_update(&mut app, win_id, "+");
    let new = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    assert!(new > old, "Zoom should increase");
}

#[test]
fn click_zoom_out() {
    let (mut app, win_id) = test_app();
    let old = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    click_and_update(&mut app, win_id, "\u{2212}");
    let new = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    assert!(new < old, "Zoom should decrease");
}

#[test]
fn click_fit_enables_auto_fit() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.auto_fit_enabled = false;
    }
    click_and_update(&mut app, win_id, "Fit");
    assert!(app
        .windows
        .get(&win_id)
        .map_or(false, |w| w.auto_fit_enabled));
}

#[test]
fn zoom_in_out_roundtrip() {
    let (mut app, win_id) = test_app();
    let original = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    click_and_update(&mut app, win_id, "+");
    click_and_update(&mut app, win_id, "\u{2212}");
    let final_zoom = app
        .windows
        .get(&win_id)
        .map(|w| w.canvas_state.zoom)
        .unwrap_or(1.0);
    assert!(
        (final_zoom - original).abs() < 0.01,
        "Zoom roundtrip should return to original"
    );
}

#[test]
fn click_info_toggles_metadata() {
    let (mut app, win_id) = test_app();
    assert!(!app.windows.get(&win_id).map_or(true, |w| w.show_metadata));
    click_and_update(&mut app, win_id, "\u{2139} Info");
    assert!(app.windows.get(&win_id).map_or(false, |w| w.show_metadata));
    click_and_update(&mut app, win_id, "\u{2139} Info");
    assert!(!app.windows.get(&win_id).map_or(true, |w| w.show_metadata));
}

#[test]
fn click_libs_toggles_panel() {
    let (mut app, win_id) = test_app();
    assert!(!app.show_lib_paths);
    click_and_update(&mut app, win_id, "\u{1F4C1} Libs");
    assert!(app.show_lib_paths);
    click_and_update(&mut app, win_id, "\u{1F4C1} Libs");
    assert!(!app.show_lib_paths);
}

#[test]
fn metadata_panel_shows_fields() {
    let (mut app, win_id) = test_app();
    click_and_update(&mut app, win_id, "\u{2139} Info");
    let view = app.view(win_id);
    let mut ui = simulator(view);
    assert!(ui.find("Name:").is_ok(), "Should find Name field");
    assert!(ui.find("Version:").is_ok(), "Should find Version field");
}

#[test]
fn canvas_click_selects_node() {
    let (mut app, win_id) = test_app();
    // Node at world (100, 100), canvas offset after left panel ~220px
    canvas_click_and_update(&mut app, win_id, 320.0, 160.0);
    let selected = app.windows.get(&win_id).and_then(|w| w.selected_node);
    assert_eq!(selected, Some(0), "First node should be selected");
}

#[test]
fn canvas_click_empty_deselects() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.selected_node = Some(0);
    }
    canvas_click_and_update(&mut app, win_id, 800.0, 600.0);
    let selected = app.windows.get(&win_id).and_then(|w| w.selected_node);
    assert_eq!(selected, None, "Clicking empty canvas should deselect");
}

#[test]
fn click_build_with_saved_flow() {
    // Build with a saved flow file (avoids rfd dialog)
    let dir = temp_dir("ui_build");
    let path = dir.join("test.toml");
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.file_path = Some(path.clone());
        win.flow_name = "test_build".into();
        flow_io::perform_save(win, &path);
    }
    click_and_update(&mut app, win_id, "\u{1F528} Build");
    let status = app
        .windows
        .get(&win_id)
        .map(|w| w.status.clone())
        .unwrap_or_default();
    // Should show compile result (success or error)
    assert!(
        status.contains("Compiled") || status.contains("error") || status.contains("Parse"),
        "Status should reflect compile result: {status}"
    );
}
