# Eliminate NodeLayout — Use ProcessReference + subprocesses Directly

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove `NodeLayout` and `WindowState.nodes`, rendering the canvas directly from `flow_definition.process_refs` (for layout) and `flow_definition.subprocesses` (for ports, description).

**Architecture:** `NodeLayout` duplicates data from `ProcessReference` (alias, source, x, y, width, height, initializations) and resolved subprocess definitions (inputs, outputs, description). We eliminate it by having the canvas read `ProcessReference` fields directly and look up port/description info from `FlowDefinition.subprocesses`. Mutations (drag, resize, add, delete) modify `process_refs` directly. `selected_node` indexes into `process_refs`. History `DeleteNode` stores `ProcessReference` + removed subprocess + removed connections.

**Key types:**

`ProcessReference` fields: `alias: Name`, `source: String`, `initializations: BTreeMap<String, InputInitializer>`, `x: Option<f32>`, `y: Option<f32>`, `width: Option<f32>`, `height: Option<f32>`

`FlowDefinition.subprocesses: BTreeMap<Name, Process>` — resolved definitions for each subprocess, keyed by alias. `Process` is either `FunctionProcess(FunctionDefinition)` or `FlowProcess(FlowDefinition)`, both of which have `inputs: IOSet`, `outputs: IOSet`, `description: String`.

**Rendering approach:** Create free functions that compute rendering properties from `ProcessReference` + subprocess lookup, replacing `NodeLayout` methods. For example, `node_fill_color(source: &str)`, `node_port_position(pref, port_index, is_input, subprocesses)`, etc.

---

## Task 1: Add rendering helper functions

**Files:** `flowedit/src/canvas_view.rs`

Add free functions that provide the same rendering info that NodeLayout methods did, but reading from ProcessReference + subprocesses:

```rust
fn node_x(pref: &ProcessReference) -> f32 { pref.x.unwrap_or(100.0) }
fn node_y(pref: &ProcessReference) -> f32 { pref.y.unwrap_or(100.0) }
fn node_width(pref: &ProcessReference) -> f32 { pref.width.unwrap_or(DEFAULT_WIDTH) }
fn node_height(pref: &ProcessReference, subprocesses: &BTreeMap<Name, Process>) -> f32 {
    let (inputs, outputs) = subprocess_ports(pref, subprocesses);
    let min_ports = inputs.len().max(outputs.len());
    let min_height = PORT_START_Y + (min_ports as f32 + 1.0) * PORT_SPACING;
    pref.height.unwrap_or(DEFAULT_HEIGHT.max(min_height))
}

fn node_alias(pref: &ProcessReference) -> &str {
    if pref.alias.is_empty() { derive_short_name(&pref.source) } // note: returns String, need to handle
    else { &pref.alias }
}

fn node_fill_color(source: &str) -> Color { /* same logic as NodeLayout::fill_color */ }
fn node_is_openable(source: &str) -> bool { !source.starts_with("lib://") && !source.starts_with("context://") }

fn subprocess_ports(pref: &ProcessReference, subprocesses: &BTreeMap<Name, Process>) -> (Vec<PortInfo>, Vec<PortInfo>) {
    let alias = if pref.alias.is_empty() { derive_short_name(&pref.source) } else { pref.alias.clone() };
    subprocesses.get(&alias).map(|proc| match proc {
        Process::FunctionProcess(f) => extract_ports(&f.inputs, &f.outputs),
        Process::FlowProcess(f) => extract_ports(&f.inputs, &f.outputs),
    }).unwrap_or_default()
}

fn subprocess_description(pref: &ProcessReference, subprocesses: &BTreeMap<Name, Process>) -> String {
    let alias = if pref.alias.is_empty() { derive_short_name(&pref.source) } else { pref.alias.clone() };
    subprocesses.get(&alias).map(|proc| match proc {
        Process::FunctionProcess(f) => f.description.clone(),
        Process::FlowProcess(f) => f.description.clone(),
    }).unwrap_or_default()
}

fn initializer_display(init: &InputInitializer) -> String {
    match init {
        InputInitializer::Once(v) => format!("once: {}", format_value(v)),
        InputInitializer::Always(v) => format!("always: {}", format_value(v)),
    }
}

fn node_output_port_position(pref: &ProcessReference, port_index: usize) -> Point {
    Point::new(
        node_x(pref) + node_width(pref),
        node_y(pref) + PORT_START_Y + port_index as f32 * PORT_SPACING,
    )
}

fn node_input_port_position(pref: &ProcessReference, port_index: usize) -> Point {
    Point::new(
        node_x(pref),
        node_y(pref) + PORT_START_Y + port_index as f32 * PORT_SPACING,
    )
}
```

These are additive — existing code continues to work.

