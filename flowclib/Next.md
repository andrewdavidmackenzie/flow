Avoid having to generate use for all functions in code gen - producing warnings....

use build.rs in std lib to build tables of runnables that compiler can use to find them by path.
replaces usages of terminal.toml in samples with a stdlib version...

Loading of functions from stdlib (at compile time) - by finding them by name on the load path
and then requesting their definition?
Maybe initialize default PATH with "." so that it finds default one.
remove terminal.toml from examples and use stdlib version in it's place.

Find Implementations and create them for code generation, and check types etc before???

Release versions of the libs to cargo.io and change cargo_gen.rs to use that and not path


Drop unneeded functions and values

Check connections all match up in direction.

Write tests for compile functions and connection mapping and dropping etc

Write test cases for the pruning cases mentioned in flow_compiling.md

Look at error chain crate and define own errors in loader/flowc in particular

Maybe add references to values and functions in the connections when we are doing that

Multithreading
==============
Experiment with multithreading of execution. Use rayon divide and conquer technique on 
the list of runnables?

Test Coverage
=============
https://github.com/codecov/example-rust

Definition Doubts
=================
maybe need to define splitters to break up tuples or structs so others can act on
parts of them? Named fields as separate outputs of a Value?

Efficiency
==========
Outputs of a function that has been run could be reference counted and given as inputs to 
other functions by reference, then freed when the last one is used. Arc.

Code Improvements
=================
More idiomatic rust, in terms of collections and iterations. using filter, into_iter(), iter(), 
retain, map, filter_map, collect, 

Improve error handling by defining own error types and using error_chain?

Define type aliases for the optional lists of inputs etc to give more meaning to signatures.

Look at methods in flow and loader and see how many of the ones that look for io etc
could be returnng references and not creating new strings with format!

Look to see how connection tables in compile could be done with just references and not creating
all those new strings and new vectors.

Generics
========
Investogate how to make functions and values and I/O of generic types!