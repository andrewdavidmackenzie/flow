# Eliminate EdgeLayout — Use Connection Directly

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove `EdgeLayout` and `WindowState.edges`, using `flow_definition.connections: Vec<Connection>` directly for rendering, editing, and serialization.

**Architecture:** `EdgeLayout` is a flat `(from_node, from_port, to_node, to_port, name)` extracted from `Connection`'s Route-based format. We eliminate it by having the canvas parse `Connection.from()`/`Connection.to()` routes on the fly via `split_route()`. History stores `Connection` instead of `EdgeLayout`. `save_flow_toml` serializes from `flow_definition.connections` directly. Note: `Connection.to` is `Vec<Route>` (fan-out), but the editor creates 1:1 connections — each Connection has exactly one to-route.

**Tech Stack:** Rust, flowcore (Connection, Route), iced (canvas rendering)

**Important:** Any edits proposed outside of `flowedit/` must be shown to the user before being made. `flowcore/src/model/connection.rs` already has getters/setters added in prior commit.

---

## File Map

| File | Changes |
|------|---------|
| `flowedit/src/window_state.rs` | Remove `edges: Vec<EdgeLayout>` field |
| `flowedit/src/canvas_view.rs` | Change all functions taking `&[EdgeLayout]` to take `&[Connection]`; remove `EdgeLayout` struct, `build_edge_layouts()`, `EdgeLayout::new()`, `EdgeLayout::references_node()`; add `connection_references_node()` helper |
| `flowedit/src/history.rs` | Change `CreateConnection { edge: EdgeLayout }` and `DeleteConnection { index, edge: EdgeLayout }` to use `Connection`; update `apply_undo`/`apply_redo` |
| `flowedit/src/main.rs` | Replace all `win.edges` with `win.flow_definition.connections`; update connection creation to build `Connection` objects; update edge-related FlowEditMessage handlers |
| `flowedit/src/flow_io.rs` | Remove `edges` parameter from `save_flow_toml()`, serialize from `flow_definition.connections`; remove `build_edge_layouts()` calls from `load_flow()`; remove `edges` from `LoadedFlow` |
| `flowedit/src/initializer.rs` | No changes needed (doesn't reference edges) |
| `flowedit/src/ui_test.rs` | Update tests that create/assert on `EdgeLayout` to use `Connection` |
| `flowedit/src/library_mgmt.rs` | Minor — remove EdgeLayout import if present |

---

## Task 1: Add `connection_references_node` helper and update `split_route` visibility

**Files:**
- Modify: `flowedit/src/canvas_view.rs`

This helper replaces `EdgeLayout::references_node()`. It checks whether a `Connection`'s from or to routes reference a given node alias.

- [ ] **Step 1: Add the helper function**

```rust
/// Check whether a Connection references a node by alias in its from or to routes.
pub(crate) fn connection_references_node(conn: &Connection, alias: &str) -> bool {
    let (from_node, _) = split_route(&conn.from().to_string());
    if from_node == alias {
        return true;
    }
    for to_route in conn.to() {
        let (to_node, _) = split_route(&to_route.to_string());
        if to_node == alias {
            return true;
        }
    }
    false
}
```

Add `use flowcore::model::connection::Connection;` to canvas_view.rs imports if not present. Make `split_route` `pub(crate)` so other modules can use it.

- [ ] **Step 2: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`

- [ ] **Step 3: Commit**

---

## Task 2: Update EditAction to use Connection instead of EdgeLayout

**Files:**
- Modify: `flowedit/src/history.rs`

- [ ] **Step 1: Change EditAction variants**

Replace:
```rust
CreateConnection { edge: EdgeLayout },
DeleteConnection { index: usize, edge: EdgeLayout },
```
With:
```rust
CreateConnection { connection: Connection },
DeleteConnection { index: usize, connection: Connection },
```

Update imports: add `use flowcore::model::connection::Connection;`, remove `EdgeLayout` import.

- [ ] **Step 2: Update apply_undo**

In `apply_undo` for `CreateConnection`: instead of matching on edge fields, use `connection_references_node` or route comparison to find and remove the connection:
```rust
EditAction::CreateConnection { connection } => {
    let from_str = connection.from().to_string();
    let to_strs: Vec<String> = connection.to().iter().map(ToString::to_string).collect();
    win.flow_definition.connections.retain(|c| {
        c.from().to_string() != from_str
            || c.to().iter().map(ToString::to_string).collect::<Vec<_>>() != to_strs
    });
    win.status = String::from("Undo: create connection");
}
```

For `DeleteConnection`: insert the connection back:
```rust
EditAction::DeleteConnection { index, connection } => {
    let idx = index.min(win.flow_definition.connections.len());
    win.flow_definition.connections.insert(idx, connection);
    win.status = String::from("Undo: delete connection");
}
```

- [ ] **Step 3: Update apply_redo**

Mirror the undo logic:
- `CreateConnection`: push the connection
- `DeleteConnection`: remove at index

- [ ] **Step 4: Update history tests**

Change test helpers that create `EdgeLayout::new(...)` to create `Connection::new("from_node/from_port", "to_node/to_port")` (or `Connection::new("from_node", "to_node")` for portless connections). Update assertions.

- [ ] **Step 5: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`

- [ ] **Step 6: Commit**

---

## Task 3: Remove `edges` from WindowState and LoadedFlow

**Files:**
- Modify: `flowedit/src/window_state.rs`
- Modify: `flowedit/src/flow_io.rs`

- [ ] **Step 1: Remove `edges` from WindowState**

Remove `pub(crate) edges: Vec<EdgeLayout>` from the struct and `edges: Vec::new()` from `Default::default()`. Remove the `EdgeLayout` import if no longer needed (keep other imports from canvas_view).

- [ ] **Step 2: Remove `edges` from LoadedFlow**

In `flow_io.rs`, remove `pub(crate) edges: Vec<EdgeLayout>` from `LoadedFlow`. Remove the `build_edge_layouts` call in `load_flow()`. The connections are already in `loaded.flow_def.connections`.

- [ ] **Step 3: Update all WindowState struct literals**

In `main.rs`: remove `edges: loaded.edges` / `edges: Vec::new()` from every WindowState construction. There are ~8 locations (new(), hierarchy open, open_node flows, create_new_subflow, create_new_function, etc.).

In `flow_io.rs` tests: remove `edges` from `test_win_state()`.

In `library_mgmt.rs` tests: remove `edges` from `test_win_state()`.

- [ ] **Step 4: Build (expect errors in canvas_view.rs, history.rs, main.rs)**

The build will fail because code still references `win.edges`. That's expected — the next tasks fix those references.

- [ ] **Step 5: Commit (WIP — will not build yet)**

---

## Task 4: Update canvas_view.rs to use Connection

**Files:**
- Modify: `flowedit/src/canvas_view.rs`

This is the largest task. Every function that takes `&[EdgeLayout]` or references `win.edges` must use `&[Connection]` / `win.flow_definition.connections`.

- [ ] **Step 1: Update FlowCanvas struct and FlowCanvasState::view()**

Change `edges: &'a [EdgeLayout]` to `connections: &'a [Connection]` in both the `FlowCanvas` struct and the `view()` method signature. Update the Canvas::new construction.

- [ ] **Step 2: Update draw_edges()**

Change signature from `edges: &[EdgeLayout]` to `connections: &[Connection]`. Inside the function, for each connection parse the routes:

```rust
for conn_idx in draw_order {
    let Some(conn) = connections.get(conn_idx) else { continue };
    let (from_node_str, from_port_str) = split_route(&conn.from().to_string());
    // Handle fan-out: iterate conn.to()
    for to_route in conn.to() {
        let (to_node_str, to_port_str) = split_route(&to_route.to_string());
        // ... rest of rendering logic using from_node_str, from_port_str, to_node_str, to_port_str
        // ... conn.name() for label
    }
}
```

- [ ] **Step 3: Update hit_test_connection()**

Change `edges` parameter to `connections: &[Connection]`. Same route-splitting approach inside the iteration.

- [ ] **Step 4: Update draw_flow_io_ports() edge iteration**

The section that draws bezier connections from flow I/O ports: change from `edges.iter()` to `connections.iter()`, parse routes with `split_route()`. Check `from_node == "input"` and `to_node == "output"` on the parsed strings.

- [ ] **Step 5: Update view_canvas_area()**

Where `FlowCanvas` / `FlowCanvasState::view()` is called: pass `&win.flow_definition.connections` instead of `&win.edges`.

- [ ] **Step 6: Update CanvasMessage handlers**

In `handle_canvas_message()`:
- `CanvasMessage::Deleted`: replace `win.edges.iter().filter(|e| e.references_node(...))` with `win.flow_definition.connections.iter().filter(|c| connection_references_node(c, ...))` and `.retain()` similarly
- `CanvasMessage::ConnectionCreated`: build `Connection::new(from_route, to_route)` where from_route = `format!("{from_node}/{from_port}")` (or just `from_node` if port is empty). Push to `win.flow_definition.connections`
- `CanvasMessage::ConnectionSelected`: index into `win.flow_definition.connections`, parse routes for status display
- `CanvasMessage::ConnectionDeleted`: remove from `win.flow_definition.connections`
- Status messages: use `win.flow_definition.connections.len()` for edge count

- [ ] **Step 7: Remove EdgeLayout struct, build_edge_layouts(), related methods**

Remove:
- `struct EdgeLayout` and its `impl` block (`new()`, `references_node()`)
- `fn build_edge_layouts()`
- Keep `split_route()` (still needed)

- [ ] **Step 8: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`

- [ ] **Step 9: Commit**

---

## Task 5: Update main.rs edge references

**Files:**
- Modify: `flowedit/src/main.rs`

- [ ] **Step 1: Update FlowEditMessage handlers**

`DeleteInput`/`DeleteOutput`: replace `win.edges.retain(...)` with `win.flow_definition.connections.retain(...)`, checking parsed routes:
```rust
win.flow_definition.connections.retain(|c| {
    let (from_node, _) = split_route(&c.from().to_string());
    !(from_node == "input" && /* port match */)
});
```

`InputNameChanged`/`OutputNameChanged`: update connection routes instead of edge strings. For each connection whose from/to references the old port name, use `conn.set_from()` or `conn.set_to()` with the updated route.

- [ ] **Step 2: Update open_node and window creation**

Remove `edges: loaded.edges` and similar from WindowState construction (if not already done in Task 3).

- [ ] **Step 3: Remove EdgeLayout import**

Remove `EdgeLayout` from the `use canvas_view::{...}` import in main.rs.

- [ ] **Step 4: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`

- [ ] **Step 5: Commit**

---

## Task 6: Update save_flow_toml to serialize from Connection

**Files:**
- Modify: `flowedit/src/flow_io.rs`

- [ ] **Step 1: Remove `edges` parameter from save_flow_toml()**

Change signature from `save_flow_toml(flow, edges, path)` to `save_flow_toml(flow, path)`. Serialize connections from `flow.connections`:

```rust
for conn in &flow.connections {
    let _ = writeln!(out, "\n[[connection]]");
    if !conn.name().is_empty() {
        let _ = writeln!(out, "name = \"{}\"", conn.name());
    }
    let _ = writeln!(out, "from = \"{}\"", conn.from());
    // Connection.to is Vec<Route>; write single or array
    if conn.to().len() == 1 {
        let _ = writeln!(out, "to = \"{}\"", conn.to()[0]);
    } else {
        let to_strs: Vec<String> = conn.to().iter().map(|r| format!("\"{r}\"")).collect();
        let _ = writeln!(out, "to = [{}]", to_strs.join(", "));
    }
}
```

- [ ] **Step 2: Update perform_save() call**

Change `save_flow_toml(&win.flow_definition, &win.edges, path)` to `save_flow_toml(&win.flow_definition, path)`.

- [ ] **Step 3: Remove format_endpoint() if only used for edge display**

Check if `format_endpoint()` is still needed. If it was only used by EdgeLayout status messages, remove it.

- [ ] **Step 4: Remove build_edge_layouts import and EdgeLayout from flow_io imports**

- [ ] **Step 5: Update tests**

Update `save_flow_toml` tests to not pass edges. Update assertions to verify connections serialize correctly.

- [ ] **Step 6: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`

- [ ] **Step 7: Commit**

---

## Task 7: Update ui_test.rs

**Files:**
- Modify: `flowedit/src/ui_test.rs`

- [ ] **Step 1: Replace EdgeLayout usage with Connection**

Every test that creates `EdgeLayout::new(...)` should create `Connection::new("from/port", "to/port")` instead.

Every test that pushes to `win.edges` should push to `win.flow_definition.connections`.

Every assertion on `win.edges.len()` should assert on `win.flow_definition.connections.len()`.

Import `Connection` from flowcore.

- [ ] **Step 2: Build and test**

Run: `cargo build -p flowedit && cargo test -p flowedit`

- [ ] **Step 3: Commit**

---

## Task 8: Final cleanup and full test

- [ ] **Step 1: Remove any remaining EdgeLayout references**

```bash
grep -rn 'EdgeLayout' flowedit/src/
```

Should return zero results. If any remain, fix them.

- [ ] **Step 2: cargo fmt**

- [ ] **Step 3: make clippy**

Fix any new warnings.

- [ ] **Step 4: make test**

All tests must pass.

- [ ] **Step 5: Final commit if needed**
