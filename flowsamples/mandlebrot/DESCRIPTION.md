mandlebrot
==
Render a mandlebrot set, with the size and imaginary space configured via input parameters.

The pre-configured test renders a very small mandlebrot set (25x25 pixels) in order to keep the test time short.

<a href="expected.file" target="_blank"><img src="expected.file"></a>

Description
===
Renders a mandelbrot into a PNG file.

Notably, this project in the `project` folder is a full standalone rust project
and the functions are made available as functions to the Flow project that is described 
in the toml files alongside - showing how native code can live alongside and be used by 
the flow.

Context Diagram
===
<a href="context.dot.svg" target="_blank"><img src="context.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Context Flow
* Child flow described separately, with named outputs to parent flow
* Connections between Input/Outputs of parent/child flows
* Setting initial value of a Value at startup
* Multiple connections into and out of functions and values
* Library Functions used (`toString` and `add` from `flowstdlib`) to convert Number to String and to add numbers
* Use of aliases to refer to functions with different names inside a flow
* Connections between flows, functions and values

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.dot.svg" target="_blank"><img src="functions.dot.svg"></a>

Click image to view functions graph.