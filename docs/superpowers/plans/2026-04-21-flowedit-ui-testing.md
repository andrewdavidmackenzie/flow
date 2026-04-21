# flowedit: Automated UI Testing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add comprehensive automated UI tests for flowedit's interactive features using iced_test's headless simulator, covering node operations, connections, undo/redo, keyboard shortcuts, panel interactions, and window lifecycle.

**Architecture:** Move all existing tests from main.rs to a dedicated `ui_test.rs` module. Refactor `test_app()` to accept a `FlowDefinition` for flexible test setup. Add reusable simulator helpers for drag, right-click, keyboard shortcuts. Add ~30 new tests covering the remaining interactive features.

**Tech Stack:** Rust, iced 0.14.0, iced_test 0.14.0, flowcore model types

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `flowedit/src/ui_test.rs` | Create | All test helpers, setup functions, and tests |
| `flowedit/src/main.rs` | Modify | Remove `mod test` block, add `#[cfg(test)] mod ui_test;` |

---

### Task 1: Move existing tests to `ui_test.rs`

**Files:**
- Create: `flowedit/src/ui_test.rs`
- Modify: `flowedit/src/main.rs`

- [ ] **Step 1: Create `ui_test.rs` with all existing test code**

Create `flowedit/src/ui_test.rs`. Copy the entire contents of the `#[cfg(test)] mod test { ... }` block from `main.rs` (lines 2839-3601), but as a flat module (no wrapping `mod test`). The file-level `#[cfg(test)]` comes from the `mod` declaration in main.rs.

The file should start with:
```rust
#![allow(clippy::indexing_slicing)]

use super::*;
use std::collections::HashMap;
use iced_test::simulator::{self, simulator};
```

Merge the inner `mod ui` contents into the top level — no nested module needed since the whole file is tests. All helper functions (`test_node`, `test_win_state`, `test_app`, `temp_dir`, `click_and_update`, `canvas_click_and_update`) and all test functions go at the top level.

- [ ] **Step 2: Add module declaration in `main.rs`**

Replace the entire `#[cfg(test)] #[allow(clippy::indexing_slicing)] mod test { ... }` block (lines 2839-3601) with:

```rust
#[cfg(test)]
mod ui_test;
```

- [ ] **Step 3: Verify all tests pass**

Run: `cargo test -p flowedit`
Expected: All 140 tests pass (same count as before)

- [ ] **Step 4: Run clippy and fmt**

Run: `make clippy && cargo fmt`

- [ ] **Step 5: Commit**

```bash
git add flowedit/src/ui_test.rs flowedit/src/main.rs
git commit -m "flowedit: Move all tests to dedicated ui_test.rs module (#2580)"
```

---

### Task 2: Add simulator interaction helpers

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Add `drag` helper**

```rust
fn drag(app: &mut FlowEdit, win_id: window::Id, from: iced::Point, to: iced::Point) {
    let view = app.view(win_id);
    let mut ui = simulator(view);
    ui.point_at(from);
    ui.simulate(simulator::click());
    // Collect press messages
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }

    // Move to destination
    let view = app.view(win_id);
    let mut ui = simulator(view);
    ui.point_at(to);
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }

    // Release
    let view = app.view(win_id);
    let mut ui = simulator(view);
    ui.point_at(to);
    ui.simulate(vec![
        iced_test::simulator::Interaction::Mouse(iced_test::simulator::Mouse::Release(
            iced::mouse::Button::Left,
        )),
    ]);
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }
}
```

Note: The exact event sequence may need adjustment based on how iced_test routes events to the Canvas Program. The Canvas widget receives raw iced::Event and tracks its own drag state internally. If the simulator approach doesn't generate the right canvas events, fall back to direct message construction for drag operations.

- [ ] **Step 2: Add `right_click_at` helper**

```rust
fn right_click_at(app: &mut FlowEdit, win_id: window::Id, x: f32, y: f32) {
    let view = app.view(win_id);
    let mut ui = simulator(view);
    ui.point_at(iced::Point::new(x, y));
    ui.simulate(vec![
        iced_test::simulator::Interaction::Mouse(iced_test::simulator::Mouse::Press(
            iced::mouse::Button::Right,
        )),
        iced_test::simulator::Interaction::Mouse(iced_test::simulator::Mouse::Release(
            iced::mouse::Button::Right,
        )),
    ]);
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }
}
```

