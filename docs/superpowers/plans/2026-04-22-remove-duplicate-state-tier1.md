# Remove Duplicate State — Tier 1 (Quick Wins) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove three redundant fields from flowedit that duplicate data already in `FlowDefinition`: `WindowState.file_path`, `WindowState.flow_inputs`/`flow_outputs`, and `FlowEdit.root_flow_path`.

**Architecture:** Each field is replaced by reading from / writing to the canonical `FlowDefinition` stored in `WindowState.flow_definition`. `file_path` maps to `flow_definition.source_url` (a `Url`). `flow_inputs`/`flow_outputs` (Vec<PortInfo>) map to `flow_definition.inputs`/`flow_definition.outputs` (Vec<IO>). `root_flow_path` is derived from the root window's `source_url`. This also fixes a bug where flow I/O edits (AddInput, DeleteInput, etc.) were never synced to the FlowDefinition and would be lost on save.

**Tech Stack:** Rust, flowcore (model types), iced (GUI framework)

**Important:** Any edits proposed outside of `flowedit/` must be shown to the user before being made.

---

### Task 1: Add `set_name` method to `IO` in flowcore

The `IO` struct's `name` field is private, accessed via the `HasName` trait which only provides `fn name(&self) -> &Name`. flowedit needs to rename ports, so we need a setter.

**Files:**
- Modify: `flowcore/src/model/io.rs:70` (impl IO block)

**IMPORTANT: Show this edit to the user before making it — it is outside flowedit.**

- [ ] **Step 1: Add `set_name` method to IO**

Add this method inside the `impl IO` block (after `new_named`, around line 93):

```rust
/// Set the name of this IO
pub fn set_name(&mut self, name: Name) {
    self.name = name;
}
```

- [ ] **Step 2: Verify flowcore builds**

Run: `cargo build -p flowcore`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add flowcore/src/model/io.rs
git commit -m "flowcore: Add set_name method to IO for editor support (#2593)"
```

---

### Task 2: Remove `WindowState.file_path` — use `flow_definition.source_url`

Replace `file_path: Option<PathBuf>` with reads/writes to `flow_definition.source_url: Url`. The mapping:
- `file_path = None` → `source_url == FlowDefinition::default_url()` (which is `"file://"`)
- `file_path = Some(path)` → `source_url = Url::from_file_path(path)`
- Reading: `source_url.to_file_path().ok()` → `Option<PathBuf>`

Add a helper method on `WindowState` to avoid repeating the conversion.

**Files:**
- Modify: `flowedit/src/window_state.rs` — remove `file_path` field, add `file_path()` helper method
- Modify: `flowedit/src/main.rs` — update all `win.file_path` reads/writes
- Modify: `flowedit/src/flow_io.rs` — update `perform_save`, `handle_save`, `perform_open`, `perform_new`, `perform_compile`
- Modify: `flowedit/src/library_mgmt.rs` — update `resolve_node_source`
- Modify: `flowedit/src/ui_test.rs` — update test that sets `win.file_path`

- [ ] **Step 1: Add `file_path()` and `set_file_path()` helpers to WindowState**

In `flowedit/src/window_state.rs`, add to the `impl WindowState` block:

```rust
/// Get the file path from the flow definition's source URL.
/// Returns None if no file has been saved/loaded yet.
pub(crate) fn file_path(&self) -> Option<PathBuf> {
    self.flow_definition.source_url.to_file_path().ok()
}

/// Set the file path by updating the flow definition's source URL.
pub(crate) fn set_file_path(&mut self, path: &Path) {
    if let Ok(url) = Url::from_file_path(path) {
        self.flow_definition.source_url = url;
    }
}

