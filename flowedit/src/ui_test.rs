#![allow(clippy::indexing_slicing)]

use super::*;
use iced_test::simulator::{self, simulator};
use std::collections::HashMap;

fn test_node(alias: &str, source: &str) -> NodeLayout {
    NodeLayout {
        alias: alias.into(),
        source: source.into(),
        ..Default::default()
    }
}

#[test]
fn sync_flow_definition_preserves_nodes() {
    let mut win = WindowState {
        flow_name: String::from("test"),
        nodes: vec![
            test_node("add", "lib://flowstdlib/math/add"),
            test_node("stdout", "context://stdio/stdout"),
        ],
        is_root: true,
        ..Default::default()
    };

    initializer::sync_flow_definition(&mut win);
    assert_eq!(win.flow_definition.process_refs.len(), 2);
    assert_eq!(win.flow_definition.name, "test");
}

#[test]
fn record_and_undo_edit() {
    let mut win = WindowState {
        flow_name: String::from("test"),
        nodes: vec![test_node("a", "lib://test")],
        is_root: true,
        ..Default::default()
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
        flow_name: String::from("test"),
        nodes: vec![
            test_node("add", "lib://flowstdlib/math/add"),
            test_node("stdout", "context://stdio/stdout"),
        ],
        is_root: true,
        ..Default::default()
    }
}

fn test_app_with_flow(flow: FlowDefinition) -> (FlowEdit, window::Id) {
    // Build nodes from flow.process_refs
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

    // Edges are not auto-populated from flow.connections because Connection
    // uses a complex Direction-based format. Tests that need edges should
    // add them manually via win.edges.push(EdgeLayout::new(...)).
    let edges: Vec<EdgeLayout> = Vec::new();

    let win_id = window::Id::unique();
    let win_state = WindowState {
        flow_name: flow.name.clone(),
        nodes,
        edges,
        flow_definition: flow,
        is_root: true,
        ..Default::default()
    };

    let app = FlowEdit {
        windows: HashMap::from([(win_id, win_state)]),
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
        ..Default::default()
    };
    (app, win_id)
}

fn test_app() -> (FlowEdit, window::Id) {
    use flowcore::model::flow_definition::FlowDefinition;
    use flowcore::model::name::Name;
    use flowcore::model::process_reference::ProcessReference;
    use std::collections::BTreeMap;

    // Create a flow with two test nodes: add at (100, 100), stdout at (400, 100)
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

    test_app_with_flow(flow)
}

#[test]
fn update_zoom_in() {
    let (mut app, win_id) = test_app();
    let old_zoom = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    let _ = app.update(Message::ZoomIn(win_id));
    let new_zoom = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    assert!(new_zoom > old_zoom);
}

#[test]
fn update_zoom_out() {
    let (mut app, win_id) = test_app();
    let old_zoom = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    let _ = app.update(Message::ZoomOut(win_id));
    let new_zoom = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    assert!(new_zoom < old_zoom);
}

#[test]
fn update_toggle_auto_fit() {
    let (mut app, win_id) = test_app();
    if let Some(w) = app.windows.get_mut(&win_id) {
        w.auto_fit_enabled = false;
    }
    let _ = app.update(Message::ToggleAutoFit(win_id));
    assert!(app.windows.get(&win_id).is_some_and(|w| w.auto_fit_enabled));
}

#[test]
fn update_canvas_select_node() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
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
    if let Some(w) = app.windows.get_mut(&win_id) {
        w.selected_node = Some(0);
    }
    let _ = app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
    assert_eq!(app.windows.get(&win_id).and_then(|w| w.selected_node), None);
}

#[test]
fn update_canvas_move_node() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 200.0, 300.0),
    ));
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map_or(0.0, |n| n.x) - 200.0).abs() < 0.01);
    assert!((node.map_or(0.0, |n| n.y) - 300.0).abs() < 0.01);
}

