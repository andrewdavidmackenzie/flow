# flowedit — Visual Flow Editor: Analysis & Design

## 1. Overview

A new `flowedit` binary that provides a WYSIWYG canvas-based editor for creating, editing,
and running flow programs. Built on iced 0.14 with the Canvas widget for custom node/connection
rendering. Shares code with existing libraries (flowcore, flowrlib, flowstdlib) and reuses
patterns from flowrgui.

## 2. UI Layout

```text
┌─────────────────────────────────────────────────────────────────────┐
│  File  Edit  Flow  Help                                      flowedit │
├─────────────┬───────────────────────────────────────────────────────┤
│             │                                                       │
│  Process    │              CANVAS                                   │
│  Library    │                                                       │
│             │    ┌──────────┐        ┌──────────┐                   │
│  flowstdlib/│    │ sequence │        │  stdout  │                   │
│    math/    │    │          │        │          │                   │
│      add    │    │ ○ start  ├───────→│ ○ default│                   │
│      sub    │    │ ○ step   │number  │          │                   │
│      mul    │    │ ○ limit  │        └──────────┘                   │
│    control/ │    └──────────┘                                       │
│      ...    │                                                       │
│  mylib/     │                                                       │
│    filters/ │                                                       │
│      ...    │                                                       │
│  context/   │                                                       │
│    stdio/   │                                                       │
│      stdin  │                                                       │
│             │                                                       │
├─────────────┴───────────────────────────────────────────────────────┤
│  ┌─────────┬─────────┬─────────┬─────────┐                         │
│  │ Stdout  │ Stderr  │ Stdin   │ FileIO  │  (tabs, reused from     │
│  ├─────────┴─────────┴─────────┴─────────┤   flowrgui TabSet)      │
│  │ output appears here during execution   │                         │
│  └────────────────────────────────────────┘                         │
├─────────────────────────────────────────────────────────────────────┤
│  Status: Ready | Compiling | Running                                │
└─────────────────────────────────────────────────────────────────────┘
```

### Layout Description

- **Menu bar** — File (New, Open, Save, Save As), Edit (Undo, Redo, Delete), Flow (Compile,
  Run, Stop), Help
- **Left panel: Process Library** — tree view of all available processes discovered from
  libraries found on `FLOW_LIB_PATH` (or specified via `-L` CLI option), grouped by library.
  Context functions are treated as a library. User can drag a process onto the canvas to add it.
- **Center: Canvas** — the main editing area where flow nodes are drawn and connected
- **Bottom panel: I/O Tabs** — reused from flowrgui's TabSet (stdout, stderr, stdin, fileio,
  images) for flow execution output
- **Status bar** — current state: Ready (editing), Compiling, or Running
  (Running may have future sub-states like Paused via debugger).
  Errors are displayed as messages, not a state.

## 3. Node Rendering

Each process on the canvas is rendered as a rounded rectangle ("bubble"):

```text
         ┌─────────────────────────┐
         │       sequence          │
         │                         │
  1 once ○  start          number  ○
  1 once ○  step                   │
 20 once ○  limit                  │
         │                         │
         └─────────────────────────┘
         ↑                         ↑
    Input ports with             Output port
    initializer values          (right edge)
    (left edge)
```

### Node Details

- **Title**: process alias (or short name derived from source if no alias) centered at top
- **Input ports**: small circles on the left edge, labeled with port name
- **Input initializers**: displayed next to the port as `value type` (e.g., `1 once` or
  `0 always`). Clicking an initializer in edit mode opens a field to change the value.
- **Output ports**: small circles on the right edge, labeled with port name
- **Hover behavior**: hovering over a port shows a tooltip with the port's data type(s)
- **Selection**: clicking a node selects it (highlighted border). Selected nodes can be
  moved, resized, or deleted.
- **Resize**: drag handles on corners/edges of the node rectangle
- **Colors**: different fill colors for different process types:
  - Library functions: blue
  - Context functions: green
  - Nested flows: orange
  - Provided implementations (custom code compiled to WASM): purple

### Nested Flow Nodes

When a process references another flow definition, it appears as a node with a double border.
Double-clicking it opens that flow in a new editor window/tab. In the nested editor:
- Flow inputs appear as port connectors on the left edge of the canvas
- Flow outputs appear as port connectors on the right edge of the canvas

## 4. Connection Rendering & Interaction

Connections are drawn as bezier curves between output and input ports:

```text
    ┌──────────┐                    ┌──────────┐
    │ sequence │                    │  stdout  │
    │          │                    │          │
    │   number ○───╮          ╭───→○ default  │
    │          │    ╰────────╯     │          │
    └──────────┘                    └──────────┘
```

