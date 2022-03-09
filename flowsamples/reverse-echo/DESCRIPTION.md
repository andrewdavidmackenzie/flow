reverse-echo
==

Description
===
Trivial flow that takes a line on `stdin`, reverses it and then prints it on `stdout`

Context Diagram
===
<a href="root.dot.svg" target="_blank"><img src="root.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Context Flow
* Library Functions used (`stdin` and `stdout` from `flowstdlib`)
* Custom function (in rust) with a structure on the output with sub-elements
* Connections between functions
* Connections from sub-elements of a function's output
* Function with single input (stdout) not requiring input name

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.dot.svg" target="_blank"><img src="functions.dot.svg"></a>

Click image to view functions graph.