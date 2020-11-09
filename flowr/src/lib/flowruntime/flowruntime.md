# `flowruntime`

`flowruntime` defines the functions an execution environment 
(i.e. an app or server where flows are executed using `flowrlib`) must provide
in order to interact with `stdio` or the file system.

## Runtime functions
Those functions are organized into the following modules, each with multiple functions:
* [args](args/args.md) - used to get arguments that flow was invoked with
* [file](file/file.md) - used to interact with the file system
* [stdio](stdio/stdio.md) - used to interact with stdio