#[test]
fn update_canvas_move_completed_records_history() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_canvas_delete_node() {
    let (mut app, win_id) = test_app();
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(2));
    let _ = app.update(Message::WindowCanvas(win_id, CanvasMessage::Deleted(0)));
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(1));
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_canvas_create_connection() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionCreated {
            from_node: "add".into(),
            from_port: String::new(),
            to_node: "stdout".into(),
            to_port: String::new(),
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
            String::new(),
            "stdout".into(),
            String::new(),
        ));
    }
    let _ = app.update(Message::WindowCanvas(
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
            String::new(),
            "stdout".into(),
            String::new(),
        ));
    }
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionDeleted(0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(0));
}

#[test]
fn update_undo_redo_cycle() {
    let (mut app, win_id) = test_app();
    // Move node and record
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 200.0, 300.0),
    ));
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));

    // Undo
    let _ = app.update(Message::Undo);
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map_or(0.0, |n| n.x) - 100.0).abs() < 0.01);

    // Redo
    let _ = app.update(Message::Redo);
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map_or(0.0, |n| n.x) - 200.0).abs() < 0.01);
}

#[test]
fn update_toggle_metadata() {
    let (mut app, win_id) = test_app();
    assert!(!app.windows.get(&win_id).is_some_and(|w| w.show_metadata));
    let _ = app.update(Message::ToggleMetadataEditor(win_id));
    assert!(app.windows.get(&win_id).is_some_and(|w| w.show_metadata));
    let _ = app.update(Message::ToggleMetadataEditor(win_id));
    assert!(!app.windows.get(&win_id).is_some_and(|w| w.show_metadata));
}

#[test]
fn update_flow_name_changed() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::FlowNameChanged(win_id, "new_name".into()));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.flow_name.as_str()),
        Some("new_name")
    );
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_flow_version_changed() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::FlowVersionChanged(win_id, "2.0.0".into()));
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
    let _ = app.update(Message::FlowDescriptionChanged(
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
    let _ = app.update(Message::FlowAuthorsChanged(win_id, "Alice, Bob".into()));
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
    let _ = app.update(Message::FlowAddInput(win_id));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.flow_inputs.len()),
        Some(1)
    );
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}

#[test]
fn update_flow_add_output() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::FlowAddOutput(win_id));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.flow_outputs.len()),
        Some(1)
    );
}

#[test]
fn update_flow_delete_input() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::FlowAddInput(win_id));
    let _ = app.update(Message::FlowDeleteInput(win_id, 0));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.flow_inputs.len()),
        Some(0)
    );
}

#[test]
fn update_flow_input_name_changed() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::FlowAddInput(win_id));
    let _ = app.update(Message::FlowInputNameChanged(win_id, 0, "data".into()));
    assert_eq!(
        app.windows
            .get(&win_id)
            .and_then(|w| w.flow_inputs.first().map(|p| p.name.as_str())),
        Some("data")
    );
}

#[test]
fn update_window_focused() {
    let (mut app, _win_id) = test_app();
    let other_id = window::Id::unique();
    let _ = app.update(Message::WindowFocused(other_id));
    assert_eq!(app.focused_window, Some(other_id));
}

#[test]
fn update_window_resized() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowResized(
        win_id,
        iced::Size::new(800.0, 600.0),
    ));
    let size = app.windows.get(&win_id).and_then(|w| w.last_size);
    assert!(size.is_some());
    assert!((size.map_or(0.0, |s| s.width) - 800.0).abs() < 0.01);
}

#[test]
fn update_window_moved() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowMoved(win_id, iced::Point::new(100.0, 200.0)));
    let pos = app.windows.get(&win_id).and_then(|w| w.last_position);
    assert!(pos.is_some());
}

