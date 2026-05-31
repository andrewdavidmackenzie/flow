## Flow Hierarchy and Decomposition

A flow program is organized as a tree of **processes**. Each process is either
a **function** (which does computation) or a **sub-flow** (which groups other
processes together). The root flow sits at the top of the tree and can contain
any mix of functions and sub-flows. Sub-flows can in turn contain more
functions and sub-flows, to any depth.

This hierarchy is purely organizational. All computation happens in functions.
A sub-flow is just a way to group related functions, give them a name, and
define their connections — much like a subroutine in procedural programming,
but without sequential execution. The sub-flow boundary has no effect on
parallelism: the runtime flattens the hierarchy and executes all functions
based on data availability, regardless of which sub-flow they belong to.

### Example: the Mandelbrot Flow

The mandelbrot example illustrates a three-level hierarchy. The root flow
contains three sub-flows, each responsible for a distinct phase of the
computation.

#### Root Level

The root flow defines three sub-flows and connects them:

<a href="../../flowr/examples/mandlebrot/root.dot.svg" target="_blank">
<img src="../../flowr/examples/mandlebrot/root.dot.svg"></a>

At this level, `parse_args`, `generate_pixels`, and `render` each look like
a single process with inputs and outputs. The root flow doesn't know or care
what's inside them — it only sees their interfaces.

#### Inside a Sub-flow

Each sub-flow contains its own processes and connections. For example,
`render` contains two provided functions (`pixel_to_point` and `escapes`)
and a context function (`image_buffer`):

<a href="../../flowr/examples/mandlebrot/render.dot.svg" target="_blank">
<img src="../../flowr/examples/mandlebrot/render.dot.svg"></a>

And `generate_pixels` is built entirely from library functions:

<a href="../../flowr/examples/mandlebrot/generate_pixels.dot.svg" target="_blank">
<img src="../../flowr/examples/mandlebrot/generate_pixels.dot.svg"></a>

#### The Hierarchy as a Tree

The full structure looks like this:

```
mandlebrot (root flow)
├── parse_args (sub-flow)
│   └── get (context function)
├── generate_pixels (sub-flow)
│   ├── subtract (library function)
│   ├── zip (library function)
│   ├── range (library sub-flow)
│   │   └── ... (library functions inside range)
│   ├── join (library function)
│   ├── duplicate (library function)
│   └── enumerate (library function)
└── render (sub-flow)
    ├── pixel_to_point (provided function)
    ├── escapes (provided function)
    └── image_buffer (context function)
```

### Types of Processes

At any level of the hierarchy, a process can be one of:

- **Library function** — a reusable function from `flowstdlib` or another
  library, referenced with `lib://`. Examples: `add`, `subtract`, `compare`.

- **Library sub-flow** — a reusable flow from a library that itself contains
  functions. Example: `range` is a sub-flow in flowstdlib that uses `sequence`
  internally.

- **Provided function** — a function whose implementation is compiled to WASM
  and shipped alongside the flow. Referenced by a local path to its definition
  file. Examples: `escapes`, `pixel_to_point`.

- **Context function** — a function provided by the flow runner (e.g.,
  `flowrcli` or `flowrgui`) for interacting with the environment. Referenced
  with `context://`. Examples: `stdio/stdout`, `image/image_read`.

- **Sub-flow** — a flow definition that groups other processes. Referenced
  by a local path to its `.toml` file.

### Why Decompose?

Decomposing a flow into sub-flows serves several purposes:

- **Readability**: Each sub-flow has a clear purpose and a manageable number
  of processes. The root flow reads like a high-level algorithm description.

- **Reuse**: A sub-flow can be referenced from multiple places, or published
  as part of a library for others to use.

- **Encapsulation**: A sub-flow's internals can change without affecting the
  parent flow, as long as its inputs and outputs stay the same.

- **Visualization**: Each level of the hierarchy generates its own diagram,
  making it easy to understand the flow at different levels of detail.

### The Flattened Function Graph

At compile time, the hierarchy is flattened into a single graph of functions
and connections — the **function graph**. This is what the runtime actually
executes. Sub-flow boundaries are removed, and all functions are connected
directly.

The mandelbrot example's function graph looks like this:

<a href="../../flowr/examples/mandlebrot/functions.dot.svg" target="_blank">
<img src="../../flowr/examples/mandlebrot/functions.dot.svg"></a>

Every function from every sub-flow is visible, with all connections resolved.
This is the graph that is serialized into the `manifest.json` file and loaded
by the flow runner for execution.

For details on how sub-flows behave at runtime, see
[Sub-flow Execution Semantics](sub_flow_semantics.md).
