args
==

Description
===
A flow that shows how arguments can be passed into a flow when executed and are available to functions at run-time.

Context Diagram
===
![Context diagram](context.dot.svg)

Features Used
===
* Context Flow
* Library Functions used (`stdio/stdout` from `flowruntime`)
* Reduced syntax so that `alias` of referenced processes default to their names (`get` and `stdout`)
* Selecting a specific indexed entry of an `Array` output
* Library Flows used (`args/get` from `flowstdlib`)
* Connections between functions

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
![Full functions diagram](functions.dot.svg)