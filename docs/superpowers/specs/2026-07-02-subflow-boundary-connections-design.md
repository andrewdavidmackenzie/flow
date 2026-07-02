# Subflow Boundary Connections in SVG Diagrams

**Issue:** [#2890](https://github.com/andrewdavidmackenzie/flow/issues/2890)
**Date:** 2026-07-02

## Problem

When rendering a subflow's own SVG diagram, the flow's input and output ports
(declared via `[[input]]`/`[[output]]` in the TOML definition) are not shown.
Connections that reference `input/...` or `output/...` routes are silently
dropped because no matching node exists in the layout.

This means the internal wiring of a subflow — how data enters from the flow's
inputs, flows through internal processes, and exits via the flow's outputs — is
invisible in the diagram.

## Solution

Render a bounding box around the internal processes with the flow's input ports
on the left wall and output ports on the right wall, connected to internal
process ports.

## Design

### Rendering Order

`render_flow()` changes from:

1. Render internal nodes
2. Render connections
3. Render initializers
4. Compute document bounds

To:

1. Render internal nodes (unchanged)
2. Compute bounding box around all internal nodes (with padding)
3. If the flow has inputs or outputs, render the bounding box rectangle
4. Compute boundary port positions (inputs on left inner wall, outputs on
   right inner wall)
5. Render connections — resolve `input/...` and `output/...` routes from
   boundary port positions instead of node layouts
6. Render initializers (unchanged)
7. Expand document bounds to include the bounding box

### Bounding Box

A light-colored rounded rectangle surrounding all internal nodes:
- Fill: `#F0F0F0` (light gray)
- Border: `#CCCCCC` (subtle gray)
- Corner radius: same as node rects
- Padding: enough to accommodate port labels between the box edge and internal
  nodes

### Boundary Port Positions

- **Input ports** are placed on the inside of the left wall. They act as data
  sources (sending data rightward into internal nodes), so their connection
  anchor point is on their right side.
- **Output ports** are placed on the inside of the right wall. They act as data
  sinks (receiving data from internal nodes), so their connection anchor point
  is on their left side.
- Ports are vertically centered within the bounding box height, using the same
  `PORT_SPACING` constant as node ports.
- Port shapes reuse the existing semi-circle primitives from `shapes.rs`:
  - Boundary inputs use the `output_port` shape (faces right/inward)
  - Boundary outputs use the `input_port` shape (faces left/inward)

### Connection Routing

`render_connection()` currently splits routes via `split_route()` and looks up
the node in the `layouts` HashMap. For boundary connections:

- When `from_node == "input"`: resolve position from the boundary input port
  map (the port's right-side anchor)
- When `to_node == "output"`: resolve position from the boundary output port
  map (the port's left-side anchor)

The connection curves use the same Bézier edge rendering as inter-node
connections.

### Boundary Port Data Structure

A simple struct or pair of HashMaps mapping port name to (x, y) anchor position:

```rust
struct BoundaryPorts {
    inputs: HashMap<String, (f32, f32)>,   // port name → (x, y) anchor
    outputs: HashMap<String, (f32, f32)>,  // port name → (x, y) anchor
}
```

### Visual Styling

- Bounding box is visually distinct from process nodes (lighter, thinner border)
- Port labels use the same font and style as node port labels
- Boundary input labels are placed to the right of the port semi-circle
- Boundary output labels are placed to the left of the port semi-circle

### Scope

- Only `flowc/src/lib/graph/renderer.rs` has significant changes
- Minor additions to `flowc/src/lib/graph/shapes.rs` if new primitives needed
- Style constants may be added to `flowcore/src/graph/style.rs`
- No changes to the layout algorithm (`flowcore/src/graph/layout.rs`)
- No changes to the flow model

### When the Boundary Appears

The bounding box and boundary ports are rendered whenever the flow has at least
one `[[input]]` or `[[output]]` declaration. A valid subflow always has
connections between its ports and internal processes.

### Testing

- Existing SVG tests for flows without inputs/outputs are unaffected
- Flows with inputs/outputs (e.g., `sequence.toml`) will produce updated SVGs;
  expected files will need manual review and approval before replacement
- Layout unit tests in `flowcore` are unaffected