- [ ] **Step 3: Add `send_key` helper**

```rust
fn send_key(app: &mut FlowEdit, win_id: window::Id, key: iced::keyboard::Key) {
    let view = app.view(win_id);
    let mut ui = simulator(view);
    ui.simulate(vec![
        iced_test::simulator::Interaction::Keyboard(
            iced_test::simulator::Keyboard::PressKey {
                key: key.clone(),
                text: None,
            },
        ),
        iced_test::simulator::Interaction::Keyboard(
            iced_test::simulator::Keyboard::ReleaseKey { key },
        ),
    ]);
    let msgs: Vec<Message> = ui.into_messages().collect();
    for msg in msgs {
        let _ = app.update(msg);
    }
}
```

- [ ] **Step 4: Write a simple test using each helper to verify they work**

```rust
#[test]
fn helper_right_click_sets_context_menu() {
    let (mut app, win_id) = test_app();
    right_click_at(&mut app, win_id, 600.0, 400.0);
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.context_menu)
        .is_some());
}
```

- [ ] **Step 5: Verify tests pass**

Run: `cargo test -p flowedit`
Expected: PASS

- [ ] **Step 6: Run clippy and fmt**

Run: `make clippy && cargo fmt`

- [ ] **Step 7: Commit**

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Add simulator interaction helpers (drag, right_click, send_key) (#2580)"
```

---

### Task 3: Refactor `test_app` to accept `FlowDefinition`

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Refactor `test_app` to accept a `FlowDefinition`**

Change `test_app()` to `test_app_with_flow(flow: FlowDefinition)`. Build nodes/edges from the flow definition the same way `load_flow()` does. Keep a no-arg `test_app()` that calls `test_app_with_flow(FlowDefinition::default())` with the existing test nodes added to the flow's process_refs.

```rust
fn test_app() -> (FlowEdit, window::Id) {
    let mut flow = FlowDefinition::default();
    flow.name = "test".into();
    flow.process_refs.push(ProcessReference {
        alias: "add".into(),
        source: "lib://flowstdlib/math/add".into(),
        initializations: std::collections::BTreeMap::new(),
        x: Some(100.0),
        y: Some(100.0),
        width: Some(180.0),
        height: Some(120.0),
    });
    flow.process_refs.push(ProcessReference {
        alias: "stdout".into(),
        source: "context://stdio/stdout".into(),
        initializations: std::collections::BTreeMap::new(),
        x: Some(400.0),
        y: Some(100.0),
        width: Some(180.0),
        height: Some(120.0),
    });
    test_app_with_flow(flow)
}

fn test_app_with_flow(flow: FlowDefinition) -> (FlowEdit, window::Id) {
    let win_id = window::Id::unique();

    // Build nodes from process_refs (same logic as load_flow)
    let nodes: Vec<NodeLayout> = flow
        .process_refs
        .iter()
        .map(|pref| NodeLayout {
            alias: pref.alias.clone(),
            source: pref.source.clone(),
            description: String::new(),
            x: pref.x.unwrap_or(0.0),
            y: pref.y.unwrap_or(0.0),
            width: pref.width.unwrap_or(180.0),
            height: pref.height.unwrap_or(120.0),
            inputs: Vec::new(),
            outputs: Vec::new(),
            initializers: HashMap::new(),
        })
        .collect();

    // Build edges from flow connections
    let edges: Vec<EdgeLayout> = flow
        .connections
        .iter()
        .filter_map(|conn| {
            // Extract from/to info from Connection
            // This depends on Connection's structure
            None // Placeholder — implement based on Connection API
        })
        .collect();

    let win = WindowState {
        kind: WindowKind::FlowEditor,
        flow_name: flow.name.clone(),
        nodes,
        edges,
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
        flow_definition: flow,
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

    let app = FlowEdit {
        windows: HashMap::from([(win_id, win)]),
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
```

Note: The edge construction from `flow.connections` depends on `Connection`'s API. Check `flowcore::model::connection::Connection` for the `from`/`to` field structure and implement accordingly. If `Connection` construction is complex, edges can be added manually in tests that need them.

- [ ] **Step 2: Update all existing test call sites**

All tests that called `test_app()` should still work since the no-arg version is preserved. Verify no breakage.

- [ ] **Step 3: Verify tests pass**

Run: `cargo test -p flowedit`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Refactor test_app to accept FlowDefinition (#2580)"
```

---

### Task 4: Node deletion tests via simulator

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Add test — select node and delete via Delete key**

```rust
#[test]
fn ui_select_and_delete_node() {
    let (mut app, win_id) = test_app();
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(2));

    // Select node 0 via canvas click
    canvas_click_and_update(&mut app, win_id, 320.0, 160.0);
    assert_eq!(
        app.windows.get(&win_id).and_then(|w| w.selected_node),
        Some(0)
    );

    // Press Delete key
    send_key(&mut app, win_id, iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete));

    // Verify node removed
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(1));
    assert_eq!(app.windows.get(&win_id).map(|w| w.unsaved_edits), Some(1));
}
```

Note: If the Delete key doesn't propagate through the simulator to the canvas or subscription, fall back to direct message:
```rust
app.update(Message::WindowCanvas(win_id, CanvasMessage::Deleted(0)));
```
The important thing is to verify the delete behavior. Try the simulator approach first; if it doesn't work, document why and use the direct message approach.

- [ ] **Step 2: Add test — delete with nothing selected**

```rust
#[test]
fn ui_delete_with_nothing_selected() {
    let (mut app, win_id) = test_app();
    let count_before = app.windows.get(&win_id).map(|w| w.nodes.len());

    send_key(&mut app, win_id, iced::keyboard::Key::Named(iced::keyboard::key::Named::Delete));

    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), count_before);
}
```

- [ ] **Step 3: Add test — delete node removes connected edges**

```rust
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
    app.update(Message::WindowCanvas(win_id, CanvasMessage::Deleted(0)));

    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(1));
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.edges.len()),
        Some(0),
        "Edge should be removed when connected node is deleted"
    );
}
```

- [ ] **Step 4: Verify tests pass**

Run: `cargo test -p flowedit`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Add node deletion UI tests (#2580)"
```

---

### Task 5: Connection tests

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Add test — select and delete connection**

```rust
#[test]
fn ui_select_and_delete_connection() {
    let (mut app, win_id) = test_app();

    // Add a connection
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

    // Select the connection
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionSelected(Some(0)),
    ));
    assert_eq!(
        app.windows.get(&win_id).and_then(|w| w.selected_connection),
        Some(0)
    );

    // Delete via message
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionDeleted(0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(0));
    assert!(app.windows.get(&win_id).map(|w| w.unsaved_edits).unwrap_or(0) > 0);
}
```

- [ ] **Step 2: Add test — connection deselect**

```rust
#[test]
fn ui_connection_deselect() {
    let (mut app, win_id) = test_app();

    // Add and select a connection
    if let Some(win) = app.windows.get_mut(&win_id) {
        win.edges.push(EdgeLayout::new(
            "add".into(), "".into(), "stdout".into(), "".into(),
        ));
        win.selected_connection = Some(0);
    }

    // Click empty canvas should deselect connection
    app.update(Message::WindowCanvas(win_id, CanvasMessage::Selected(None)));
    assert_eq!(
        app.windows.get(&win_id).and_then(|w| w.selected_connection),
        None
    );
}
```

- [ ] **Step 3: Verify and commit**

Run: `cargo test -p flowedit`

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Add connection UI tests (#2580)"
```

---

### Task 6: Undo/Redo tests via simulator

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Add test — undo node deletion restores node**

```rust
#[test]
fn ui_undo_node_deletion() {
    let (mut app, win_id) = test_app();
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(2));

    // Delete node
    app.update(Message::WindowCanvas(win_id, CanvasMessage::Deleted(0)));
    assert_eq!(app.windows.get(&win_id).map(|w| w.nodes.len()), Some(1));

    // Undo
    app.update(Message::Undo);
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.nodes.len()),
        Some(2),
        "Undo should restore deleted node"
    );
}
```

- [ ] **Step 2: Add test — undo connection deletion**

```rust
#[test]
fn ui_undo_connection_deletion() {
    let (mut app, win_id) = test_app();

    // Create connection
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

    // Delete it
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ConnectionDeleted(0),
    ));
    assert_eq!(app.windows.get(&win_id).map(|w| w.edges.len()), Some(0));

    // Undo
    app.update(Message::Undo);
    assert_eq!(
        app.windows.get(&win_id).map(|w| w.edges.len()),
        Some(1),
        "Undo should restore deleted connection"
    );
}
```

- [ ] **Step 3: Add test — undo with empty history is safe**

```rust
#[test]
fn ui_undo_empty_history_no_crash() {
    let (mut app, _win_id) = test_app();
    // Should not panic
    app.update(Message::Undo);
    app.update(Message::Redo);
}
```

- [ ] **Step 4: Verify and commit**

Run: `cargo test -p flowedit`

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Add undo/redo UI tests (#2580)"
```

---

### Task 7: Keyboard shortcut tests

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Add test — Cmd+Z triggers Undo**

Test by setting up a state change, then using the keyboard shortcut to undo it. Since keyboard shortcuts go through `subscription()` which uses `keyboard::listen()`, and iced_test may not process subscriptions, fall back to testing via direct message construction. The subscription mapping is: Cmd+Z → `Message::Undo`.

```rust
#[test]
fn shortcut_undo() {
    let (mut app, win_id) = test_app();

    // Make a change
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 200.0, 300.0),
    ));
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
    ));

    // Simulate Cmd+Z → Undo
    app.update(Message::Undo);

    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map(|n| n.x).unwrap_or(0.0) - 100.0).abs() < 0.01);
}
```

- [ ] **Step 2: Add test — Cmd+Shift+Z triggers Redo**

```rust
#[test]
fn shortcut_redo() {
    let (mut app, win_id) = test_app();

    // Make a change, undo, then redo
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Moved(0, 200.0, 300.0),
    ));
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::MoveCompleted(0, 100.0, 100.0, 200.0, 300.0),
    ));
    app.update(Message::Undo);
    app.update(Message::Redo);

    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map(|n| n.x).unwrap_or(0.0) - 200.0).abs() < 0.01);
}
```

- [ ] **Step 3: Verify and commit**

Run: `cargo test -p flowedit`

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Add keyboard shortcut tests (#2580)"
```

