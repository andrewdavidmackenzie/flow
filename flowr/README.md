# `flowr`

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowr/index.html)

`flowr` is a binary run-time for running flows from the CLI built using the `flowrlib` library.

It handles the execution of `Functions` forming a `Flow` according to the defined semantics.

## Context Functions
The `flowr` `context` module implements the context functions that this runtime
provides for flows to interact with the surrounding "context" it is being run in, such as IO and File System.

These are all impure functions, or functions with side effects, not part of the flow itself.

Those functions are organized into the following modules, each with multiple functions:
* [args](src/cli/args/args.md) - used to get arguments that flow was invoked with
* [file](src/cli/file/file.md) - used to interact with the file system
* [image](src/cli/image/image.md) - used to create image files
* [stdio](src/cli/stdio/stdio.md) - used to interact with stdio