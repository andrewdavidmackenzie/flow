# `flowr`

`flowr` includes a number of "runner" applications (built using the `flowrlib` library) for running flows:
- `flowrcli`to run flows from the command line
- `flowrgui`a flow runner with a graphical user interface (GUI) built using Iced

They handle the execution of `Functions` forming a `Flow` according to the defined semantics.

## Context Functions
The `context` folder implements the context functions that each runtime provides for flows to interact with the 
environment (such as Standard IO and File System) as well as providing definitions of the context functions 
to be used when compiling a flow.

These are all impure functions, or functions with side effects, not part of the flow itself.

Those functions are organized into the following modules, each with multiple functions:
* [args](src/bin/flowrcli/cli/args/args.md) - used to get arguments that flow was invoked with
* [file](src/bin/flowrcli/cli/file/file.md) - used to interact with the file system
* [image](src/bin/flowrcli/cli/image/image.md) - used to create image files
* [stdio](src/bin/flowrcli/cli/stdio/stdio.md) - used to interact with stdio

## Features
These are the conditionally compiled features of `flowrcli`:
- default - "debugger" and "metrics" features are enabled by default
- debugger - feature to add the debugger
- metrics - feature for tracking of metrics during execution

## Code Docs
See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowr/index.html)