## Task 2: Update canvas rendering to use ProcessReference

**Files:** `flowedit/src/canvas_view.rs`

Change `FlowCanvas` struct: replace `nodes: &'a [NodeLayout]` with `process_refs: &'a [ProcessReference]` and `subprocesses: &'a BTreeMap<Name, Process>`.

Update `FlowCanvasState::view()` to accept `process_refs` + `subprocesses` instead of `nodes`.

Update `draw_nodes()`, `draw_node()`, `draw_port()`, `draw_edges()`, `draw_flow_io_ports()`, `hit_test_node()`, `hit_test_port()`, `hit_test_connection()`, `hit_test_resize_handle()`, `hit_test_open_icon()`, `is_in_source_text_zone()`, `auto_fit()`, `compute_flow_io_positions()`, and all other functions that take `&[NodeLayout]` to use `&[ProcessReference]` + `&BTreeMap<Name, Process>`.

Replace `node.alias` with `node_alias(pref)`, `node.x` with `node_x(pref)`, `node.inputs` with `subprocess_ports(pref, subprocesses).0`, etc.

Update `view_canvas_area()` to pass `&win.flow_definition.process_refs` and `&win.flow_definition.subprocesses`.

## Task 3: Update CanvasMessage handlers

**Files:** `flowedit/src/canvas_view.rs`

In `handle_canvas_message()`:
- `Moved(idx, x, y)`: set `win.flow_definition.process_refs[idx].x = Some(x)` and `.y = Some(y)` directly
- `Resized(idx, x, y, w, h)`: set x/y/width/height on process_refs[idx] directly
- `MoveCompleted`: same, record history
- `Deleted(idx)`: remove from `win.flow_definition.process_refs`, also remove from `win.flow_definition.subprocesses`, remove connected connections
- `ConnectionCreated/Selected/Deleted`: already uses connections (from EdgeLayout elimination)

## Task 4: Update history to use ProcessReference

**Files:** `flowedit/src/history.rs`

Change `EditAction::DeleteNode`:
```rust
DeleteNode {
    index: usize,
    process_ref: ProcessReference,
    subprocess: Option<(Name, Process)>,
    removed_connections: Vec<Connection>,
}
```

`MoveNode` and `ResizeNode` stay the same (they store coordinates, not NodeLayout).

Update `apply_undo`/`apply_redo` for DeleteNode to insert/remove from `process_refs` and `subprocesses`.

## Task 5: Update main.rs — remove win.nodes

**Files:** `flowedit/src/main.rs`

- Remove `nodes` from all WindowState construction
- Node creation (NewSubFlow, NewFunction, add_library_function): add `ProcessReference` to `flow_definition.process_refs` and add the resolved `Process` to `flow_definition.subprocesses`
- `generate_unique_alias`: take `&[ProcessReference]` instead of `&[NodeLayout]`
- `next_node_position`: take `&[ProcessReference]` instead of `&[NodeLayout]`
- open_node: use process_refs index to find source

## Task 6: Update flow_io.rs — remove LoadedFlow.nodes

**Files:** `flowedit/src/flow_io.rs`

- Remove `nodes: Vec<NodeLayout>` from `LoadedFlow`
- Remove `build_node_layouts()` call from `load_flow()` — process_refs and subprocesses are already in the FlowDefinition
- Remove `build_node_layouts()` function
- Update `perform_open`: remove `win.nodes = loaded.nodes`
- Update `perform_new`: remove `win.nodes = Vec::new()`
- `generate_unique_alias`: take `&[ProcessReference]`
- `next_node_position`: take `&[ProcessReference]`

## Task 7: Update remaining files

**Files:** `flowedit/src/initializer.rs`, `flowedit/src/library_mgmt.rs`

- `sync_flow_definition()`: this function syncs NodeLayout positions back to ProcessReference — it becomes unnecessary since we write to ProcessReference directly. Remove it.
- `apply_initializer_state()`: modify `flow_definition.process_refs` directly (already does this for the model side; remove the NodeLayout display side)
- `add_library_function()`: add ProcessReference + resolved Process to subprocesses, no NodeLayout

## Task 8: Remove NodeLayout struct and update WindowState

**Files:** `flowedit/src/window_state.rs`, `flowedit/src/canvas_view.rs`

- Remove `nodes: Vec<NodeLayout>` from WindowState
- Remove `NodeLayout` struct, `Default` impl, `build_node_layouts()`
- Remove `PortInfo` if no longer used (check FunctionViewer first — that's Sub-plan C)

## Task 9: Update ui_test.rs

Update all tests: replace `win.nodes` with `win.flow_definition.process_refs`, replace NodeLayout construction with ProcessReference construction, update assertions.

## Task 10: Final cleanup, fmt, clippy, test
