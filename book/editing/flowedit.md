# Visual Flow Editor — `flowedit`

`flowedit` is a visual editor for creating, editing, and viewing flow definition files.
It provides a canvas-based WYSIWYG interface where flows are represented as connected
process nodes.

## Launching

```bash
# Open an existing flow
flowedit path/to/root.toml

# Start with an empty canvas
flowedit
```

The flow definition file can be in TOML, YAML, or JSON format.

## User Interface

The editor window has three main areas:

```text
┌──────────────────────────────────────────────┐
│  Menu bar                                     │
├──────────────────────────────────────────────┤
│                                               │
│              Canvas Area                      │
│                                               │
│   Nodes and connections are displayed here    │
│                                               │
├──────────────────────────────────────────────┤
│  Status bar                                   │
└──────────────────────────────────────────────┘
```

### Canvas Area

The main editing area where flow process nodes and their connections are displayed.
When a flow is loaded, each process appears as a colored rounded rectangle (a "node")
with connection lines drawn between them as Bézier curves.

### Status Bar

Shows the current editor state:
- **Ready** — default state, the flow can be edited
- **Compiling** — the flow is being compiled (future feature)
- **Running** — the flow is executing (future feature)

The status bar also shows contextual information such as the name of a selected node
or the number of nodes and connections loaded.

## Nodes

Each process in the flow is rendered as a rounded rectangle on the canvas.

### Node Colors

Nodes are color-coded by their process source type:

| Color | Process Type | Example |
|-------|-------------|---------|
| Blue | Library function | `lib://flowstdlib/math/add` |
| Green | Context function | `context://stdio/stdout` |
| Orange | Nested flow | Another flow definition |
| Purple | Provided implementation | Custom code compiled to WASM |

### Node Labels

Each node displays:
- **Title** — the process alias (e.g., `add1`) or a short name derived from the source
  URL if no alias is set (e.g., `lib://flowstdlib/math/sequence` displays as `sequence`)
- **Source** — the source URL, truncated with ellipsis if too long

### Ports

Ports are the connection points on the edges of a node:

- **Input ports** — small circles on the **left** edge, labeled with the port name
- **Output ports** — small circles on the **right** edge, labeled with the port name

Ports are resolved from the actual function or flow definition by loading
each subprocess source. This provides real port names and data types.

### Input Initializers

Input ports that have initializer values show:
- A **yellow** circle (instead of white) indicating the port has an initial value
- The initializer value and type displayed to the left of the port in yellow text

Initializer types:
- **once** — the value is provided once at flow startup (e.g., `once: 1`)
- **always** — the value is provided every time the function runs (e.g., `always: 0`)

**Right-click** on an input port to edit its initializer. A dialog appears with:
- A dropdown to select the type (none, once, always)
- A JSON value field (shown only when type is not "none")
- Apply and Cancel buttons

### Resizing Nodes

When a node is selected, 8 yellow resize handles appear on its edges and corners.
Drag any handle to resize the node. Edge handles resize one dimension, corner
handles resize both.

## Connections

Connections between nodes are drawn as smooth Bézier curves from an output port on
one node to an input port on another node. Each connection has a filled arrow head
at the destination (input) end to indicate the direction of data flow.

Self-connections (loopbacks) are routed around the outside of the node box so they
are clearly visible.

### Connection Names

Connections can have optional names defined in the flow definition:

```toml
[[connection]]
name = "next value"
from = "add"
to = "stdout"
```

Named connections display their name along the connection curve — above the line
for normal connections, and below the box for loopback connections.

### Port Types

When the editor can resolve function definitions from installed libraries, real
port names and data types are displayed. Port types are stored for future hover
tooltip display.

## Interactions

### Selecting Nodes

Click on a node to select it. The selected node is highlighted with a yellow border
and its name appears in the status bar. Click on empty canvas to deselect.

### Moving Nodes

Click and drag a selected node to reposition it on the canvas. All connections to
and from the node are automatically redrawn as the node moves.

The cursor changes contextually:
- **Grab hand** over nodes (draggable)
- **Grabbing hand** while dragging a node or panning
- **Crosshair** over ports or while connecting
- **Directional resize arrows** over resize handles
- **Pointer** over the pencil icon on openable nodes

Hover over a node with a truncated source label to see the full path in a tooltip.

