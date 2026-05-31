GCD (Greatest Common Divisor)
==

Description
===
Computes the Greatest Common Divisor of two numbers using Euclid's
subtraction-based algorithm, implemented entirely in dataflow using
only flowstdlib functions.

The algorithm: repeatedly subtract the smaller from the larger until
they are equal. The equal value is the GCD.

Root Diagram
===
<a href="root.svg" target="_blank"><img src="root.svg"></a>

Click image to navigate flow hierarchy.

Functions Diagram
===
<a href="functions.svg" target="_blank"><img src="functions.svg"></a>

Click image to view functions graph.

Features Used
===
* Library Functions used
    * `compare_switch` for routing larger/smaller values
    * `subtract` for computing the difference
* Context Functions (`args/get`, `stdio/stdout`)
* Loopback connections for iterative computation
* Termination via the `equal` output of `compare_switch`
