## Flow Runtime Library

This is the runtime library of flow.

It handles the execution of runnables according to the semantics defined.

### Runtime functions
Additionally, it provides a number of standard, impure (have side effects or do IO), functions to 
flows to help them interact with the environment they run in.

Those functions are organized into the following modules, each with multiple functions:
* env
* stdio
* file