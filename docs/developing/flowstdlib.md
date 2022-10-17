# `flowstdlib`

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

## Building this library from Source
Libraries like `flowstdlib` are built using `flowc` with the `-l` option. 

This builds a directory tree of all required files for a portable library, including:-
  * documentation files (.md MarkDown files, generated .dot files and generated SVG files)
  * TOML definition files for flows and functions
  * Function implementations compiled to a .wasm WASM file for each function.
  * A `manifest.json` manifest of the libraries functions and where the implementations (.wasm files) can be found. \
This is used by the Runtime to be able to load it.

## Native use by a Runtime
It can be compiled and linked natively to a (rust) run-time. `flowr` offers the `-n` option to specify this use of it.

## WASM use by a Runtime
Its functions can  be loaded from WASM files by `flowrlib` at run-time using the `manifest.json` file to locate them.

## Configuring `FLOW_LIB_PATH` during development
If you are using it as part of the larger `flow` workspace then you just need the `flow` project root directory
in your `FLOW_LIB_PATH` as described above (or added to the lib search part using the `-L <dir>` option).