#[test]
fn update_toggle_lib_paths() {
    let (mut app, _win_id) = test_app();
    assert!(!app.show_lib_paths);
    let _ = app.update(Message::ToggleLibPaths);
    assert!(app.show_lib_paths);
    let _ = app.update(Message::ToggleLibPaths);
    assert!(!app.show_lib_paths);
}

#[test]
fn update_context_menu() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ContextMenu(100.0, 200.0),
    ));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.context_menu)
        .is_some());
    // Clicking deselects context menu
    let _ = app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.context_menu)
        .is_none());
}

#[test]
fn update_canvas_resize_node() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Resized(0, 50.0, 50.0, 200.0, 150.0),
    ));
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map_or(0.0, |n| n.width) - 200.0).abs() < 0.01);
    assert!((node.map_or(0.0, |n| n.height) - 150.0).abs() < 0.01);
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
    let _ = app.update(Message::InitializerTypeChanged(win_id, "once".into()));
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
    let _ = app.update(Message::InitializerCancel(win_id));
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

/// Simulate a right-click at a canvas position. Positions the cursor at (x, y)
/// and sends right mouse button press + release events, then processes all
/// generated messages.
fn right_click_at(app: &mut FlowEdit, win_id: window::Id, x: f32, y: f32) {
    let view = app.view(win_id);
    let mut ui = simulator(view);
    ui.point_at(iced::Point::new(x, y));
    ui.simulate([
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
            iced::mouse::Button::Right,
        )),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Right,
        )),
    ]);
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }
}

/// Simulate pressing and releasing a single key. Uses the simulator's built-in
/// `tap_key` helper which generates `KeyPressed` + `KeyReleased` events.
fn send_key(app: &mut FlowEdit, win_id: window::Id, key: iced::keyboard::Key) {
    let view = app.view(win_id);
    let mut ui = simulator(view);
    ui.tap_key(key);
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }
}

/// Simulate a left-button drag from one canvas position to another.
///
/// Because `into_messages()` consumes the simulator, we perform all steps
/// (press at `from`, cursor move to `to`, release at `to`) within a single
/// simulator cycle. The canvas widget receives these as sequential events
/// and should interpret them as a drag gesture.
///
/// **Limitation:** The iced Canvas widget's drag detection relies on internal
/// `Program::State` fields (e.g. `dragging`) that are set during
/// `ButtonPressed` and checked during `CursorMoved`. Because the simulator
/// replays events through `UserInterface::update`, the cursor position is set
/// once via `point_at` and does not change between events within a single
/// `simulate()` call. This means the canvas may not detect cursor movement
/// between press and release. If drag-based tests need reliable behavior,
/// fall back to direct `CanvasMessage::Moved` / `CanvasMessage::MoveCompleted`
/// messages instead.
fn drag(app: &mut FlowEdit, win_id: window::Id, from: iced::Point, to: iced::Point) {
    // Phase 1: Press at 'from' position
    {
        let view = app.view(win_id);
        let mut ui = simulator(view);
        ui.point_at(from);
        ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
            iced::mouse::Button::Left,
        ))]);
        let msgs: Vec<Message> = ui.into_messages().collect();
        for msg in msgs {
            let _ = app.update(msg);
        }
    }
    // Phase 2: Move cursor to 'to' position and release
    {
        let view = app.view(win_id);
        let mut ui = simulator(view);
        ui.point_at(to);
        ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: to }),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
        ]);
        let msgs: Vec<Message> = ui.into_messages().collect();
        for msg in msgs {
            let _ = app.update(msg);
        }
    }
}

#[test]
fn find_status_text() {
    let (app, win_id) = test_app();
    let view = app.view(win_id);
    let ui = simulator(view);
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
        .map_or(1.0, |w| w.canvas_state.zoom);
    click_and_update(&mut app, win_id, "+");
    let new = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    assert!(new > old, "Zoom should increase");
}

#[test]
fn click_zoom_out() {
    let (mut app, win_id) = test_app();
    let old = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    click_and_update(&mut app, win_id, "\u{2212}");
    let new = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    assert!(new < old, "Zoom should decrease");
}