---

### Task 8: Context menu, initializer, and library panel tests

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Add test — right-click on empty canvas shows context menu**

```rust
#[test]
fn ui_right_click_shows_context_menu() {
    let (mut app, win_id) = test_app();
    // Use direct message since right-click through simulator may not reach canvas
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ContextMenu(500.0, 300.0),
    ));
    let menu = app.windows.get(&win_id).and_then(|w| w.context_menu);
    assert!(menu.is_some(), "Context menu should be set");
}
```

- [ ] **Step 2: Add test — initializer editor open and cancel**

```rust
#[test]
fn ui_initializer_editor_open_and_cancel() {
    let (mut app, win_id) = test_app();
    // Open initializer editor via canvas message
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::InitializerEdit(0, "input".into()),
    ));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.initializer_editor.as_ref())
        .is_some());

    // Cancel
    app.update(Message::InitializerCancel(win_id));
    assert!(app
        .windows
        .get(&win_id)
        .and_then(|w| w.initializer_editor.as_ref())
        .is_none());
}
```

- [ ] **Step 3: Add test — library panel add function via click**

```rust
#[test]
fn ui_library_click_adds_node() {
    let (mut app, win_id) = test_app();
    let count_before = app.windows.get(&win_id).map(|w| w.nodes.len()).unwrap_or(0);

    // Simulate library panel action directly (clicking function button in library)
    app.update(Message::Library(
        win_id,
        library_panel::LibraryMessage::AddFunction(
            "lib://test_lib/math/add".into(),
            "add".into(),
        ),
    ));

    let count_after = app.windows.get(&win_id).map(|w| w.nodes.len()).unwrap_or(0);
    assert_eq!(
        count_after,
        count_before + 1,
        "Adding from library should create a new node"
    );
}
```

