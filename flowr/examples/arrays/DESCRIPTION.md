arrays
==

Description
===
Sample to show the capabilities of:
* gathering a stream of outputs of type Object to an input of type Object, of a specified size. 
This is done by the P2 'composer' (`ComposeArray`) function.
* Decomposing an output of type array of objects to a stream of objects. This is done by the runtime when it sees
a connection from an array of Type to Type.

* P1 - sequence    - generates a stream of outputs of type number
* P2 - accumulator - accumulate the stream of numbers into arrays of numbers of size 4
* P3 - adder       - input of type Number and output of type Number, adding 1 in the process
* P4 - print       - print the output (the original sequence with '1' added to each number)

This example (with default settings on a multi-core machine) shows parallelism of the add function,
dispatching multiple jobs for it in parallel as the array of number output from the previous process
is deserialized (from array/number to number) in the connection from accumulator to added, creating
a job for each Number. You can see this by using the `-j` option of the runtime to limit the number of outstanding
jobs and the `-m` options to dump metrics after execution. The "Max Number of Parallel Jobs" should be similar or
greater than 4, which is the size of the array of numbers formed.

Root
===
<a href="root.dot.svg" target="_blank"><img src="root.dot.svg"></a>

Click image to navigate flow hierarchy.

Features Used
===
* Root Flow
* Setting initial value of a Function's inputs
* Multiple connections into and out of functions and values
* Library Functions
* Array of numbers construction (using the "accumulate" function) from a stream of numbers. This uses a loop-back 
connection of partially accumulated arrays. By specifying an array ("chunk size") of four, a a stream of arrays of 
four numbers (the"chunks") is produced.
* Implicit conversion between arrays of (four) numbers to a stream of numbers done automatically by run-time, from the 
accumulator's output of arrays of four numbers, to "add"'s input of a single number.

Functions Diagram
===
This diagram shows the exploded diagram of all functions in all flows, and their connections.
<a href="functions.dot.svg" target="_blank"><img src="functions.dot.svg"></a>

Click image to view functions graph.