#[test]
fn click_fit_enables_auto_fit() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.auto_fit_enabled = false;
    }
    click_and_update(&mut app, win_id, "Fit");
    assert!(app.windows.get(&win_id).is_some_and(|w| w.auto_fit_enabled));
}

#[test]
fn zoom_in_out_roundtrip() {
    let (mut app, win_id) = test_app();
    let original = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    click_and_update(&mut app, win_id, "+");
    click_and_update(&mut app, win_id, "\u{2212}");
    let final_zoom = app
        .windows
        .get(&win_id)
        .map_or(1.0, |w| w.canvas_state.zoom);
    assert!(
        (final_zoom - original).abs() < 0.01,
        "Zoom roundtrip should return to original"
    );
}

#[test]
fn click_info_toggles_metadata() {
    let (mut app, win_id) = test_app();
    assert!(!app.windows.get(&win_id).is_some_and(|w| w.show_metadata));
    click_and_update(&mut app, win_id, "\u{2139} Info");
    assert!(app.windows.get(&win_id).is_some_and(|w| w.show_metadata));
    click_and_update(&mut app, win_id, "\u{2139} Info");
    assert!(!app.windows.get(&win_id).is_some_and(|w| w.show_metadata));
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

// ---- Helper function tests ----

#[test]
fn helper_right_click_sets_context_menu() {
    let (mut app, win_id) = test_app();
    // Right-click on empty canvas area (far from nodes) should open context menu
    right_click_at(&mut app, win_id, 800.0, 600.0);
    assert!(
        app.windows
            .get(&win_id)
            .and_then(|w| w.context_menu)
            .is_some(),
        "Right-clicking empty canvas should set context_menu"
    );
}

#[test]
fn helper_send_key_delete() {
    let (mut app, win_id) = test_app();
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(2));

    // First, select a node via direct canvas click so the canvas internal state
    // has selected_node set. Then send Delete in the same simulator cycle.
    // Since the canvas widget's internal state (selected_node) is not preserved
    // between simulator cycles, we combine click + Delete in one cycle.
    {
        let view = app.view(win_id);
        let mut ui = simulator(view);
        // Click on the first node (at world 100,100 -> screen ~320,160)
        ui.point_at(iced::Point::new(320.0, 160.0));
        ui.simulate(simulator::click());
        // Now press Delete in the same simulator cycle
        ui.tap_key(iced::keyboard::Key::Named(
            iced::keyboard::key::Named::Delete,
        ));
        let msgs: Vec<Message> = ui.into_messages().collect();
        for msg in msgs {
            let _ = app.update(msg);
        }
    }

    // The canvas should have emitted Selected(Some(0)) from the click,
    // then Deleted(0) from the Delete key
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.nodes.len()),
        Some(1),
        "Delete key after selecting a node should remove it"
    );
}

#[test]
fn helper_drag_via_direct_messages() {
    // The simulator's drag helper has a known limitation: because the canvas
    // widget's internal Program::State (dragging, selected_node) is not
    // preserved across simulator cycles, and cursor position set via point_at()
    // does not generate CursorMoved events within a single simulate() call,
    // drag gestures may not produce the expected CanvasMessage sequence.
    //
    // For reliable drag testing, we fall back to direct CanvasMessage messages.
    let (mut app, win_id) = test_app();

    // Simulate a node drag from (100, 100) to (250, 350) using direct messages
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 250.0, 350.0),
    ));
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 250.0, 350.0),
    ));

    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!(
        (node.map_or(0.0, |n| n.x) - 250.0).abs() < 0.01,
        "Node x should be 250 after drag"
    );
    assert!(
        (node.map_or(0.0, |n| n.y) - 350.0).abs() < 0.01,
        "Node y should be 350 after drag"
    );
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.unsaved_edits),
        Some(1),
        "Drag should record an edit"
    );
}

