# Single FlowDefinition Owner Architecture

## Status: Planned (not yet started)

## Problem

After the state deduplication work in #2593, each window still owns its own `FlowDefinition`. The `FunctionViewer` clones its `FunctionDefinition` from the parent flow's `subprocesses`, meaning edits are not immediately reflected in the canonical tree. The `propagate_function_ports()` function manually syncs changes back, but this is fragile and creates a window where the data is inconsistent.

Similarly, when a sub-flow is opened in a child window, it gets its own `FlowDefinition` loaded independently, not a reference into the parent's `subprocesses`.

## Proposed Architecture

**`FlowEdit` owns the single canonical `FlowDefinition`:**

```
FlowEdit {
    flow_definition: FlowDefinition,       // THE source of truth
    compiled_manifest: Option<PathBuf>,     // one per app, not per window
    windows: HashMap<window::Id, WindowState>,
    ...
}
```

**Each `WindowState` knows what it's editing but doesn't own the data:**

```
enum EditTarget {
    RootFlow,                              // editing flow_definition itself
    SubFlow { alias: Name },               // editing flow_definition.subprocesses[alias]
    Function { alias: Name },              // editing a FunctionDefinition within subprocesses
}

struct WindowState {
    target: EditTarget,                    // what part of the tree this window edits
    canvas_state: FlowCanvasState,
    selected_node: Option<usize>,
    selected_connection: Option<usize>,
    // ... UI-only state
}
```

**`view()` and `update()` resolve the target:**

In iced's `update(&mut self, message)`, `self` is `&mut FlowEdit`. Both the canonical `flow_definition` and all `WindowState`s are accessible. The update method looks up the target:

```rust
fn get_flow_for_window(&self, win: &WindowState) -> &FlowDefinition {
    match &win.target {
        EditTarget::RootFlow => &self.flow_definition,
        EditTarget::SubFlow { alias } => {
            // walk subprocesses to find the nested FlowDefinition
        }
        EditTarget::Function { alias } => { /* not a flow */ }
    }
}
```

**Benefits:**
- No cloning or syncing — all edits go to the canonical tree
- `is_root` field removed — derived from `EditTarget::RootFlow`
- `compiled_manifest` moves to `FlowEdit` (one per app)
- `file_path()` derived from the root flow's `source_url`
- Hierarchy panel walks the canonical `subprocesses` tree directly
- Child windows are thin views, not independent copies

**Challenges:**
- Iced's `view()` takes `&self` — need to resolve target and pass the right `&FlowDefinition` or `&FunctionDefinition` to the view methods
- History undo/redo needs to modify the canonical tree, not a per-window copy
- Opening a sub-flow that isn't yet in `subprocesses` requires loading it via flowclib first

## Relationship to Other Work

This architecture change would also enable:
- **Hierarchy panel as a view onto FlowDefinition** (#2593) — walk `subprocesses` directly
- **Component encapsulation** (#2597) — each component's `view()` takes a reference to the relevant part of the canonical tree
- **Proper save** — serialize the single canonical `FlowDefinition` via serde

## Tasks

1. Move `flow_definition` from `WindowState` to `FlowEdit`
2. Add `EditTarget` enum to `WindowState`
3. Move `compiled_manifest` to `FlowEdit`
4. Remove `is_root` from `WindowState`
5. Update all `update()` handlers to resolve target from `FlowEdit.flow_definition`
6. Update all `view()` methods to receive the resolved `&FlowDefinition` or `&FunctionDefinition`
7. Remove `propagate_function_ports()` — edits go directly to the canonical tree
8. Update hierarchy panel to walk `FlowEdit.flow_definition.subprocesses`
9. Update history to operate on the canonical tree
