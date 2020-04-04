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

Context Diagram
===
![Context diagram](pipeline.dot.png)

Features Used
===
* Context Flow
* Setting initial value of a Value at startup
* Multiple connections into and out of functions and values
* Library Functions used from `flowstdlib`
* Use of aliases to refer to functions with different names
* Connections between functions and values
* Referring to a function's input by name in connections

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
![Full functions diagram](functions.dot.png)