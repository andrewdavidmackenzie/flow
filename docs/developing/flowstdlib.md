# `flowstdlib` Overview

See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowstdlib/index.html)

`flowstdlib` is a standard library of functions and flows for `flow` programs to use.

## Modules
`flowstdlib` contains the following modules:
  * [`control`](../../flowstdlib/src/control/control.md)
  * [`data`](../../flowstdlib/src/data/data.md)
  * [`fmt`](../../flowstdlib/src/fmt/fmt.md)
  * [`math`](../../flowstdlib/src/math/math.md)

## Use by the Compiler
In order for the compiler to be able to find the library's `flow` and `function` definitions, the directory containing
this library must be part of `FLOW_LIB_PATH` or specified using an instance of the `-L` command line option to `flowc`, 

NOTE: That flows are compiled down to a graph of functions at compile time, and do not exist at runtime.

## Building this library from Source
Libraries like `flowstdlib` are built using `flowc` with the `-l` option. 

This builds a directory tree (in `target/{lib_name}`) of all required files for a portable library, including:-
  * documentation files (.md MarkDown files, .dot graphs of flows, graphs rendered as .dot.svg SVG files)
  * TOML definition files for flows and functions
  * Function implementations compiled to a .wasm WASM file
  * A `manifest.json` manifest of the libraries functions and where the implementations (.wasm files) can be found.
This is used by the Runtime to be able to load it.

## Dual nature of flow libraries
Flow libraries such as `flowstdlib` have a dual nature. They can compiled and linked natively to a binary such
as `flowr`, or when compiled by `flowc` (using the `-l`) all the functions implementations are compiled to
`.wasm` WASM files.

## Native use by a Runtime
`flowr` offers the `-n/--native` option for the `flowstdlib` to be used natively. When used, functions it
contains will be run natively (machine code), as opposed to WASM implementations of the functions.

## WASM use by a Runtime
If the `-n/--native` option is not used, and the library manifest (`manifest.json`) is found by the flow 
runner (e.g. `flowr`) at runtime (using`FLOW_LIB_PATH` or `-L`), then the manifest is read and the functions 
WASM implementations found and loaded.

When a job is executed that requires one of these library functions, the WASM implementation is run.

## Configuring `FLOW_LIB_PATH` during development
If you are using it as part of the larger `flow` workspace then you just need the `flow` project root directory
in your `FLOW_LIB_PATH` as described above (or added to the lib search part using the `-L <dir>` option).

## features
These are the conditionally compiled features of `flowstdlib`:
- default - No features are enabled by default
- wasm - feature to enable compile of functions to WASM implementation. If not activated, the WASM implementations
will not be compiled and the library must be linked natively as described above.