# flowedit: Show Function/Flow Descriptions (Issue #2574)

## Overview

Add description display and editing to flowedit, covering three areas:
1. Tooltip on hover in the library/context palette panels
2. Two-zone tooltips on canvas nodes (source path vs. description)
3. Editable description field in the function/flow viewer window

This also includes an architectural refactor: replace ad-hoc filesystem scanning
with proper library/flow parsing using existing model types, and establish a
shared-ownership data architecture so all windows reference canonical definitions
rather than copying fields.

Depends on #2573 (optional description field on `FunctionDefinition`).

## 1. Data Model Changes

### FlowDefinition — add description field

Add `#[serde(default)] pub description: String` to `FlowDefinition` in
`flowcore/src/model/flow_definition.rs`, mirroring the existing field on
`FunctionDefinition`.

### ProcessReference — no changes

Descriptions belong to the definition (function or flow), not the reference.
`ProcessReference` is unchanged.

## 2. Shared Data Architecture

### Problem

flowedit currently duplicates data across UI structs: `FunctionEntry` copies
`name` and `source` strings, `NodeLayout` copies fields, `FunctionViewer` copies
fields. Edits in one place are not reflected elsewhere without explicit sync.

### Solution

Establish a single source of truth using shared ownership:

- **One `Arc<RwLock<FlowDefinition>>`** — the flow being edited, containing all
  `ProcessReference`s, resolved `subprocesses` (`FunctionDefinition`s and nested
  `FlowDefinition`s), and connections.
- **One `Arc<RwLock<LibraryManifest>>` per library** — flowstdlib, each context
  function set, any other installed library.
- **All windows** reference into these shared structures. No copies.

Parse flows and libraries using `flowrclib` parsing functions and the existing
`Provider` infrastructure, producing the canonical model types
(`FunctionDefinition`, `FlowDefinition`, `LibraryManifest`).

Note: `LibraryManifest` currently holds `locators` (URL to implementation path)
and `source_urls`, but not parsed `FunctionDefinition`/`FlowDefinition` objects.
It will need to be extended to also hold the parsed definitions for each
function/flow it catalogs, so that the UI can access descriptions (and other
definition fields) directly from the manifest without re-parsing.

### UI struct changes

- **Library/context panels** — reference into the loaded `LibraryManifest`
  definitions for name, source, description. No separate `description` field on
  `FunctionEntry`.
- **`NodeLayout`** — reference into the flow's resolved subprocess definitions
  for description, ports, etc.
- **`FunctionViewer`** — hold a reference to the `FunctionDefinition` or
  `FlowDefinition` it is editing.

### On edit

Write-lock the definition, mutate `.description` (or other fields), release. All
readers see the update on next render cycle.

### Cross-window sharing

All windows reference the same shared structures. Use `Arc<RwLock<>>` for safe
shared access across the multi-window architecture.

## 3. Library and Context Panels

### Unified panel code

Both the library panels and the context functions panel are structurally the
same: a tree view of functions/flows from a `LibraryManifest`. The same code
handles both, parameterized by:
- The `LibraryManifest` being displayed
- A read-only flag (context and library functions are read-only for now;
  provided implementations embedded in the flow are editable)

Multiple libraries can be loaded simultaneously — each gets its own collapsible
section backed by its own `LibraryManifest`.

### Tooltip on hover

When hovering over a function/flow name in the palette tree, show an iced
`Tooltip` widget with the description from the canonical definition.

- Read `.description` from the referenced `FunctionDefinition` or
  `FlowDefinition` in the manifest.
- No tooltip if description is empty.
- Tooltip position: `tooltip::Position::Bottom`.

## 4. Canvas Two-Zone Tooltips

### Current behavior

Hovering over a canvas node shows the full source path as a tooltip, but only
when the source label is truncated (longer than `MAX_SOURCE_CHARS`).

### New behavior

Two hit-test zones per node, checked with simple layered-box priority (inner
wins over outer):

1. **Source text box (inner)** — a bounding box around the source label text
   (positioned at `node.y + 34.0`, centered). If the cursor is inside this box,
   show a tooltip with the full source path. Always shown, not just when
   truncated.

2. **Node body (outer)** — the full node rectangle. If the cursor is inside the
   node but NOT inside the source text box, show a tooltip with the description
   from the resolved definition. No tooltip if description is empty.

### Implementation

Extend the hover detection in `canvas_view.rs` to check the source text
bounding box first. If the cursor is within it, emit a tooltip with the full
source path. Otherwise, if within the node rectangle, emit a tooltip with the
description. The source text position and approximate dimensions are already
known from the rendering code.

## 5. Editable Description in Function Viewer

### Current state

The `FunctionViewer` window has a `text_input` for the function name, followed
by input/output ports and a source file reference.

### New behavior

Add a `text_input` for the description below the name input, nearly full width
of the viewer box. Similar styling to the name input.

- Bound to the canonical definition's `.description` via the shared reference.
- Edits mutate the definition in place — immediately reflected in palette
  tooltips and canvas tooltips without saving.
- Library and context functions: description field is displayed but not editable
  (no `on_input` handler). Only provided implementations have editable
  descriptions.
- New message variant: `FunctionDescriptionChanged(window::Id, String)`.

### Saving

When the flow or function definition is saved to TOML, the description is
serialized as part of the definition. No extra save logic — `FunctionDefinition`
and `FlowDefinition` already have serde support for the `description` field.

## 6. Testing

### Unit tests

- `FlowDefinition` deserialization with and without `description` field (mirrors
  existing `FunctionDefinition` tests from #2573).
- Library panel tooltip: verify that an empty description produces no tooltip
  and a non-empty description produces one.
- Canvas hit-testing: verify inner box (source text) takes priority over outer
  box (node body).

### Manual tests

- Hover over functions in palette — tooltip shows description from library TOML.
- Hover over source text on canvas node — shows full source path.
- Hover over rest of node on canvas — shows description.
- Edit description in function viewer — verify palette and canvas tooltips update
  immediately without saving.
- Save flow — verify description persists in TOML and reloads correctly.
- Context functions — verify description tooltip appears but field is not
  editable in the viewer.

## Key Files

| Component | File |
|-----------|------|
| FunctionDefinition model | `flowcore/src/model/function_definition.rs` |
| FlowDefinition model | `flowcore/src/model/flow_definition.rs` |
| ProcessReference model | `flowcore/src/model/process_reference.rs` |
| LibraryManifest model | `flowcore/src/model/lib_manifest.rs` |
| Library panel | `flowedit/src/library_panel.rs` |
| Canvas rendering & hit-testing | `flowedit/src/canvas_view.rs` |
| Main app, tooltips, function viewer | `flowedit/src/main.rs` |
| flowrclib parser | `flowc/src/lib/compiler/parser.rs` |
