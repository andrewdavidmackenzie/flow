## Value Definitions
A value of the specified type that is available as an input to something else, or which can
be written to for storage:
* `name` - the name of the value
* `type` - the type of the value
* `init` [Optional] - it's initial value if one is desired
* `static` [Optional] - the value should remain, even after sending via its output

A value can have an initial value specified. It will be setup at flow initialization before execution
begins and will immediately be available at it's output on the first iteration of the execution loop.

If no initial value is provided, then initially a value will be empty and remain so until another 
value or function (connected to it by a connection) sends a value to it.

When the output of a value is read (as the input to another value or function) it is emptied, unless the 
optional `static` keyword is used, in which case when written to, that value remains available at its output
always.
