hello-world
==

Description
===
A simple flow that prints "Hello World!" on `stdout`

Context Diagram
===
<a href="context.dot.svg" target="_blank"><img src="context.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Context Flow
* A nested flow from a separate file in the same project
* A String value that is initialized at start-up
* Library Functions used (`stdout` from `flowstdlib`)
* Connection between a named output of the sub-flow and the function's input
* Initialization of a flow's output in [flow1](flow1.toml)

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.dot.svg" target="_blank"><img src="functions.dot.svg"></a>

Click image to view functions graph.