/// Clear the file path by resetting the source URL to the default.
pub(crate) fn clear_file_path(&mut self) {
    self.flow_definition.source_url = FlowDefinition::default_url();
}
```

Add `use std::path::Path;` and `use url::Url;` to the imports.

- [ ] **Step 2: Remove `file_path` field from WindowState**

Remove the `file_path: Option<PathBuf>` field from the struct and from `Default::default()`.

- [ ] **Step 3: Update flow_io.rs**

Replace all `win.file_path` reads with `win.file_path()` and all `win.file_path = Some(path)` writes with `win.set_file_path(&path)`. In `perform_new`, replace `win.file_path = None` with `win.clear_file_path()`.

Key changes:
- `perform_save`: `win.file_path = Some(path.clone())` → `win.set_file_path(path)`
- `handle_save`: `win.file_path.clone()` → `win.file_path()`
- `perform_open`: `win.file_path = Some(path)` → `win.set_file_path(&path)`
- `perform_new`: `win.file_path = None` → `win.clear_file_path()`
- `perform_compile`: `win.file_path.is_none()` → `win.file_path().is_none()`, `win.file_path.clone()` → `win.file_path()`

- [ ] **Step 4: Update main.rs**

Replace all `file_path` field accesses on WindowState:
- In `new()` (init): set `flow_definition.source_url` before constructing WindowState, remove `file_path` from struct literal
- In `Message::Open`: `win.file_path.clone()` → `win.file_path()`, `self.root_flow_path = win.file_path.clone()` → derive from source_url
- In node-opening code: `win.file_path.as_ref()` → `win.file_path()`
- In window creation: remove `file_path` field from WindowState literals, set source_url on flow_definition instead
- In `compile` button handler: `win.file_path.as_ref()` → `win.file_path()`

- [ ] **Step 5: Update library_mgmt.rs**

`resolve_node_source`: `win.file_path.as_ref()?.parent()?` → `win.file_path()?.parent()?.to_path_buf()` (adjust as needed for the borrow)

- [ ] **Step 6: Update ui_test.rs**

Replace `win.file_path = Some(path.clone())` with `win.set_file_path(&path)`.

- [ ] **Step 7: Update test WindowState literals in flow_io.rs, library_mgmt.rs, history.rs**

Remove `file_path: None` / `file_path: Some(...)` from all test WindowState struct literals. For tests that need a file path, use `win.set_file_path(&path)` after construction.

- [ ] **Step 8: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`
Expected: All tests pass

- [ ] **Step 9: Commit**

```bash
git add flowedit/src/window_state.rs flowedit/src/main.rs flowedit/src/flow_io.rs flowedit/src/library_mgmt.rs flowedit/src/ui_test.rs flowedit/src/history.rs
git commit -m "flowedit: Remove WindowState.file_path, use flow_definition.source_url (#2593)"
```

---

### Task 3: Remove `WindowState.flow_inputs`/`flow_outputs` — use `flow_definition.inputs`/`outputs`

Replace `flow_inputs: Vec<PortInfo>` and `flow_outputs: Vec<PortInfo>` with direct reads/writes to `flow_definition.inputs: Vec<IO>` and `flow_definition.outputs: Vec<IO>`.

This also fixes a bug: the `AddInput`, `DeleteInput`, `InputNameChanged`, `InputTypeChanged` (and output equivalents) message handlers currently only modify the `PortInfo` vecs, never updating `flow_definition.inputs`/`outputs`. Since `save_flow_toml` serializes from `flow_definition.inputs`/`outputs`, flow I/O edits are silently lost on save.

The `PortInfo` struct is still used by `NodeLayout` for subprocess ports — it is NOT removed in this task (that happens in Tier 2 when NodeLayout is eliminated).

**Files:**
- Modify: `flowedit/src/window_state.rs` — remove `flow_inputs`/`flow_outputs` fields
- Modify: `flowedit/src/main.rs` — update all FlowEditMessage handlers and view code to use `flow_definition.inputs`/`outputs`
- Modify: `flowedit/src/flow_io.rs` — remove `extract_ports` calls for flow-level I/O, remove `flow_inputs`/`flow_outputs` from `perform_open`/`perform_new`
- Modify: `flowedit/src/canvas_view.rs` — change functions that take `&[PortInfo]` for flow I/O to take `&[IO]`
- Modify: `flowedit/src/ui_test.rs` — update assertions to check `flow_definition.inputs`/`outputs` instead of `flow_inputs`/`flow_outputs`

- [ ] **Step 1: Update canvas_view.rs rendering functions to accept `&[IO]`**

Change these function signatures to use `&[IO]` instead of `&[PortInfo]`:
- `compute_flow_io_positions(nodes, flow_inputs, flow_outputs)` — change params to `&[IO]`, access name via `io.name()` (returns `&str`)
- `draw_flow_io_ports(...)` — change flow_inputs/flow_outputs params to `&[IO]`
- `draw_flow_io_beziers(...)` — change flow_inputs/flow_outputs params to `&[IO]`
- `FlowCanvasData` struct — change `flow_inputs`/`flow_outputs` fields to `&'a [IO]`

In all these functions, `input.name.clone()` becomes `input.name().to_string()` and `input.name` becomes `input.name()`.

Add `use flowcore::model::io::IO;` to canvas_view.rs imports. Add `use flowcore::model::name::HasName;` for the `name()` trait method.

- [ ] **Step 2: Update canvas_view.rs callers**

