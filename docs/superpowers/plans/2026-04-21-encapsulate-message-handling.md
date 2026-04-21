# Encapsulate Message Handling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move message handling from `FlowEdit::update()` in main.rs into each module's own handler function, reducing main.rs to a thin message router.

**Architecture:** Each module gets a `handle_*_message(win, msg) -> Action` function. The action enum captures cross-module effects (open window, rebuild library). main.rs dispatches to the handler and acts on returned actions. Pure refactor — no behavior changes.

**Scope note:** Hierarchy and library message handlers access `FlowEdit` fields (`self.windows`, `self.library_cache`, etc.) beyond `WindowState`, so they stay in main.rs for now. They'll be extracted in a follow-up when `FlowEdit` is restructured.

**Tech Stack:** Rust, iced 0.14.0

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `flowedit/src/canvas_view.rs` | Modify | Add `CanvasAction` enum + `handle_canvas_message()` |
| `flowedit/src/undo_redo.rs` | Modify | Add `handle_undo()` / `handle_redo()` wrappers |
| `flowedit/src/initializer.rs` | Modify | Add `handle_initializer_message()` |
| `flowedit/src/flow_io.rs` | Modify | Add `handle_file_message()` + `FileAction` |
| `flowedit/src/main.rs` | Modify | Replace inline handlers with dispatch calls |

---

### Task 1: Extract canvas message handling

The largest extraction — all `CanvasMessage` variants plus `ZoomIn`/`ZoomOut`/`ToggleAutoFit`.

**Files:**
- Modify: `flowedit/src/canvas_view.rs`
- Modify: `flowedit/src/main.rs`

- [ ] **Step 1: Add `CanvasAction` enum and `handle_canvas_message` to canvas_view.rs**

At the top of `canvas_view.rs` (after the existing imports), add:

```rust
use crate::flow_io;
use crate::history::EditAction;
use crate::undo_redo;
use crate::WindowState;

/// Actions that canvas message handling needs main.rs to perform.
pub(crate) enum CanvasAction {
    /// Fully handled, no further action needed.
    None,
    /// Open a node in a new window (sub-flow or function viewer).
    OpenNode(usize),
}

/// Handle a canvas message, updating window state. Returns an action
/// if main.rs needs to do cross-module work (e.g., open a window).
pub(crate) fn handle_canvas_message(
    win: &mut WindowState,
    msg: CanvasMessage,
) -> CanvasAction {
    match msg {
        // ... move all CanvasMessage match arms from main.rs here ...
        // OpenNode returns CanvasAction::OpenNode(idx)
        // InitializerEdit sets win.initializer_editor directly (no action needed)
        // Everything else returns CanvasAction::None
    }
}

/// Handle ZoomIn message.
pub(crate) fn handle_zoom_in(win: &mut WindowState) {
    win.auto_fit_enabled = false;
    win.canvas_state.zoom_in();
    let pct = (win.canvas_state.zoom * 100.0) as u32;
    win.status = format!("Zoom: {pct}%");
}

/// Handle ZoomOut message.
pub(crate) fn handle_zoom_out(win: &mut WindowState) {
    win.auto_fit_enabled = false;
    win.canvas_state.zoom_out();
    let pct = (win.canvas_state.zoom * 100.0) as u32;
    win.status = format!("Zoom: {pct}%");
}

/// Handle ToggleAutoFit message.
pub(crate) fn handle_toggle_auto_fit(win: &mut WindowState) {
    win.auto_fit_enabled = !win.auto_fit_enabled;
    if win.auto_fit_enabled {
        win.auto_fit_pending = true;
        win.canvas_state.request_redraw();
        win.status = String::from("Auto-fit enabled");
    } else {
        win.status = String::from("Auto-fit disabled");
    }
}
```

Move ALL the `CanvasMessage` match arms from main.rs (lines 547-782) into `handle_canvas_message`. The `CanvasMessage::OpenNode(idx)` arm returns `CanvasAction::OpenNode(idx)` instead of calling `self.open_node()`. All other arms return `CanvasAction::None`.

Note: `CanvasMessage::ContextMenu` currently accesses `self.windows.get_mut(&win_id)` redundantly — the `win` parameter is already the window, so just set `win.context_menu = Some((x, y))` directly.

- [ ] **Step 2: Replace main.rs handlers with dispatch calls**

In main.rs `update()`, replace the entire `Message::WindowCanvas(...)` block (lines 543-783) with:

```rust
Message::WindowCanvas(win_id, canvas_msg) => {
    let Some(win) = self.windows.get_mut(&win_id) else {
        return Task::none();
    };
    match canvas_view::handle_canvas_message(win, canvas_msg) {
        canvas_view::CanvasAction::OpenNode(idx) => {
            return self.open_node(win_id, idx);
        }
        canvas_view::CanvasAction::None => {}
    }
}
```

