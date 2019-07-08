## Project Components
Major components of the project and their status are:
* `flowclib` - a rust library for parsing and "compiling" flow descriptions from toml files, producing generated output projects that can be compiled and run.
* `flowc` - the flow "compiler" that is a CLI built around the 'flowclib' and that takes a number of command line arguments and source files or URLs.
* `flowrlib` - the flow "runtime" library that is currently compiled and linked with a generated flow and takes care of executing it.
* `flowr` - A flow-runner standalone binary that can be used to run and debug flows compiled with `flowc`.
* `flowstdlib` - the flow "standard library" which contains a set of pre-defined functions that can be used by flows being defined by the user
* `ide` - the first steps for a project to provide a ui to flow definition and execution built using rust and WebAssembly with wasm-bindgen.