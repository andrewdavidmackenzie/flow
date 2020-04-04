factorial
==

Description
===
A flow that calculates the factorial of a number and prints it out on `stdout`.

Context Diagram
===
![Context diagram](factorial.dot.png)

Features Used
===
* Context Flow
* Connections between functions
* Loop-back connections to accumulate a multiplication result
* Initializing function inputs with values, once and constantly
* Multiple connections into and out of functions
* Library Functions `to_number`, `multiply`, `subtract` from `flowstdlib`
* Library Functions `tap`, `compare` from `flowstdlib`
* Use of aliases to refer to functions with different names inside a flow

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
![Full functions diagram](functions.dot.png)