### Connection Drawing

- Bezier curve with control points calculated automatically based on horizontal distance
  between ports (similar to iced bezier_tool example)
- Connection color: default gray, highlighted on hover/selection
- Arrow head on the destination (input) end

### Connection Interaction

1. **Creating**: Click an output port → line follows cursor as bezier curve → release on
   compatible input port. Can also start from input port and drag to output port.
2. **Compatibility**: When dragging, only type-compatible ports are highlighted and
   selectable. Incompatible ports are dimmed. Type checking must reuse the existing
   `flowc` compiler code (e.g., `DataType::compatible_types()` in flowcore, and
   connection validation in flowrclib) — not replicate it. This may require exposing
   additional public API functions in flowcore/flowrclib for the editor to call.
3. **Reconnecting**: Click and drag an existing connection endpoint to move it to a different
   compatible port.
4. **Selecting**: Click a connection line to select it (highlighted).
5. **Deleting**: Select a connection, then press Delete or use Edit menu.

## 5. Data Model

### EditorFlow — the in-memory representation

```rust
struct EditorFlow {
    definition: FlowDefinition,    // from flowcore — includes layout in ProcessReferences
    dirty: bool,                   // unsaved changes
    file_path: Option<PathBuf>,    // where this flow is saved
}
```

### Persistence — layout in flow definition

Layout metadata is stored as optional fields in the `[[process]]` entries of the flow
definition file:

```toml
[[process]]
alias = "sequence"
source = "lib://flowstdlib/math/sequence"
input.start = { once = 1 }
input.limit = { once = 9 }
# Visual layout (used by flowedit, ignored by flowc)
x = 100.0
y = 200.0
width = 180.0
height = 120.0
```

This requires adding optional `x`, `y`, `width`, `height` fields (as `f32`) to
`ProcessReference` in flowcore. The compiler (`flowc`) will parse but ignore these fields.
The `#[serde(default, skip_serializing_if)]` pattern ensures they're only written when
present. Note: serde/TOML auto-converts integers to f32, so users can write `x = 100`
or `x = 100.0` — both work. This should be a test case.

## 6. Process Library Panel

The left panel shows available processes in a tree structure, organized by library.

### Library Discovery

Libraries are discovered by scanning the **flow lib search path** (`FLOW_LIB_PATH` or
`~/.flow/lib` by default). Each directory in the path that contains a `lib.json` manifest
is a library. `flowstdlib` is not special-cased — it's just another library found via this
path. If it's not installed, its functions simply won't appear.

### Tree Structure

```text
Libraries
├── flowstdlib/
│   ├── math/
│   │   ├── add
│   │   ├── subtract
│   │   ├── multiply
│   │   └── sequence
│   ├── control/
│   │   ├── select
│   │   └── tap
│   ├── data/
│   │   ├── accumulate
│   │   └── append
│   └── ...
├── mylib/                    (user-installed library)
│   ├── filters/
│   │   └── lowpass
│   └── transforms/
│       └── fft
└── Context/
    └── stdio/
        ├── stdin
        ├── stdout
        └── stderr
```

Each library is loaded from its manifest, which describes its available functions/flows
with their input/output signatures. Context functions are discovered from the runner
specification.

The tree has collapsible/expandable branches (click to toggle). The entire panel is
in a scrollable view with both horizontal and vertical scroll bars for large library
trees.

The main canvas area is also in a scrollable view with horizontal and vertical scroll
bars, allowing flows larger than the visible area to be panned.

### Interaction

The user can:
- Browse the tree
- Search/filter by name
- Drag a process onto the canvas to add it
- Double-click to add it at a default position

When a process is added, a new `ProcessReference` is created in the flow definition with
the appropriate `lib://` or `context://` source URL and default layout coordinates.

## 7. Architecture & Code Reuse

### New crate structure

```
flowr/src/bin/flowedit/
├── main.rs              # Application entry, CLI args, iced::application builder
├── editor.rs            # Main editor state, update/view
├── canvas_view.rs       # Canvas widget — node/connection rendering & interaction
├── library_panel.rs     # Process library tree view
├── menu.rs              # Menu bar
└── model.rs             # EditorFlow, NodeLayout, editor-specific data types
```

### Reuse from existing code

| Component | Source | How |
|-----------|--------|-----|
| I/O Tabs (stdout, stderr, etc.) | flowrgui/tabs.rs | Import directly or extract to shared module |
| Flow execution | flowrlib (Coordinator, Dispatcher, Executor) | Same as flowrgui |
| Flow compilation | flowrclib (compile, parser) | Use as library |
| Flow definition model | flowcore (FlowDefinition, etc.) | Direct dependency |
| Coordinator connection | flowrgui/connection_manager.rs | Reuse pattern |
| Type compatibility | flowcore/model/datatype.rs | `DataType::compatible_types()` |
| Serialization | flowcore/deserializers | TOML/YAML/JSON roundtrip |

