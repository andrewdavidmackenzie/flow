matrix_mult
==

Description
===
A flow that calculates the matrix product of two matrices.

Root Diagram
===
<a href="root.dot.svg" target="_blank"><img src="root.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Root Flow
* Getting structured json input from readline/stdin
* Connections between functions
* Multiple connections into and out of functions
* Library Function `multiply` from `flowstdlib` `matrix` module
* Use of aliases to refer to functions with different names inside a flow
* Automatic array decomposing into the elements the array holds when target function input does not accept the 
array type
* Accumulating stream of items into an array of items using 'accumulate'

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.dot.svg" target="_blank"><img src="functions.dot.svg"></a>

Click image to view functions graph.