### Deleting Nodes

Select a node and press **Delete** or **Backspace** to remove it from the flow.
All connections to and from the deleted node are also removed.

### Creating Connections

Click and drag from any port (input or output) to start creating a connection.
A green Bézier curve preview follows the cursor as you drag.

- **Output → Input**: drag from a right-side port to a left-side port
- **Input → Output**: drag from a left-side port to a right-side port (the
  connection direction is determined automatically)

While dragging, **compatible target ports** are highlighted with a green circle.
Port type compatibility is checked — if both ports have type information, at
least one type must match. Ports with unknown types accept any connection.

Release the mouse on a valid target port to complete the connection. Release on
empty canvas or an incompatible port to cancel.

The cursor changes to a crosshair when hovering over ports.

### Selecting Connections

Click near a connection line to select it. The selected connection is highlighted
in yellow with a thicker stroke. Click on empty canvas to deselect.

Selecting a connection deselects any selected node, and vice versa.

### Deleting Connections

Select a connection and press **Delete** or **Backspace** to remove it.

## Layout

### Saved Layout

Process positions can be saved in the flow definition file using optional layout
fields on each `[[process]]` entry:

```toml
[[process]]
alias = "add1"
source = "lib://flowstdlib/math/add"
input.i2 = {always = 1}
x = 300
y = 100
width = 180
height = 120
```

The layout fields (`x`, `y`, `width`, `height`) are ignored by the flow compiler
(`flowc`) and are only used by `flowedit` for visual positioning. Both integer and
floating-point values are accepted (e.g., `x = 100` or `x = 100.0`).

### Auto Layout

When a flow is loaded without saved positions, nodes are automatically arranged
following the connection topology: source nodes (with no incoming connections) are
placed on the left, and each downstream node is placed one column to the right.
Nodes are spread vertically within each column. The view auto-fits to show all
nodes on initial load.

## Zoom and Scroll

### Scrolling

- **Mouse wheel** — scrolls the canvas vertically and horizontally
- **Middle-mouse-button drag** — pans the canvas freely

### Zooming

- **Cmd + mouse wheel** (Ctrl on Linux) — zoom in/out centered on cursor
  (without modifier, scroll wheel pans the canvas)
- **Zoom controls** — floating buttons in the bottom-right corner:
  - **+** — zoom in
  - **−** — zoom out
  - **Fit** — auto-fit all nodes in the visible area with padding

The zoom range is 10% to 500%. The current zoom level is shown in the status bar
when zooming.

The **Fit** button is a toggle: when active (blue), the view continuously
adjusts to fit all nodes as the window is resized. Manual zoom or pan
automatically disables auto-fit.

## Undo / Redo

- **Cmd+Z** — undo the last edit (move, resize, delete, create/delete connection)
- **Cmd+Shift+Z** — redo the last undone action

The status bar shows the current unsaved edit count. New edits clear the
redo history.

## File Operations

- **Cmd+S** — save to the current file (or prompt with Save As if new)
- **Cmd+Shift+S** — save to a new file (always prompts)
- **Cmd+O** — open a flow file
- **Cmd+N** — create a new empty flow

The window title shows the filename and a `*` when there are unsaved edits.

## Process Library

The left panel shows available processes organized in a collapsible tree:

- **Context** — runtime context functions (stdio, file, image, args)
- **flowstdlib** — the standard library (math, control, data, etc.)
- Any other libraries installed in `FLOW_LIB_PATH` or `~/.flow/lib`

Click a function name to add it as a new node on the canvas. The node is
placed to the right of existing nodes and auto-fit adjusts the view if enabled.
If a function with the same name already exists, the new node gets a unique
alias (e.g., `add_2`, `add_3`).

Each function also has a pencil icon (✎) that opens the function definition
in a viewer window, showing its TOML definition, Rust source, and documentation.

## Flow Hierarchy

Above the Process Library, a collapsible tree view shows the structure of the
loaded flow. The root flow is at the top, with child sub-flows and functions
as children, recursively.

- **Orange** nodes are flows — click to expand/collapse, pencil icon to open
- **Purple** nodes are provided implementations — click to open in editor
- **Blue** nodes are library/context functions (display only)

## Creating New Processes

