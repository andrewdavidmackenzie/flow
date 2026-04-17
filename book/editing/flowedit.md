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
with connection lines drawn between them as bezier curves.

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

Ports are discovered from two sources:
1. **Input initializers** defined on the process reference (e.g., `input.start = {once = 1}`)
2. **Connections** in the flow definition that reference the node's ports

### Input Initializers

Input ports that have initializer values show:
- A **yellow** circle (instead of white) indicating the port has an initial value
- The initializer value and type displayed to the left of the port in yellow text

Initializer types:
- **once** — the value is provided once at flow startup (e.g., `1 once`)
- **always** — the value is provided every time the function runs (e.g., `0 always`)

## Connections

Connections between nodes are drawn as smooth bezier curves from an output port on
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

The cursor changes to a grab hand when hovering over a node, and a grabbing hand
while dragging.

### Deleting Nodes

Select a node and press **Delete** or **Backspace** to remove it from the flow.
All connections to and from the deleted node are also removed.

### Creating Connections

Click and drag from any port (input or output) to start creating a connection.
A green bezier curve preview follows the cursor as you drag. Compatible target
ports highlight when you hover over them.

- **Output → Input**: drag from a right-side port to a left-side port
- **Input → Output**: drag from a left-side port to a right-side port (the
  connection direction is determined automatically)

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

- **Ctrl + mouse wheel** (Cmd on macOS) — zoom in/out centered on cursor
- **Zoom controls** — floating buttons in the bottom-right corner:
  - **+** — zoom in
  - **−** — zoom out
  - **Fit** — auto-fit all nodes in the visible area with padding

The zoom range is 10% to 500%. The current zoom level is shown in the status bar
when zooming.
