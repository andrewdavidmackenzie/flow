# flowedit: Automated UI Testing for Interactive Features (Issue #2580)

## Overview

Add comprehensive automated UI tests for flowedit's interactive features
using iced_test 0.14.0's headless simulator. Tests cover node operations,
connection management, undo/redo, keyboard shortcuts, panel interactions,
and canvas controls.

## Approach

All tests go through the iced_test simulator — no direct `app.update()`
calls except where the simulator truly can't reach (file dialogs, window
creation). Build reusable test helpers for common interaction patterns
(drag, right-click, keyboard shortcuts). Maximize reuse of flowcore/flowclib
types when constructing test state.

## Test File Structure

Move all test code out of `main.rs` into a dedicated test module:

```
flowedit/src/
  main.rs          — add `#[cfg(test)] mod ui_test;`
  ui_test.rs       — all test helpers, test setup, and UI/unit tests
```

The existing `#[cfg(test)] mod test { ... mod ui { ... } }` block in
main.rs (~600 lines) moves entirely to `ui_test.rs`. This includes test
helpers (`test_app`, `test_win_state`, `test_node`, `temp_dir`,
`click_and_update`, `canvas_click_and_update`), all existing unit tests,
and all existing iced_test UI tests. New tests are added to this file.

## Test Helpers

### Interaction Helpers

Add to `flowedit/src/ui_test.rs`:

- `drag(app, win_id, from, to)` — left-button drag between two screen
  positions. Sequences: point_at(from) + ButtonPressed(Left) + multiple
  CursorMoved steps to `to` + ButtonReleased(Left). Feeds all generated
  messages through `app.update()`.

- `right_click(app, win_id, x, y)` — right-click at position. Sequences:
  point_at + ButtonPressed(Right) + ButtonReleased(Right).

- `key_press(app, win_id, key)` — single key press (e.g., Delete,
  Backspace). Constructs KeyPressed + KeyReleased events.

- `cmd_key(app, win_id, key)` — Cmd+key shortcut. Constructs KeyPressed
  with `keyboard::Modifiers::COMMAND` set.

- `middle_drag(app, win_id, from, to)` — middle-button drag for panning.
  Same pattern as `drag` but with `Button::Middle`.

### Flexible Test Setup

Refactor `test_app()` to accept a `FlowDefinition` parameter. Each test
constructs exactly the flow it needs — simple tests pass
`FlowDefinition::default()`, complex tests build one with nodes, ports,
and connections using flowcore types directly (`ProcessReference`,
`Connection`, `IO::new_named()`, `FunctionDefinition::default()`, etc.).

The app state (nodes, edges, ports) is derived from the provided
`FlowDefinition` the same way `load_flow()` does it — no parallel
construction of UI state.

## Test Groups

### Group 1: Node Deletion (Priority 1)

- Select node via canvas click, press Delete, verify node removed from
  `win.nodes` and `flow_definition.process_refs`
- Verify edges connected to deleted node are also removed
- Verify `unsaved_edits` incremented
- Delete with nothing selected — verify no change

### Group 2: Connection Creation/Selection/Deletion (Priority 2)

- Drag from output port position to input port position, verify edge
  created in `win.edges`
- Click on connection midpoint, verify `selected_connection` set
- Select connection, press Delete, verify connection removed
- Verify `flow_definition.connections` updated on creation/deletion

### Group 3: Undo/Redo (Priority 3)

- Delete node, Undo, verify node restored with original position/ports
- Delete connection, Undo, verify connection restored
- Move node (drag), Undo, verify node back at original position
- Undo then Redo, verify state matches post-edit state
- Undo with empty history — verify no crash

### Group 4: Node Moving/Resizing

- Drag node center, verify position updated in `win.nodes`
- Drag near node edge, verify size updated (width/height changed)
- Verify `MoveCompleted` / `ResizeCompleted` recorded in history

### Group 5: Keyboard Shortcuts

- Delete key deletes selected node
- Delete key deletes selected connection
- Cmd+Z triggers Undo
- Cmd+Shift+Z triggers Redo

### Group 6: Context Menu

- Right-click on empty canvas, verify `context_menu` position set
- Right-click on a node — verify selection (not context menu)

### Group 7: Initializer Editing

- Right-click on input port position, verify `InitializerEdit` message
  generated with correct node index and port name
- Send `InitializerApply` with value, verify initializer stored on node

### Group 8: Library Panel

- Click function name in library panel, verify node added to canvas
  (via `LibraryAction::Add`)

### Group 9: Hierarchy Panel

- Toggle expand/collapse on hierarchy entries, verify state change

### Group 10: Pan Controls

- Middle-mouse drag, verify canvas offset changes via `Pan` message

### Group 11: Window Creation

`window::open()` returns a Task that won't execute headlessly, but the
app inserts the new `WindowState` into `self.windows` immediately. Tests
can verify the window state is constructed correctly.

- Open sub-flow node (pencil icon on flow node) — verify new window
  entry in `self.windows` with correct `WindowKind::FlowEditor`, flow
  name, nodes, edges
- Open function viewer (pencil icon on function node) — verify new
  window entry with `WindowKind::FunctionViewer`, correct name, ports,
  description, read_only flag
- Note: requires test flow files on disk for `load_flow()` / parser

### Group 12: Window Lifecycle

`CloseRequested`, `WindowClosed`, `WindowFocused`, `CloseActiveWindow`,
`QuitAll` are all regular `Message` variants that can be constructed and
sent directly.

- `CloseRequested` with no unsaved edits — verify window removed from
  `self.windows`
- `CloseRequested` with unsaved edits — dialog blocks (can't test dialog
  itself), but can test with `unsaved_edits = 0` to verify removal path
- `WindowFocused` — verify `focused_window` updated
- `CloseActiveWindow` — verify focused window removed
- `QuitAll` — verify all windows removed (or app signals exit)

## Not Testable Headlessly (Skipped)

- File dialogs (Open, Save As, + Library browse) — native OS widgets
  via `rfd`, runs in-process on macOS
- Scroll wheel zoom — iced_test has no scroll event constructor
- Unsaved changes dialog prompt — `rfd::MessageDialog` blocks in-process

## Key Files

| File | Role |
|------|------|
| `flowedit/src/ui_test.rs` | All test helpers, setup, and UI/unit tests (new) |
| `flowedit/src/main.rs` | `#[cfg(test)] mod ui_test;` declaration |
| `flowedit/src/canvas_view.rs` | Canvas event handling, hit testing |
| `flowedit/src/undo_redo.rs` | History management |
| `flowedit/src/library_panel.rs` | Library panel messages |
| `flowedit/src/hierarchy_panel.rs` | Hierarchy panel messages |
| `flowedit/src/initializer.rs` | Initializer editing |
