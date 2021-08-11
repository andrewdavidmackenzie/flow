# `flowruntime`

`flowruntime` defines and implements the functions all flow runtimes 
(i.e. an app or server where flows are executed using `flowrlib`) must provide
in order to interact with `stdio` or the file system.

## Runtime functions
Those functions are organized into the following modules, each with multiple functions:
* [env](env/env.md) - used to interact with the environment
* [file](file/file.md) - used to interact with the file system
* [image](image/image.md) - used to read/write images to files
* [stdio](stdio/stdio.md) - used to interact with stdio