Replace `Message::ZoomIn`, `ZoomOut`, `ToggleAutoFit` blocks with:

```rust
Message::ZoomIn(win_id) => {
    if let Some(win) = self.windows.get_mut(&win_id) {
        canvas_view::handle_zoom_in(win);
    }
}
Message::ZoomOut(win_id) => {
    if let Some(win) = self.windows.get_mut(&win_id) {
        canvas_view::handle_zoom_out(win);
    }
}
Message::ToggleAutoFit(win_id) => {
    if let Some(win) = self.windows.get_mut(&win_id) {
        canvas_view::handle_toggle_auto_fit(win);
    }
}
```

- [ ] **Step 3: Remove now-unused imports from main.rs**

After moving the canvas handlers, some imports in main.rs may become unused (e.g., `EditAction`, `InputInitializer` if only used by canvas handlers). Clean up.

- [ ] **Step 4: Verify all tests pass**

Run: `cargo test -p flowedit`
Expected: All 180 tests pass

- [ ] **Step 5: Run clippy and fmt**

Run: `make clippy && cargo fmt -p flowedit`

- [ ] **Step 6: Commit**

```bash
git add flowedit/src/canvas_view.rs flowedit/src/main.rs
git commit -m "flowedit: Extract canvas message handling from main.rs (#2593)"
```

---

### Task 2: Extract undo/redo message handling

Simple — the handlers just dispatch to existing `apply_undo`/`apply_redo`.

**Files:**
- Modify: `flowedit/src/undo_redo.rs`
- Modify: `flowedit/src/main.rs`

- [ ] **Step 1: Add handler functions to undo_redo.rs**

```rust
/// Handle the Undo message for the given window.
pub(crate) fn handle_undo(win: &mut WindowState) {
    apply_undo(win);
    win.unsaved_edits = (win.unsaved_edits - 1).max(0);
}

/// Handle the Redo message for the given window.
pub(crate) fn handle_redo(win: &mut WindowState) {
    apply_redo(win);
    win.unsaved_edits += 1;
}
```

- [ ] **Step 2: Replace main.rs handlers**

Replace `Message::Undo` and `Message::Redo` blocks with:

```rust
Message::Undo => {
    let target = self.focused_window.or(self.root_window);
    if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
        undo_redo::handle_undo(win);
    }
}
Message::Redo => {
    let target = self.focused_window.or(self.root_window);
    if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
        undo_redo::handle_redo(win);
    }
}
```

- [ ] **Step 3: Verify, clippy, fmt, commit**

Run: `cargo test -p flowedit && make clippy && cargo fmt -p flowedit`

```bash
git add flowedit/src/undo_redo.rs flowedit/src/main.rs
git commit -m "flowedit: Extract undo/redo message handling from main.rs (#2593)"
```

---

### Task 3: Extract initializer message handling

**Files:**
- Modify: `flowedit/src/initializer.rs`
- Modify: `flowedit/src/main.rs`

- [ ] **Step 1: Add handler functions to initializer.rs**

```rust
use crate::InitializerEditor;

/// Handle InitializerTypeChanged message.
pub(crate) fn handle_type_changed(win: &mut WindowState, new_type: String) {
    if let Some(ref mut editor) = win.initializer_editor {
        editor.init_type = new_type;
    }
}

/// Handle InitializerValueChanged message.
pub(crate) fn handle_value_changed(win: &mut WindowState, new_value: String) {
    if let Some(ref mut editor) = win.initializer_editor {
        editor.value_text = new_value;
    }
}

/// Handle InitializerApply message.
pub(crate) fn handle_apply(win: &mut WindowState) {
    if let Some(editor) = win.initializer_editor.take() {
        apply_initializer_edit(win, &editor);
    }
}

/// Handle InitializerCancel message.
pub(crate) fn handle_cancel(win: &mut WindowState) {
    win.initializer_editor = None;
}
```

- [ ] **Step 2: Replace main.rs handlers**

Replace the four `InitializerTypeChanged`, `InitializerValueChanged`, `InitializerApply`, `InitializerCancel` blocks with:

```rust
Message::InitializerTypeChanged(win_id, new_type) => {
    if let Some(win) = self.windows.get_mut(&win_id) {
        initializer::handle_type_changed(win, new_type);
    }
}
Message::InitializerValueChanged(win_id, new_value) => {
    if let Some(win) = self.windows.get_mut(&win_id) {
        initializer::handle_value_changed(win, new_value);
    }
}
Message::InitializerApply(win_id) => {
    if let Some(win) = self.windows.get_mut(&win_id) {
        initializer::handle_apply(win);
    }
}
Message::InitializerCancel(win_id) => {
    if let Some(win) = self.windows.get_mut(&win_id) {
        initializer::handle_cancel(win);
    }
}
```

- [ ] **Step 3: Verify, clippy, fmt, commit**

