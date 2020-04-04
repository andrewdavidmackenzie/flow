reverse-echo
==

Description
===
Trivial flow that takes a line on `stdin`, reverses it and then prints it on `stdout`

Context Diagram
===
![Context diagram](reverse-echo.dot.png)

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
![Full functions diagram](functions.dot.png)