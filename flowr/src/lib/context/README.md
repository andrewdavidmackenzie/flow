# `context`

`context` defines and implements the functions all flow  must use in order to interact with `stdio` 
or the file system. i.e. all impure functions, or functions with side effects not part of the flow itself.

## `context` functions
Those functions are organized into the following modules, each with multiple functions:
* [args](args/args.md) - used to get arguments that flow was invoked with
* [file](file/file.md) - used to interact with the file system
* [stdio](stdio/stdio.md) - used to interact with stdio