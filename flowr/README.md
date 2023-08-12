# `flowr`

`flowr` includes the `flowrlib`library for running flows (see below for details)

`flowr` includes a number of "runner" applications (built using the `flowrlib` library) for running flows:
- `flowrcli`to run flows from the command line
- `flowrgui`a flow runner with a graphical user interface (GUI) built using Iced
- `flowrex` a binary that only executes jobs (does not coordinate flow execution) and can be used over the network 
  by a coordinator as a way to have more execution resources executing a flow's jobs

They handle the execution of `Functions` forming a `Flow` according to the defined semantics.

## flowrlib
It is responsible for reading a flow definition in a `Manifest` file, loading the required libraries
from `LibraryManifest` files and then coordinating the execution by dispatching `Jobs` to be executed
by `Function` `Implementations`, providing them the `Inputs` required to run and gathering the `Outputs` produced
and passing those `Outputs` to other connected `Functions` in the network of `Functions`.

### features
These are the conditionally compiled features of `flowr` crate:
- submission - include the ability to receive a submission of a flow for execution
- context - makes this crate aware of the flow context functions or not
- debugger - feature to add the debugger
- metrics - feature for tracking of metrics during execution
- flowstdlib - (is an optional dependency, which act like a feature flag) to allow native versions of flowstdlib
functions to be compiled and linked or not (and rely on wasm versions)

By default, the following are enabled: "debugger", "metrics", "context", "submission", "flowstdlib"

## `flowrcli` and `flowrgui`

### Context Functions
The `context` folder implements the context functions that each runtime provides for flows to interact with the 
environment (such as Standard IO and File System) as well as providing definitions of the context functions 
to be used when compiling a flow.

These are all impure functions, or functions with side effects, not part of the flow itself.

Those functions are organized into the following modules, each with multiple functions:
* [args](src/bin/flowrcli/context/args/args.md) - used to get arguments that flow was invoked with
* [file](src/bin/flowrcli/context/file/file.md) - used to interact with the file system
* [image](src/bin/flowrcli/context/image/image.md) - used to create image files
* [stdio](src/bin/flowrcli/context/stdio/stdio.md) - used to interact with stdio

## `flowrex`
You can find more details about how to use it in running flows in the [distributed](../docs/running/distributed.md)
section.