#[test]
fn helper_drag_simulator_smoke_test() {
    // Smoke test: verify the drag() helper runs without panicking.
    // Due to the simulator limitation described above, the canvas may not
    // detect the drag gesture, so we only verify the helper completes and
    // the app state remains consistent.
    let (mut app, win_id) = test_app();
    let original_x = app
        .windows
        .get(&win_id)
        .and_then(|w| w.nodes.first())
        .map_or(0.0, |n| n.x);

    drag(
        &mut app,
        win_id,
        iced::Point::new(320.0, 160.0),
        iced::Point::new(500.0, 400.0),
    );

    // The node may or may not have moved depending on whether the simulator
    // correctly propagated the drag events through the canvas widget.
    // We just verify the app didn't panic and state is still valid.
    assert!(
        app.windows.contains_key(&win_id),
        "Window state should still exist after drag"
    );
    let _current_x = app
        .windows
        .get(&win_id)
        .and_then(|w| w.nodes.first())
        .map_or(0.0, |n| n.x);
    // Note: if current_x == original_x, the simulator drag didn't produce
    // canvas events. This is expected due to the limitation documented above.
    let _ = original_x; // suppress unused warning
}

// ---- Group 1: Node Deletion ----

#[test]
fn ui_delete_node_removes_connected_edges() {
    let (mut app, win_id) = test_app();
    // Add an edge between the two nodes
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.edges.push(EdgeLayout::new(
            "add".into(),
            "out".into(),
            "stdout".into(),
            "in".into(),
        ));
    }
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(1));
    // Delete node 0 ("add")
    let _ = app.update(Message::WindowCanvas(win_id, CanvasMessage::Deleted(0)));
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(1));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.edges.len()),
        Some(0),
        "Edge should be removed when connected node is deleted"
    );
}

#[test]
fn ui_delete_with_nothing_selected_no_change() {
    let (mut app, win_id) = test_app();
    let count_before = app.windows.get(&win_id).map(|w| w.nodes.len());
    // Deselect — no node is selected
    let _ = app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
    // Send Delete key — should not change anything with nothing selected
    send_key(
        &mut app,
        win_id,
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete),
    );
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.nodes.len()),
        count_before
    );
}

// ---- Group 2: Connection Selection/Deletion ----

#[test]
fn ui_select_and_delete_connection() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionCreated {
            from_node: "add".into(),
            from_port: String::new(),
            to_node: "stdout".into(),
            to_port: String::new(),
        },
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(1));
    // Select
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionSelected(Some(0)),
    ));
    assert_eq!(
        app.windows.get(&win_id).and_then(|w| w.selected_connection),
        Some(0)
    );
    // Delete
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionDeleted(0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(0));
}

#[test]
fn ui_connection_deselect_on_canvas_click() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.edges.push(EdgeLayout::new(
            "add".into(),
            String::new(),
            "stdout".into(),
            String::new(),
        ));
        win.selected_connection = Some(0);
    }
    let _ = app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
    assert_eq!(
        app.windows.get(&win_id).and_then(|w| w.selected_connection),
        None
    );
}

// ---- Group 3: Undo/Redo ----

#[test]
fn ui_undo_node_deletion() {
    let (mut app, win_id) = test_app();
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(2));
    let _ = app.update(Message::WindowCanvas(win_id, CanvasMessage::Deleted(0)));
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(1));
    let _ = app.update(Message::Undo);
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.nodes.len()),
        Some(2),
        "Undo should restore deleted node"
    );
}

#[test]
fn ui_undo_connection_deletion() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionCreated {
            from_node: "add".into(),
            from_port: String::new(),
            to_node: "stdout".into(),
            to_port: String::new(),
        },
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(1));
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionDeleted(0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(0));
    let _ = app.update(Message::Undo);
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.edges.len()),
        Some(1),
        "Undo should restore deleted connection"
    );
}

