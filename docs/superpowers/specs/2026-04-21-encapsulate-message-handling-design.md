# flowedit: Encapsulate Message Handling (Issue #2593, Sub-project 1)

## Overview

Move message handling from `FlowEdit::update()` in main.rs into each
module's own handler function. main.rs becomes a thin router that
dispatches messages to the appropriate module and handles cross-module
coordination.

This is the first of 5 sub-projects for issue #2593.

## Pattern

Each module gets a handler function:

```text
pub(crate) fn handle_<module>_message(
    win: &mut WindowState,
    msg: <ModuleMessage>,
) -> <ModuleAction>
```

The action enum captures cross-module effects that main.rs needs to
handle (e.g., opening a new window). Most messages return `Action::None`
(fully handled internally).

main.rs dispatches:

```text
Message::WindowCanvas(win_id, msg) => {
    if let Some(win) = self.windows.get_mut(&win_id) {
        match canvas_view::handle_canvas_message(win, msg) {
            CanvasAction::OpenNode(idx) => self.open_node(win_id, idx),
            CanvasAction::None => {}
        }
    }
}
```

## Modules to Extract (in order)

### 1. canvas_view.rs

Move handling of:
- All `CanvasMessage` variants: `Selected`, `ConnectionSelected`,
  `Moved`, `MoveCompleted`, `Resized`, `ResizeCompleted`, `Deleted`,
  `ConnectionCreated`, `ConnectionDeleted`, `InitializerEdit`,
  `OpenNode`, `Pan`, `ZoomBy`, `AutoFitViewport`, `HoverChanged`,
  `ContextMenu`
- `ZoomIn(win_id)`, `ZoomOut(win_id)`, `ToggleAutoFit(win_id)`

Action enum:
- `None` — fully handled
- `OpenNode(usize)` — main.rs opens node in new window
- `InitializerEdit(usize, String)` — main.rs opens initializer editor

### 2. hierarchy_panel.rs

Move handling of:
- `HierarchyMessage` dispatch (currently main.rs matches the result
  of `hierarchy.update()` and acts on `Open`)

Action enum:
- `None`
- `OpenPath(String, PathBuf)` — main.rs opens the path

### 3. library_panel.rs

Move handling of:
- `LibraryMessage` dispatch and `LibraryAction` matching
- `AddFunction`, `ViewFunction`, `AddLibrary` actions

Action enum:
- `None`
- `AddFunction(String, String)` — main.rs adds node to canvas
- `ViewFunction(String, String)` — main.rs opens viewer
- `AddLibrary` — main.rs opens file dialog and loads library

### 4. undo_redo.rs

Move handling of:
- `Message::Undo` and `Message::Redo`

No action enum needed — undo/redo is fully self-contained (operates
on `WindowState` only).

### 5. flow_io.rs

Move handling of:
- `Message::Save`, `Message::SaveAs`, `Message::New`, `Message::Compile`

Action enum:
- `None`
- `FlowLoaded(LoadedFlow)` — after Open, main.rs rebuilds library cache

Note: `Message::Open` triggers a file dialog then updates state — the
handler can do both since `perform_open` already exists.

### 6. initializer.rs

Move handling of:
- `InitializerTypeChanged`, `InitializerValueChanged`,
  `InitializerApply`, `InitializerCancel`

No action enum needed — operates on `WindowState.initializer_editor`.

## What Stays in main.rs

- Window lifecycle: `CloseRequested`, `WindowClosed`, `WindowFocused`,
  `QuitAll`, `CloseActiveWindow`
- Window events: `WindowResized`, `WindowMoved`
- `NewSubFlow`, `NewFunction` — complex window creation
- Function viewer messages: `FunctionSave`, `FunctionTabSelected`,
  `FunctionBrowseSource`, `FunctionNameChanged`,
  `FunctionDescriptionChanged`, `FunctionAdd/Delete Input/Output`,
  `FunctionInput/OutputName/TypeChanged`
- Flow metadata: `FlowNameChanged`, `FlowVersionChanged`,
  `FlowDescriptionChanged`, `FlowAuthorsChanged`,
  `ToggleMetadataEditor`
- Flow I/O: `FlowAddInput/Output`, `FlowDeleteInput/Output`,
  `FlowInput/OutputName/TypeChanged`
- `ToggleLibPaths`, `AddLibraryPath`, `RemoveLibraryPath`

These can be investigated for further extraction in future sub-projects.

## Testing

All 180 existing tests must pass after each extraction. No new tests
needed — this is a pure refactor (behavior unchanged). Run `make test`
after each module extraction.

## Key Files

| File | Change |
|------|--------|
| `flowedit/src/main.rs` | Remove message handlers, add dispatch calls |
| `flowedit/src/canvas_view.rs` | Add `handle_canvas_message()` + `CanvasAction` |
| `flowedit/src/hierarchy_panel.rs` | Handler already exists, add action enum |
| `flowedit/src/library_panel.rs` | Handler already exists, extend for main.rs dispatch |
| `flowedit/src/undo_redo.rs` | Already has `apply_undo`/`apply_redo` |
| `flowedit/src/flow_io.rs` | Already has `perform_*` functions |
| `flowedit/src/initializer.rs` | Already has `apply_initializer_edit` |