### Changes to existing code

1. **flowcore: ProcessReference** — add optional `x`, `y`, `width`, `height` fields with
   serde defaults
2. **flowcore: FlowDefinition** — no changes needed, processes already stored by alias
3. **flowr/Cargo.toml** — add `[[bin]] name = "flowedit"` entry

## 8. Implementation Phases

### Phase 1: Canvas with static nodes (2-3 days)

**Goal**: Render a hardcoded flow as nodes on a canvas. No interaction yet.

**Work**:
- Create `flowedit` binary skeleton with iced::application
- Implement canvas rendering: rounded rectangles for nodes, circles for ports, labels
- Load a flow definition file (e.g., `hello-world/root.toml`) and render it
- Draw connections as bezier curves between ports

**Tests**: Canvas renders without crashing. Flow definition loads correctly. Visual
verification by running the editor.

**Deliverable**: A read-only flow viewer.

### Phase 2: Node interaction (2-3 days)

**Goal**: Select, move, and resize nodes on the canvas. Intelligent auto-layout.

**Work**:
- Hit testing for nodes and ports (point-in-rectangle, point-near-circle)
- Node selection (click to select, click canvas to deselect)
- Node dragging (click and drag to move, connections follow)
- Node deletion (select + delete key)
- Zoom and pan on the canvas
- Intelligent auto-layout for flows without saved positions: arrange nodes
  left-to-right following connection topology (sources on left, sinks on right)
  rather than a simple grid

**Tests**: Node positions update after drag. Connections redraw correctly. Delete removes
node from flow definition.

**Deliverable**: Interactive flow viewer with moveable nodes.

### Phase 3: Connection creation (2-3 days)

**Goal**: Create and delete connections between ports by dragging.

**Work**:
- Port hit testing (identify which port is under cursor)
- Connection drag interaction (bezier curve from port to cursor)
- Type compatibility highlighting (compatible ports glow, others dim)
- Connection completion (release on compatible port creates connection)
- Connection selection and deletion
- Connection reconnection (drag endpoint to new port)

**Tests**: Connection created between compatible ports. Connection rejected between
incompatible ports. Connection appears in flow definition. Deletion removes from definition.

**Deliverable**: Full connection editing.

### Phase 4: Process library & adding nodes (2-3 days)

**Goal**: Browse available processes and add them to the canvas.

**Work**:
- Left panel with tree view of flowstdlib functions and context functions
- Drag from library to canvas to add a process
- Process added to flow definition with default inputs
- Input initializer editing (click on port to set once/always value)

**Tests**: Process added to canvas and flow definition. Library shows all stdlib functions.
Initializer values saved correctly.

**Deliverable**: Can create flows from scratch using library functions.

### Phase 5: Save/Load & compile/run

**Status**: Save/Load/Open/New completed in PR #2569. Remaining: compile and run.

**Remaining work** (for follow-up PR):
- Flow menu: Compile (using flowc as library) — compile the current flow definition
  and show errors in the status bar or a dialog
