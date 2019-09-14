## Flow Programming

Flows are "programmed" by describing a network of "functions" that are connected by "data flows".
 
In flow, there are some specific semantics about how the data flows and function invocation
work, but the basics are pretty simple.

A flows may have zero or more inputs, a set of outputs (which can be destructured into component parts), 
values (initialized or not), functions, and sub-flows.

Hence:

`flow = [input] + [output] + [flow] + [values] + [functions]`

To Be Extended....