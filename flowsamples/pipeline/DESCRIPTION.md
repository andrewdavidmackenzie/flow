pipeline
==

Description
===
A sample that shows a simple "pipeline flow" with a number of functions organized into a
pipeline. When supplied with a "stream" of inputs, multiple functions are able to run in
parallel utilizing more than one core on the machine.

Using command line options (-j, -t) the flow can be invoked with just one worker thread and it 
becomes sequential. The metrics of how many jobs were able to be processed in parallel can
be viewed using the -m command line option.

Root Diagram
===
<a href="root.dot.svg" target="_blank"><img src="root.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Rot Flow
* Setting initial value of a Value at startup
* Multiple connections into and out of functions and values
* Library Functions used from `flowstdlib`
* Use of aliases to refer to functions with different names
* Connections between functions and values
* Referring to a function's input by name in connections

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.dot.svg" target="_blank"><img src="functions.dot.svg"></a>

Click image to view functions graph.