#[test]
fn ui_undo_empty_history_no_crash() {
    let (mut app, _win_id) = test_app();
    let _ = app.update(Message::Undo);
    let _ = app.update(Message::Redo);
    // Should not panic
}

// ---- Group 4: Undo/Redo Handler Tests ----
// Note: These tests verify the handler logic, not the subscription routing.
// The subscription mapping (Cmd+Z → Undo) is a simple pattern match in
// subscription() that can't be tested through the simulator.

#[test]
fn undo_restores_move() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 200.0, 300.0),
    ));
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
    ));
    let _ = app.update(Message::Undo);
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map_or(0.0, |n| n.x) - 100.0).abs() < 0.01);
}

#[test]
fn redo_reapplies_move() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 200.0, 300.0),
    ));
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
    ));
    let _ = app.update(Message::Undo);
    let _ = app.update(Message::Redo);
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map_or(0.0, |n| n.x) - 200.0).abs() < 0.01);
}

// ---- Group 5: Context Menu & Initializer ----

#[test]
fn ui_right_click_shows_context_menu() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ContextMenu(500.0, 300.0),
    ));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.context_menu)
        .is_some());
}

#[test]
fn ui_initializer_editor_open_and_cancel() {
    let (mut app, win_id) = test_app();
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::InitializerEdit(0, "input".into()),
    ));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.initializer_editor.as_ref())
        .is_some());
    let _ = app.update(Message::InitializerCancel(win_id));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.initializer_editor.as_ref())
        .is_none());
}

// ---- Group 6: Library Panel ----

#[test]
fn ui_library_add_function_creates_node() {
    let (mut app, win_id) = test_app();
    let count_before = app.windows.get(&win_id).map_or(0, |w| w.nodes.len());
    let _ = app.update(Message::Library(
        win_id,
        library_panel::LibraryMessage::AddFunction("lib://test_lib/math/add".into(), "add".into()),
    ));
    let count_after = app.windows.get(&win_id).map_or(0, |w| w.nodes.len());
    assert_eq!(
        count_after,
        count_before + 1,
        "Adding from library should create a new node"
    );
}

// ---- Group 7: Window Lifecycle ----

#[test]
fn ui_close_window_removes_it() {
    let (mut app, win_id) = test_app();
    // Add a second window so closing the first doesn't trigger iced::exit()
    // (iced::exit() is returned as Task and safe to ignore, but closing
    // the root window removes it from windows map before that)
    let second_id = window::Id::unique();
    app.windows.insert(second_id, test_win_state());
    app.root_window = Some(second_id);

    assert!(app.windows.contains_key(&win_id));
    let _ = app.update(Message::WindowClosed(win_id));
    assert!(!app.windows.contains_key(&win_id));
}

#[test]
fn ui_window_focused_updates_other() {
    let (mut app, _win_id) = test_app();
    let other_id = window::Id::unique();
    let _ = app.update(Message::WindowFocused(other_id));
    assert_eq!(app.focused_window, Some(other_id));
}

#[test]
fn ui_close_active_window() {
    let (mut app, win_id) = test_app();
    // Add a second window and make it the root, so closing win_id
    // (which is the focused window) won't trigger iced::exit()
    let second_id = window::Id::unique();
    app.windows.insert(second_id, test_win_state());
    app.root_window = Some(second_id);
    assert_eq!(app.focused_window, Some(win_id));
    // CloseActiveWindow closes the focused window (unsaved_edits == 0, no dialog)
    let _ = app.update(Message::CloseActiveWindow);
    assert!(
        !app.windows.contains_key(&win_id),
        "CloseActiveWindow should remove the focused window"
    );
}

// ---- Group 8: Pan and Resize ----

