First
==
Very simple first flow used to demonstrate a simple flow that actually does something, but can be followed.
See [first flow](../../guide/src/first_flow/first_flow.md) section in the guide for a much more detailed description of this sample and a step-by-step walkthrough of it running.

Features Used
===
* Context Flow
* Values to store intermediate values
* Setting initial value of a Value at startup
* Multiple connections into and out of functions and values
* Library Functions used (`stdout` and `add` from `flowstdlib`) to print a value and to add two numbers
* Use of aliases to refer to functions with different names
* Connections between functions and values
* Referring to a function's input by name in connections

Description
===
Generates a series of numbers and prints it out on `stdout`, as per diagram below:
![First flow](first.png)