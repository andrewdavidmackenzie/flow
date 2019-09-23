## Flow Runtime Definitions

A "runtime" (i.e. an app or server where flows are executed using flowrlib) must provide
a set of functions for running flows.

There are a number of "impure" functions that interact with stdio or the file system that 
cannot be implemented in the portable `flowrlib` library, and must be provided by the final
executable that links with it and runs the flow.

### Runtime functions
Those functions are organized into the following modules:
* [args](args/args.md) - used to get arguments that flow was invoked with
* [file](file/file.md) - used to interact with the file system
* [stdio](stdio/stdio.md) - used to interact with stdio