Where `FlowCanvasData` is constructed (in `view_canvas_area` or similar), pass `&win.flow_definition.inputs` and `&win.flow_definition.outputs` instead of `&win.flow_inputs` and `&win.flow_outputs`.

The `has_flow_io` check: `!win.flow_inputs.is_empty() || !win.flow_outputs.is_empty()` becomes `!win.flow_definition.inputs.is_empty() || !win.flow_definition.outputs.is_empty()`.

- [ ] **Step 3: Update main.rs FlowEditMessage handlers**

Replace all `win.flow_inputs` / `win.flow_outputs` usage with `win.flow_definition.inputs` / `win.flow_definition.outputs`:

`AddInput`:
```rust
FlowEditMessage::AddInput => {
    let name = format!("input{}", win.flow_definition.inputs.len());
    let io = IO::new_named(
        vec![DataType::from("string")],
        Route::default(),
        name,
    );
    win.flow_definition.inputs.push(io);
    win.unsaved_edits += 1;
    win.canvas_state.request_redraw();
}
```

`AddOutput`:
```rust
FlowEditMessage::AddOutput => {
    let name = format!("output{}", win.flow_definition.outputs.len());
    let io = IO::new_named(
        vec![DataType::from("string")],
        Route::default(),
        name,
    );
    win.flow_definition.outputs.push(io);
    win.unsaved_edits += 1;
    win.canvas_state.request_redraw();
}
```

`DeleteInput(idx)`:
```rust
FlowEditMessage::DeleteInput(idx) => {
    if idx < win.flow_definition.inputs.len() {
        let name = win.flow_definition.inputs[idx].name().to_string();
        win.flow_definition.inputs.remove(idx);
        win.edges.retain(|e| !(e.from_node == "input" && e.from_port == name));
        win.unsaved_edits += 1;
        win.canvas_state.request_redraw();
    }
}
```

`DeleteOutput(idx)`:
```rust
FlowEditMessage::DeleteOutput(idx) => {
    if idx < win.flow_definition.outputs.len() {
        let name = win.flow_definition.outputs[idx].name().to_string();
        win.flow_definition.outputs.remove(idx);
        win.edges.retain(|e| !(e.to_node == "output" && e.to_port == name));
        win.unsaved_edits += 1;
        win.canvas_state.request_redraw();
    }
}
```

`InputNameChanged(idx, name)`:
```rust
FlowEditMessage::InputNameChanged(idx, name) => {
    if let Some(io) = win.flow_definition.inputs.get_mut(idx) {
        let old_name = io.name().to_string();
        io.set_name(name.clone());
        for edge in &mut win.edges {
            if edge.from_node == "input" && edge.from_port == old_name {
                edge.from_port = name.clone();
            }
        }
    }
    win.unsaved_edits += 1;
    win.canvas_state.request_redraw();
}
```

`InputTypeChanged(idx, dtype)`:
```rust
FlowEditMessage::InputTypeChanged(idx, dtype) => {
    if let Some(io) = win.flow_definition.inputs.get_mut(idx) {
        io.set_datatypes(&[DataType::from(dtype)]);
    }
    win.unsaved_edits += 1;
    win.canvas_state.request_redraw();
}
```

`OutputNameChanged(idx, name)`:
```rust
FlowEditMessage::OutputNameChanged(idx, name) => {
    if let Some(io) = win.flow_definition.outputs.get_mut(idx) {
        let old_name = io.name().to_string();
        io.set_name(name.clone());
        for edge in &mut win.edges {
            if edge.to_node == "output" && edge.to_port == old_name {
                edge.to_port = name.clone();
            }
        }
    }
    win.unsaved_edits += 1;
    win.canvas_state.request_redraw();
}
```

`OutputTypeChanged(idx, dtype)`:
```rust
FlowEditMessage::OutputTypeChanged(idx, dtype) => {
    if let Some(io) = win.flow_definition.outputs.get_mut(idx) {
        io.set_datatypes(&[DataType::from(dtype)]);
    }
    win.unsaved_edits += 1;
    win.canvas_state.request_redraw();
}
```

Add imports: `use flowcore::model::io::IO;`, `use flowcore::model::datatype::DataType;`, `use flowcore::model::route::Route;`, `use flowcore::model::name::HasName;`.

- [ ] **Step 4: Update main.rs view_flow_io_panel**

Change `win.flow_inputs.iter()` to `win.flow_definition.inputs.iter()` and `win.flow_outputs.iter()` to `win.flow_definition.outputs.iter()`.

For each `port` (now an `&IO`):
- `port.name` → `port.name().to_string()` (for display) or `port.name()` (for &str comparison)
- `port.datatypes.first().cloned().unwrap_or_default()` → `port.datatypes().first().map(|dt| dt.to_string()).unwrap_or_default()`

