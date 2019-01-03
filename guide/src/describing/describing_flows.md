## Describing flows

Context

Nesting Flows

Flow description
- alias.. why needed

Values
- initializing
- static?

Functions
- defining inputs
- defining outputs
   - default output
   - named outputs

Connections
- multiple connections to an input
- multiple connections to an output
path concept - refer to toml for specifics.

Semantics
- loops are permitted and used as a feature
- won't run until all inputs satisfied
- first to arrive at an input wins
- using named outputs
- blocked on output by other "busy" inputs, thus inputs are not overwritten but queued up with backpressure.
buffering various and why
Not permitted

Running 
- initialization
- ready to run
- running in parallel 
currently runtime only executes one at a time, but that is destined to change and the fact that many can
run in parallel is part of the whole project's goals.


Primitive functions

- Standard library

- Custom functions
