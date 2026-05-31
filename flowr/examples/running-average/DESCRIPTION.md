Running RMS Average
==

Description
===
A flow that computes the running RMS (Root Mean Square) average of a stream
of numbers read from stdin. Inspired by the running average example from
"Lucid, the Dataflow Programming Language" (Wadge & Ashcroft, 1985, page 47).

RMS = sqrt(sum(x²) / n)

This example uses **only flowstdlib functions** — no custom Rust/WASM code needed.

Root Diagram
===
<a href="root.svg" target="_blank"><img src="root.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Root flow with no sub-flows
* Library Functions used (`multiply`, `add`, `count`, `divide`, `sqrt` from `flowstdlib`)
* Context Functions used (`readline`, `stdout`)
* Loopback connections for accumulating state across iterations
* Input initializers (`once` for initial values, `always` for constants)
* Reading from stdin via `readline/json`

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.svg" target="_blank"><img src="functions.svg"></a>

Click image to view functions graph.