- [ ] **Step 4: Verify and commit**

Run: `cargo test -p flowedit`

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Add context menu, initializer, and library panel tests (#2580)"
```

---

### Task 9: Window lifecycle tests

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Add test — close window with no unsaved edits**

```rust
#[test]
fn ui_close_window_no_unsaved() {
    let (mut app, win_id) = test_app();
    assert!(app.windows.contains_key(&win_id));

    // Close request with no unsaved edits should remove window
    app.update(Message::CloseRequested(win_id));
    // The window should request close (may need WindowClosed to finalize)
    app.update(Message::WindowClosed(win_id));
    assert!(
        !app.windows.contains_key(&win_id),
        "Window should be removed after close"
    );
}
```

- [ ] **Step 2: Add test — WindowFocused updates focused_window**

```rust
#[test]
fn ui_window_focused_updates_state() {
    let (mut app, _win_id) = test_app();
    let other_id = window::Id::unique();
    app.update(Message::WindowFocused(other_id));
    assert_eq!(app.focused_window, Some(other_id));
}
```

- [ ] **Step 3: Add test — close active window**

```rust
#[test]
fn ui_close_active_window() {
    let (mut app, win_id) = test_app();
    assert!(app.windows.contains_key(&win_id));
    assert_eq!(app.focused_window, Some(win_id));

    app.update(Message::CloseActiveWindow);
    // Should trigger close on the focused window
    app.update(Message::WindowClosed(win_id));
    assert!(!app.windows.contains_key(&win_id));
}
```

- [ ] **Step 4: Verify and commit**

Run: `cargo test -p flowedit`

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Add window lifecycle tests (#2580)"
```

---

### Task 10: Pan controls and node resize tests

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Add test — pan via direct message**

```rust
#[test]
fn ui_pan_canvas() {
    let (mut app, win_id) = test_app();
    let offset_before = app
        .windows
        .get(&win_id)
        .map(|w| (w.canvas_state.scroll_offset_x, w.canvas_state.scroll_offset_y))
        .unwrap_or((0.0, 0.0));

    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Pan(50.0, 30.0),
    ));

    let offset_after = app
        .windows
        .get(&win_id)
        .map(|w| (w.canvas_state.scroll_offset_x, w.canvas_state.scroll_offset_y))
        .unwrap_or((0.0, 0.0));

    assert!(
        (offset_after.0 - offset_before.0 - 50.0).abs() < 0.01
            || (offset_after.1 - offset_before.1 - 30.0).abs() < 0.01,
        "Pan should change canvas offset"
    );
}
```

Note: Check the actual field names on `FlowCanvasState` for scroll offset — they may be named differently (e.g., `offset_x`, `offset_y`, or stored in a `Point`). Adjust the field access accordingly.

- [ ] **Step 2: Add test — node resize records history**

```rust
#[test]
fn ui_resize_node_records_history() {
    let (mut app, win_id) = test_app();
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::Resized(0, 100.0, 100.0, 250.0, 180.0),
    ));
    app.update(Message::WindowCanvas(
        win_id,
        CanvasMessage::ResizeCompleted(0, 180.0, 120.0, 250.0, 180.0),
    ));
    assert!(app.windows.get(&win_id).map(|w| w.unsaved_edits).unwrap_or(0) > 0);

    // Verify size changed
    let node = app.windows.get(&win_id).and_then(|w| w.nodes.first());
    assert!((node.map(|n| n.width).unwrap_or(0.0) - 250.0).abs() < 0.01);
    assert!((node.map(|n| n.height).unwrap_or(0.0) - 180.0).abs() < 0.01);
}
```

- [ ] **Step 3: Verify and commit**

Run: `cargo test -p flowedit`

```bash
git add flowedit/src/ui_test.rs
git commit -m "flowedit: Add pan and resize tests (#2580)"
```

---

### Task 11: Final verification

- [ ] **Step 1: Run full test suite**

Run: `make test`
Expected: All tests pass

- [ ] **Step 2: Run clippy and fmt**

Run: `make clippy && cargo fmt`
Expected: Clean

- [ ] **Step 3: Verify main.rs no longer has test code**

Run: `grep -n "#\[cfg(test)\]" flowedit/src/main.rs`
Expected: Single line with `mod ui_test;`

- [ ] **Step 4: Count total tests**

Run: `cargo test -p flowedit 2>&1 | grep "test result"`
Expected: ~155+ tests (140 existing + ~15 new)

- [ ] **Step 5: Commit any final adjustments**

```bash
git add -A
git commit -m "flowedit: Final UI test adjustments (#2580)"
```