- Flow menu: Run (using flowrlib) — execute the compiled flow
- Bottom I/O tabs for execution output (reuse from flowrgui's TabSet)
- Input initializer editing: click on a port to set once/always value via a popup
- Type compatibility checking during connection creation: highlight only compatible
  ports using flowcore's `DataType::compatible_types()`

- Extend process library panel: allow adding custom library paths via CLI (-L)
  or a UI dialog. When opening a flow with provided implementations (custom
  .rs/.wasm files), automatically add them to the library panel so they can
  be reused in other flows.

**Tests**: Compile succeeds for valid flow. Run produces expected output.

**Deliverable**: Fully functional flow editor that can create, save, compile, and run flows.

### Phase 6: Nested flows, new processes & polish

**Step 1: Click the pencil icon on a sub-flow to open nested editor**
- Click the pencil icon on a node whose source is a relative `.toml` (sub-flow) to open it
  in a new editor window
- Demo: `mandlebrot/root.toml` → double-click `generate_pixels` or `render`
- Update user manual

**Step 2: Nested flow editor with input/output editing**
- Nested editor shows the flow's declared `[[input]]` ports on the left edge
  and `[[output]]` ports on the right edge as visual anchors (not process boxes)
- Provide UI to add, edit (name, type), and delete flow inputs and outputs
- Demo: open a sub-flow, modify its inputs/outputs, save, confirm parent
  still compiles
- Update user manual

**Step 3: Click the pencil icon on a provided implementation to view/edit**
- Click the pencil icon on a node whose source is a `.rs` file (provided implementation)
  to open the function definition editor:
  - Tab 1: TOML definition (function name, inputs, outputs, source)
  - Tab 2: Rust source (`.rs` file)
  - Tab 3: Documentation (`.md` file, if it exists)
- Allow editing inputs/outputs (name, type) in the TOML tab
- Demo: `reverse-echo/root.toml` → double-click `reverse`; also try a
  flowstdlib function like `add` which has all three files
- Update user manual

**Step 4: Create new sub-flow**
- User specifies: flow name, inputs (name + type), outputs (name + type),
  and selects a filename/location for the new `.toml`
- Save the new sub-flow `.toml` with declared inputs/outputs
- Add a `source = "relative/path.toml"` process reference in the parent flow
- Open the new sub-flow in a nested editor
- Demo: create a sub-flow from scratch in a test flow
- Update user manual

**Step 5: Create new provided implementation**
- User specifies: function name, inputs (name + type), outputs (name + type),
  and selects a filename/location
- Create the function `.toml` definition and a skeleton `.rs` source file
  (with `#[flow_function]` boilerplate, correct input count, imports)
- Allow editing inputs/outputs (name, type) — same UI as step 3
- Add a `source = "relative/path"` process reference in the parent flow
- Demo: create a new function from scratch, view the generated skeleton
- Update user manual

**Step 6: Flow metadata editing**
- Edit flow name, description, authors on any flow
- Update user manual

**Tests**: Nested flow can be edited and saved. New sub-flows and provided
implementations can be created with correct definitions. Undo/redo works
for node moves, connection changes, and process additions. Metadata saved
correctly. Provided implementation skeletons compile.

**Deliverable**: Production-ready flow editor.

### Phase 7: Polish & reliability (TBD)

**Goal**: Fix known issues and improve reliability.

**Work**:
- Revisit connection hit testing — current approach of sampling points along the
  curve is unreliable for certain curve geometries (especially loopback connections
  from unnamed output ports). Consider alternative approaches: distance-to-curve
  calculation, wider hit zones, or rendering connections to an off-screen buffer
  for pixel-precise hit testing.
- Root flow background: draw the canvas background as a slightly different color
  with a rounded border to visually suggest the root flow is itself a process,
  just the one where we start viewing/editing
- Canvas scroll bars: add visible horizontal and vertical scroll bars to the
  main editing canvas (in addition to the existing mouse-wheel/middle-button pan)
- Revisit hover tooltip: improve positioning (track cursor more closely), ensure
  text size matches the source label, and extend to show port data types on hover
- Port inset so semi-circles sit inside the box border without breaking connection
  endpoint alignment
- Save/restore editor window position and size in the flow definition (e.g., as
  optional x, y, width, height fields on the root flow metadata) so the window
  reopens at the same position when editing is resumed
- Prompt to save on window close (Cmd+Q or close button) when there are unsaved edits
- Fix alias-vs-connection-route key mismatch: when a ProcessReference has no explicit
  alias, the display alias is derived from the source URL, but connection routes may
  use a different name. Need to align alias resolution with flowc's logic so port
  lookups and connection drawing always match.
- Add UI dialog to add custom library search paths or specific libraries at runtime
  (the `-L` CLI flag already supports this at startup, but there is no in-app way
  to add paths during an editing session)
- Implement CLI options for loading, compiling and running a flow compatible with
  flowrgui, to enable using the same automated tests for both editors
- Flow hierarchy navigator panel: add a collapsible tree view above the Process
  Library panel showing the structure of the loaded flow — root flow at the top,
  child sub-flows and functions as children, recursively. Double-click to open
  an editing window for that node. For library-defined functions/flows (lib://),
  open a read-only view window similar to the existing editor windows.
- Library function/flow viewer and editor: right-click (or view icon) on a
  library function in the Process Library panel to view its definition, source,
  and docs. Extend to full editing for library authoring — edit function/flow
  definitions, inputs/outputs, source files, and save changes back to the
  library directory.
- Automated UI testing for interactive features described in the user manual

## 9. Key Design Decisions

1. **New binary, not extending flowrgui** — keeps things clean, can replace flowrgui later
2. **Canvas widget for rendering** — full control over node/connection drawing
3. **Layout stored in flow definition** — optional fields on process references, single file
4. **Bezier curves for connections** — following iced bezier_tool example pattern
5. **Drag-to-connect interaction** — with type compatibility highlighting
6. **Phase-based implementation** — each phase produces working, testable software
7. **stdlib + context functions first** — custom implementations in a later issue
