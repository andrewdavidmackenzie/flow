# Single FlowDefinition Owner Architecture

Issue: #2602

## Problem

Each editor window currently owns an independent copy of its `FlowDefinition`.
Edits in one window don't propagate to others. When a sub-flow or function is
opened in a child window, a fresh copy is loaded from disk, creating divergent
state. Manual propagation helpers exist but are fragile and incomplete.

## Design

### Data Ownership

`FlowEdit` owns the single root `FlowDefinition`, which contains the complete
hierarchy of nested flows and functions via `subprocesses: BTreeMap<Name, Process>`.
This is the single source of truth for all editing windows.

`WindowState` stores a `Route` identifying which `Process` in the hierarchy it
renders. It does NOT own any flow data. It retains only UI state: canvas zoom/scroll,
selection, undo history, tooltip, context menu, metadata panel visibility, and
window size/position.

### Addressing via Route

Windows address their target using flowcore's `Route` type (e.g., `/root/parser/tokenizer`).
Each `FlowDefinition` and `FunctionDefinition` already has a `route` field populated
by the parser.

`FlowDefinition::process_from_route(&self, route: &Route) -> Option<&Process>` and
its `_mut` variant walk the `subprocesses` tree by matching route segments against
subprocess aliases. These methods are implemented in flowcore.

### Edit Flow

1. Window sends an edit message containing the target `Route` and the edit operation.
   Messages are self-contained and independent of the originating window.
2. `FlowEdit::update()` receives the message, uses `process_from_route_mut()` to find
   the target `Process`, and applies the mutation.
3. `FlowEdit::update()` marks the target route as dirty (for save tracking).
4. `FlowEdit::update()` notifies all windows whose stored `Route` is affected
   (equal to or a sub-route of the changed route) to redraw.
5. Windows call `borrow()` / read the data via their `Route` during `view()`.

### Window Lifecycle

**Opening a sub-flow or function:** Create a new `WindowState` with the target
`Route`. No file loading — the data already exists in the root hierarchy from
the initial parse. The window renders by looking up its `Route` in the hierarchy.

**Closing a window:** Remove the `WindowState`. No data effect — the hierarchy
is unaffected.

**Deleting a process:** Before removing a process from the hierarchy, iterate
all open windows and close any whose stored `Route` is equal to or a sub-route
of the deleted process's route. Then remove the process.

### Undo/Redo

Per-window. Each `WindowState` maintains its own `EditHistory`. Undo/redo is
triggered by keyboard shortcut or button on the focused window and only affects
edits made from that window. Undo sends a reverse-edit message to `FlowEdit::update()`
following the same flow as any other edit.

### Saving

Track modified files by maintaining dirty flags per `Route`. When saving,
iterate all dirty routes and serialize each `FlowDefinition` or `FunctionDefinition`
to its file (determined by `source_url`).

Serialization methods for `FlowDefinition` and `FunctionDefinition` need to be
added to flowcore, moving logic currently in flowedit's `file_ops.rs`.

### Data Flow Diagram

```text
Window A (Route: /root)        Window B (Route: /root/parser)
     |                              |
     | edit msg + Route             | edit msg + Route
     v                              v
+--------------------------------------------------+
|                FlowEdit::update()                 |
|                                                   |
|  process_from_route_mut(route) -> &mut Process    |
|  apply mutation                                   |
|  mark route dirty                                 |
|  notify affected windows                          |
+--------------------------------------------------+
                      |
                      v
            FlowEdit::view()
                      |
        +-------------+-------------+
        |                           |
        v                           v
   Window A                    Window B
   uses &root_flow             process_from_route(
   directly (root window)        &root, /root/parser)
   -> render root flow         -> render sub-flow
```

## Prerequisites

1. `process_from_route` / `process_from_route_mut` in flowcore — **done**
2. Serialization methods for `FlowDefinition` and `FunctionDefinition` in flowcore
3. Refactor edit messages to carry `Route` instead of relying on window context

## Migration Strategy

This is a large architectural change. Suggested order:

1. Add serialization to flowcore (prerequisite 2)
2. Add `Route` to `WindowState`, populate alongside existing `flow_definition`
3. Refactor edit messages to carry `Route`
4. Move edit logic from `WindowState` methods to `FlowEdit::update()`
5. Remove `flow_definition` from `WindowState`, use `process_from_route` in `view()`
6. Implement cascade-close on deletion
7. Implement dirty-flag save tracking
8. Remove file loading from child window creation
