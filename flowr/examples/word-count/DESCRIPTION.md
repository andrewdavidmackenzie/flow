word-count
==

Description
===
Counts the total number of words across multiple lines of text read from stdin.

Each line is read by `readline` and sent to `split` for tokenizing into words.
Lines are processed in parallel — the splitter works on whichever line arrives
next, demonstrating dataflow parallelism across input lines.

A work counter tracks pending split operations, and a word counter accumulates
the total. A `tap` gates the output until all splitting is complete (pending
work reaches zero), then outputs the final word count to stdout.

Root Diagram
===
<a href="root.svg" target="_blank"><img src="root.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Root Flow
* Setting initializer of a Function's input with a constant initializer
* Library Functions
* Iteration (possibly in parallel) via feedback of partial output values back to the same function's input
* Implicit conversion between arrays of string and string done by run-time, in feedback loop to the same process

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.svg" target="_blank"><img src="functions.svg"></a>

Click image to view functions graph.
