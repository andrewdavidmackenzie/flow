# `flowruntime`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowruntime/index.html)

Here we define the functions an execution environment 
(i.e. an app or server where flows are executed using `flowrlib`) must provide.

These are "impure" functions that interact with stdio or the file system that 
cannot be implemented in the portable `flowrlib` library, and must be provided by the final
executable that links with it.

## Runtime functions
Those functions are organized into the following modules, each with multiple functions:
* [args](args/args.md) - used to get arguments that flow was invoked with
* [file](file/file.md) - used to interact with the file system
* [stdio](stdio/stdio.md) - used to interact with stdio