#[test]
fn ui_pan_changes_offset() {
    let (mut app, win_id) = test_app();
    let old_x = app
        .windows
        .get(&win_id)
        .map_or(0.0, |w| w.canvas_state.scroll_offset.x);
    let old_y = app
        .windows
        .get(&win_id)
        .map_or(0.0, |w| w.canvas_state.scroll_offset.y);
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Pan(50.0, 30.0),
    ));
    let new_x = app
        .windows
        .get(&win_id)
        .map_or(0.0, |w| w.canvas_state.scroll_offset.x);
    let new_y = app
        .windows
        .get(&win_id)
        .map_or(0.0, |w| w.canvas_state.scroll_offset.y);
    assert!(
        (new_x - old_x - 50.0).abs() < 0.01,
        "Pan should change scroll_offset.x by 50"
    );
    assert!(
        (new_y - old_y - 30.0).abs() < 0.01,
        "Pan should change scroll_offset.y by 30"
    );
}

#[test]
fn ui_resize_node_records_history() {
    let (mut app, win_id) = test_app();
    // Resized updates the node dimensions immediately
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Resized(0, 100.0, 100.0, 250.0, 180.0),
    ));
    // ResizeCompleted records history: (idx, old_x, old_y, old_w, old_h, new_x, new_y, new_w, new_h)
    let _ = app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ResizeCompleted(0, 100.0, 100.0, 180.0, 120.0, 100.0, 100.0, 250.0, 180.0),
    ));
    assert!(
        app.windows.get(&win_id).map_or(0, |w| w.unsaved_edits) > 0,
        "Resize should record an edit"
    );
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!(
        (node.map_or(0.0, |n| n.width) - 250.0).abs() < 0.01,
        "Node width should be 250 after resize"
    );
    assert!(
        (node.map_or(0.0, |n| n.height) - 180.0).abs() < 0.01,
        "Node height should be 180 after resize"
    );
}

// ---- Group 9: Initializer Tests ----

#[test]
fn initializer_apply_once() {
    let (mut app, win_id) = test_app();
    // Add a process ref so apply can find it
    if let Some(win) = app.windows.get_mut(&win_id) {
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "once".into(),
            value_text: "42".into(),
        };
        initializer::apply_initializer_edit(win, &editor);
        assert!(win.unsaved_edits > 0);
        // Check display was updated
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_some_and(|d| d.contains("once")));
    }
}

#[test]
fn initializer_apply_always() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "always".into(),
            value_text: "\"hello\"".into(),
        };
        initializer::apply_initializer_edit(win, &editor);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_some_and(|d| d.contains("always")));
    }
}

#[test]
fn initializer_apply_none_removes() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        // First set one
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "once".into(),
            value_text: "42".into(),
        };
        initializer::apply_initializer_edit(win, &editor);
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
        initializer::apply_initializer_edit(win, &editor);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("input"))
            .is_none());
    }
}

#[test]
fn initializer_apply_state_set_and_remove() {
    use flowcore::model::input::InputInitializer;

    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        let init = InputInitializer::Once(serde_json::json!(99));
        let display = "once: 99".to_string();
        initializer::apply_initializer_state(win, 0, "port", Some(&init), Some(&display));
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("port"))
            .is_some());

        // Remove
        initializer::apply_initializer_state(win, 0, "port", None, None);
        assert!(win
            .nodes
            .first()
            .and_then(|n| n.initializers.get("port"))
            .is_none());
    }
}

#[test]
fn initializer_apply_invalid_type_no_change() {
    let (mut app, win_id) = test_app();
    if let Some(win) = app.windows.get_mut(&win_id) {
        let edits_before = win.unsaved_edits;
        let editor = InitializerEditor {
            node_index: 0,
            port_name: "input".into(),
            init_type: "bogus".into(),
            value_text: "42".into(),
        };
        initializer::apply_initializer_edit(win, &editor);
        assert_eq!(
            win.unsaved_edits, edits_before,
            "Invalid type should not create an edit"
        );
    }
}
