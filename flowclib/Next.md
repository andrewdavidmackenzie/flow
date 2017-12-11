
Loading of functions from stdlib (at compile time) - by finding them by name on the load path
and then requesting their definition?
Maybe initialize default PATH with "." so that it finds default one.

Using connections, for each output, add a reference to one or more input that data should be sent to when made available.

Runnables generation with the references to the runnable number and input number.

Drop unneeded functions and values

Check connections all match up in direction.

Write tests for compile functions and connection mapping and dropping etc

Write test cases for the pruning cases mentioned in flow_compiling.md

Look at error chain crate and define own errors in loader/flowc in particular

Maybe add references to values and functions in the connections when we are doing that

Definition Doubts
=================
Functions have only one output?

maybe need to define splitters to break up tuples or structs so others can act on
parts of them?

Code Improvements
=================
Define type aliases for the optional lists of inputs etc to give more meaning to signatures.

Look at methods in flow and loader and see how many of the ones that look for io etc
could be returnng references and not creating new strings with format!

Look to see how connection tables in compile could be done with just references and not creating
all those new strings and new vectors.