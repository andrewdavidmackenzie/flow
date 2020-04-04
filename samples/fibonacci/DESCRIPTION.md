fibonacci
==

Description
===
A flow that generates a Fibonacci series of numbers and prints it out on `stdout`.

Features Used
===
* Context Flow
* Child flow described separately, with named outputs to parent flow
* Connections between Input/Outputs of parent/child flows
* Values to store intermediate values
* Setting initial value of a Value at startup
* Multiple connections into and out of functions and values
* Library Functions used (`toString` and `add` from `flowstdlib`) to convert Number to String and to add numbers
* Use of aliases to refer to functions with different names inside a flow
* Connections between flows, functions and values

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
![Full functions diagram](functions.dot.png)