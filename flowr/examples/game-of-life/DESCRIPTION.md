Game of Life
==

Description
===
A flow that implements Conway's Game of Life. It takes a grid size, number of
iterations, and a seed pattern name as arguments. Each generation is rendered
to an image, showing the evolution of the cellular automaton.

Supported seed patterns: `glider`, `blinker`, `block`, `rpentomino`.

Root Diagram
===
<a href="root.svg" target="_blank"><img src="root.svg"></a>

Click image to navigate flow hierarchy.

Functions Diagram
===
<a href="functions.svg" target="_blank"><img src="functions.svg"></a>

Click image to view functions graph.

Features Used
===
* Root Flow
* Sub-flows
* Provided function (game_step - compiled to WASM)
* Library Functions used
    * `subtract` for iteration counting
    * `compare` for termination check
    * `tap` for conditional loopback
* Context Functions
    * `Args` to parse arguments
    * `image_buffer` to render pixels
    * `stdout` to display iteration count
* Loopback connections for iterative computation
* Output route selectors for destructuring pixel data