Run: `cargo test -p flowedit && make clippy && cargo fmt -p flowedit`

```bash
git add flowedit/src/initializer.rs flowedit/src/main.rs
git commit -m "flowedit: Extract initializer message handling from main.rs (#2593)"
```

---

### Task 4: Extract file operation message handling

**Files:**
- Modify: `flowedit/src/flow_io.rs`
- Modify: `flowedit/src/main.rs`

- [ ] **Step 1: Add `FileAction` enum and handler to flow_io.rs**

```rust
use std::collections::BTreeSet;

/// Actions that file message handling needs main.rs to perform.
pub(crate) enum FileAction {
    /// Fully handled.
    None,
    /// A flow was opened — main.rs should rebuild library cache.
    FlowOpened {
        lib_refs: BTreeSet<Url>,
    },
    /// A new flow was created — main.rs should clear library cache.
    NewFlow,
}

/// Handle Save message for the given window.
pub(crate) fn handle_save(win: &mut WindowState) {
    if let Some(path) = win.file_path.clone() {
        perform_save(win, &path);
    } else {
        perform_save_as(win);
    }
}

/// Handle SaveAs message for the given window.
pub(crate) fn handle_save_as(win: &mut WindowState) {
    perform_save_as(win);
}

/// Handle Open message. Returns FileAction::FlowOpened if a flow was loaded.
pub(crate) fn handle_open(win: &mut WindowState) -> FileAction {
    if let Some((lib_refs, _ctx_refs)) = perform_open(win) {
        FileAction::FlowOpened { lib_refs }
    } else {
        FileAction::None
    }
}

/// Handle New message.
pub(crate) fn handle_new(win: &mut WindowState) -> FileAction {
    perform_new(win);
    FileAction::NewFlow
}
```

- [ ] **Step 2: Replace main.rs handlers**

Replace `Message::Save`, `SaveAs`, `Open`, `New` blocks. The `Open` handler needs main.rs to rebuild hierarchy and library cache based on the returned action.

```rust
Message::Save => {
    let target = self.focused_window.or(self.root_window);
    if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
        flow_io::handle_save(win);
    }
}
Message::SaveAs => {
    let target = self.focused_window.or(self.root_window);
    if let Some(win) = target.and_then(|id| self.windows.get_mut(&id)) {
        flow_io::handle_save_as(win);
    }
}
Message::Open => {
    if let Some(root_id) = self.root_window {
        if let Some(win) = self.windows.get_mut(&root_id) {
            match flow_io::handle_open(win) {
                flow_io::FileAction::FlowOpened { lib_refs } => {
                    self.root_flow_path = win.file_path.clone();
                    win.flow_hierarchy = win
                        .file_path
                        .as_ref()
                        .map(|p| FlowHierarchy::build(p))
                        .unwrap_or_else(FlowHierarchy::empty);
                    let (lc, ld, cd) = library_mgmt::load_library_catalogs(&lib_refs);
                    self.library_cache = lc;
                    self.lib_definitions = ld;
                    self.context_definitions = cd;
                    self.library_tree = LibraryTree::from_cache(
                        &self.library_cache,
                        &self.lib_definitions,
                        &self.context_definitions,
                    );
                }
                flow_io::FileAction::None | flow_io::FileAction::NewFlow => {}
            }
        }
    }
}
Message::New => {
    if let Some(win) = self.root_window.and_then(|id| self.windows.get_mut(&id)) {
        let _ = flow_io::handle_new(win);
        self.library_cache.clear();
        self.lib_definitions.clear();
        self.context_definitions.clear();
        self.library_tree = LibraryTree::from_cache(
            &self.library_cache,
            &self.lib_definitions,
            &self.context_definitions,
        );
    }
}
```

Note: `Message::Compile` can stay in main.rs for now since it accesses `win.compiled_manifest` and updates status — it's already a thin wrapper around `flow_io::perform_compile`.

- [ ] **Step 3: Verify, clippy, fmt, commit**

Run: `cargo test -p flowedit && make clippy && cargo fmt -p flowedit`

```bash
git add flowedit/src/flow_io.rs flowedit/src/main.rs
git commit -m "flowedit: Extract file operation message handling from main.rs (#2593)"
```

---

### Task 5: Final verification and line count

- [ ] **Step 1: Run full test suite**

Run: `make test`
Expected: All tests pass

- [ ] **Step 2: Run clippy and fmt**

Run: `make clippy && cargo fmt -p flowedit`

- [ ] **Step 3: Count main.rs reduction**

Run: `wc -l flowedit/src/main.rs`
Expected: Significant reduction from 2891 lines (canvas alone is ~240 lines of handlers)

- [ ] **Step 4: Commit any final adjustments**

```bash
git add -A
git commit -m "flowedit: Final cleanup after message handling extraction (#2593)"
```
