arrays
==

Description
===
Sample to show the capabilities of the runtime mapping:
* gathering from an output of type object to an input of type object, of a specified depth : p1 --> p2
* an output of type array of objects to an input that is of type object                    : p2 --> p3

* P1 - range - generates a stream of outputs of type Number
* P2 - input of type Number of width 2, output of type Array of Number
* P3 - input of type Number and output of type Number, adding 1 in the processb/
* P4 - prints

Features Used
===
* Context Flow
* Setting initial value of a Value at startup
* Multiple connections into and out of functions and values
* Library Functions
* Implicit conversion between arrays of objects and objects done by runtime
* Explicit conversion between a stream of objects and an array using the `compose_array` library function