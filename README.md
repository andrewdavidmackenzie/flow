FLOW
###

This is an exploration project of some ideas I have for programming using a description of data flows and transformations.

Initially I want to do this as a visual programming paradigm, but to explore some of the ideas before investing too much time I have decided to use a description language (that would later be produced by a visual tool) and to program the running of it using a run-tim edeveloped in Rust (as an exercise in seriously learning rust also...).

I plan to try and develop a few trivial, and then simple, and later maybe more complicated sample programs to tease out the initial complexities.

The running of the algorithms would just be to demonstrate that the method can be used to describe the programs tried, and will not be performant, or anything like a compiled solution that would be required in a final solution.

Flow Descriptions
####

Flows may have zero or more inputs, outputs, values (constants), functions, and other sub-flows.
flow = [input] + [output] + [flow] + [values] + [functions]


To Consider
###
Error handling