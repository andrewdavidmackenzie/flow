range-of-ranges
==

Description
===
A flow that generates a range of numbers, and for each of them generates a range from 1 upto the numner.
This is intended as a test case for an issue with two linked flows like 'sequence' that can lead to a deadlock

Context Diagram
===
![Context diagram](range-of-ranges.dot.png)

Features Used
===
* Context Flow
* Library Functions used (`stdout` from `flowstdlib`)
* Library Flows used (`sequence` from `flowstdlib`)
* Connections between functions

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
![Full functions diagram](functions.dot.png)