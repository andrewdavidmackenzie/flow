sequence
==

Description
===
A flow that generates a sequence of output numbers in a range between two input numbers.
The sequence starts at 2, with steps of 3, and a limit of 100 - so 98 should be the last 
number output, followed by a "Sequence done" string.

Root Diagram
===
<a href="root.dot.svg" target="_blank"><img src="root.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Root Flow
* Sub-process inclusion (`sequence` from `flowstdlib`, which happens to be implemented as a flow)
* `context` `stdout` process to print the numbers in the sequence and a String to standard output
* Connections between sub-processes and sub-process to context output
* Initial value setting on sub-flow inputs
* `flowstdlib` `join` function used to trigger the output of a string when the sequence completes

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.dot.svg" target="_blank"><img src="functions.dot.svg"></a>

Click image to view functions graph.