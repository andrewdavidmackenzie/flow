## Project Components and Structure

Here is a summary of the project components, their purpose and a link to their `README.md`:

* [flowcore](../../flowcore/README.md) - A set of core structs and traits used by `flowr` and `flowc` plus code 
  to fetch content from file/http and resolve library (lib://) references.
* [flowmacro](../../flowmacro/README.md) - A macro used to help write function implementation code that compile natively
  and to wasm
* [flowc](../../flowc/README.md) - The `flowc` flow compiler binary is a CLI built around `flowclib` that 
  takes a number of command line arguments and source files or URLs and compiles the flow or library referenced.
    * `flowclib` is the library for compiling flow program and library definitions from toml 
      files, producing generated output projects that can be run by `flowr`.
* [flowrlib](../../flowrlib/README.md) - The flow runner library that loads and executes compiled flows.
* [flowr](../../flowr/README.md) - The `flowr` flow runner binary that can be used to run and debug 
  flows compiled with a flow compiler such as `flowc`.
* [flowrex](../../flowrex/README.md) - `flowrex` is a minimal flow job executor, intended for use across the netowkr 
  associated with `flowr` (above).
  flows compiled with a flow compiler such as `flowc`.
* [flowstdlib](../../flowstdlib/README.md) - the flow "standard library" which contains a set of functions that 
  can be 
  used by flows being defined by the user
* [flowsamples](../../flowsamples/README.md) - A set of sample flows