New sub-flows and functions can be created from:
- **Toolbar buttons**: "+ Sub-flow" and "+ Function" in the status bar
- **Right-click context menu**: right-click on empty canvas for the same options

Creating a new sub-flow prompts for a filename, writes an empty flow TOML,
adds a node to the canvas, and opens the sub-flow in a new editor window.

Creating a new function prompts for a filename, adds a node to the canvas,
and opens the function definition editor. Clicking Save generates the
function TOML, skeleton Rust source, and Cargo manifest.

## Opening Sub-flows and Functions

Nodes that represent sub-flows (nested `.toml` files) or provided implementations
(custom `.rs` functions) display a pencil icon (✎) in the top-right corner. The
cursor changes to a pointer when hovering over the icon.

### Sub-flow Windows

Clicking the pencil on a sub-flow node opens it in a new editor window within
the same process. The sub-flow window shows:

- A **rounded bounding box** around all subprocess nodes
- **Flow input ports** as blue semicircles on the left edge of the box
- **Flow output ports** as orange semicircles on the right edge of the box
- **Bezier connections** from the flow I/O ports to the internal subprocess ports

Below the canvas, an I/O editor panel allows editing the flow's declared
inputs and outputs:
- **+ Input** / **+ Output** buttons to add new ports
- Editable name and type fields for each port
- **✕** buttons to delete ports
- Changes are reflected in the canvas bounding box semicircles

Sub-flow windows do not show the Build button (only the root flow can be compiled).
Clicking the pencil icon on an already-open sub-flow brings the existing window
to the front instead of opening a duplicate.

### Function Definition Editor

Clicking the pencil on a provided implementation node opens a function definition
editor showing:

- **Editable function name** centered at the top
- **Input ports** on the left with editable name and type fields
- **Output ports** on the right with editable name and type fields
- **+** buttons to add new input or output ports
- **✕** buttons to delete existing ports
- **Source file link** — click the filename to view the Rust source code,
  or click "..." to browse for a different source file
- **Docs tab** — if a `.md` documentation file exists alongside the function

The **💾 Save** button writes the function definition to disk:
- Updates the `.toml` definition file with the current name, inputs, and outputs
- Generates a skeleton `.rs` source file if one doesn't exist (with the
  `#[flow_function]` boilerplate and correct input bindings)
- Generates a `function.toml` Cargo manifest if one doesn't exist

## Compiling

Click the **🔨 Build** button in the status bar (or press **Cmd+B**) to compile
the current flow to a manifest. The flow must be saved before compiling — if the
flow has never been saved, a Save As dialog appears first. Any unsaved edits are
automatically saved before compilation.

The compiled manifest is written to the same directory as the flow file.

## Metadata Editor

Click the **ℹ Info** button in the toolbar to toggle the metadata editor panel.
It shows editable fields for:

- **Name** — the flow name (also updates the window title)
- **Version** — semantic version string
- **Description** — human-readable description
- **Authors** — comma-separated list of author names

Changes are saved when the flow is saved (Cmd+S).

## Library Paths

Click the **📁 Libs** button in the toolbar to toggle the library search paths
panel. It shows the current library directories from `FLOW_LIB_PATH` and
`~/.flow/lib`.

- **+ Add Path...** — opens a folder picker to add a new library directory
- **✕** — removes a path from the search list

Adding or removing a path rescans the Process Library panel immediately.

## Window Position Persistence

Window size and position are saved to a sidecar file (`.filename.flowedit`)
alongside the flow TOML when saving. The next time the same flow is opened,
the window reopens at the saved size and position.

## Window Management

`flowedit` uses multi-window support — sub-flows and functions open in separate
windows within the same application process.

- **Cmd+W** — close the currently focused window
- **Cmd+Q** — quit the entire application (prompts to save if there are unsaved changes)
- Closing the root window exits the entire application
- Closing a child window only closes that window

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+Z | Undo |
| Cmd+Shift+Z | Redo |
| Cmd+S | Save |
| Cmd+Shift+S | Save As |
| Cmd+O | Open |
| Cmd+N | New |
| Cmd+B | Build (compile) |
| Cmd+W | Close window |
| Cmd+Q | Quit all |
| Cmd+= | Zoom in |
| Cmd+- | Zoom out |
| Delete / Backspace | Delete selected node or connection |