- [ ] **Step 5: Update flow_io.rs**

- Remove `extract_ports` calls for flow-level I/O in `perform_open` and throughout
- In `perform_open`: remove `win.flow_inputs = fi; win.flow_outputs = fo;`
- In `perform_new`: remove `win.flow_inputs = Vec::new(); win.flow_outputs = Vec::new();`
- `extract_ports` function itself stays (still used by `build_node_layouts` for subprocess ports) but the calls for flow-level I/O are removed
- Remove `flow_inputs`/`flow_outputs` from test WindowState literals

- [ ] **Step 6: Update main.rs init and window creation**

Remove `flow_inputs: fi, flow_outputs: fo` from all WindowState struct literals. Remove the `extract_ports` call for flow-level I/O in `new()`. The flow_definition already has the correct `inputs`/`outputs` from deserialization.

For window creation in `open_node`, `create_new_subflow`, `create_new_function`, etc.: remove `flow_inputs`/`flow_outputs` from the WindowState literals.

- [ ] **Step 7: Remove fields from WindowState**

In `window_state.rs`:
- Remove `flow_inputs: Vec<PortInfo>` and `flow_outputs: Vec<PortInfo>` fields
- Remove them from `Default::default()`
- Remove the `PortInfo` import if it's no longer used here (it's still used by `FunctionViewer`)

- [ ] **Step 8: Update ui_test.rs**

Change all test assertions:
- `w.flow_inputs.len()` → `w.flow_definition.inputs.len()`
- `w.flow_outputs.len()` → `w.flow_definition.outputs.len()`
- `w.flow_inputs.first().map(|p| p.name.as_str())` → `w.flow_definition.inputs.first().map(|io| io.name().as_str())`  (add `use flowcore::model::name::HasName;`)
- `w.flow_inputs.first().and_then(|p| p.datatypes.first()).map(|s| s.as_str())` → `w.flow_definition.inputs.first().and_then(|io| io.datatypes().first()).map(|dt| dt.to_string())` — note: this changes the comparison type from `&str` to `String`, so adjust `assert_eq!` accordingly

- [ ] **Step 9: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`
Expected: All tests pass

- [ ] **Step 10: Commit**

```bash
git add flowedit/src/window_state.rs flowedit/src/main.rs flowedit/src/flow_io.rs flowedit/src/canvas_view.rs flowedit/src/ui_test.rs
git commit -m "flowedit: Remove flow_inputs/flow_outputs, use flow_definition.inputs/outputs directly (#2593)

Fixes bug where flow I/O edits (add/delete/rename ports) were lost on save
because only the PortInfo duplicates were updated, not flow_definition."
```

---

### Task 4: Remove `FlowEdit.root_flow_path` — derive from root window

Replace `root_flow_path: Option<PathBuf>` with a helper method that reads from the root window's `flow_definition.source_url`.

**Files:**
- Modify: `flowedit/src/main.rs` — remove field, add helper, update all usages

- [ ] **Step 1: Add helper method to FlowEdit**

```rust
fn root_flow_path(&self) -> Option<PathBuf> {
    self.root_window
        .and_then(|id| self.windows.get(&id))
        .and_then(|win| win.file_path())
}
```

- [ ] **Step 2: Remove `root_flow_path` field**

Remove from the struct definition and from `Default::default()`.

- [ ] **Step 3: Update all usages**

- In `new()`: remove `let root_flow_path = file_path.clone();` and `root_flow_path,` from struct literal
- In `Message::Open`: remove `self.root_flow_path = win.file_path.clone();` (no longer needed — it's derived)
- In `build_hierarchy()`: change `self.root_flow_path.as_ref()` to `self.root_flow_path()` (the new helper returns `Option<PathBuf>`, so adjust `.map(|p| ...)` to `.as_ref().map(|p| ...)` or use `as_deref()`)

- [ ] **Step 4: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add flowedit/src/main.rs
git commit -m "flowedit: Remove FlowEdit.root_flow_path, derive from root window (#2593)"
```

---

### Task 5: Run full test suite and lint

- [ ] **Step 1: Format**

Run: `cargo fmt`

- [ ] **Step 2: Clippy**

Run: `make clippy`
Expected: Clean

- [ ] **Step 3: Full test suite**

Run: `make test`
Expected: All tests pass

- [ ] **Step 4: Fix any issues found**

If clippy or tests fail, fix the issues and re-run.

- [ ] **Step 5: Final commit if needed**

Only if formatting or clippy fixes were needed.
