## Components
Major components of the project and their status are:
* 'flowclib' - a rust library for parsing and "compiling" flow descriptions from toml files, producing generated output projects that can be compiled and run.
* 'flowc' - the flow "compiler" that is a CLI built around the 'flowclib' and that takes a number of command line arguments and source files or URLs.
* 'flowrlib' - the flow "runtime" library that is currently compiled and linked with a generated flow and takes care of executing it.
* 'flowstdlib' - the flow "standard library" which contains a set of pre-defined functions that can be used by flows being defined by the user
* 'web' - the first steps for a project to provide a web ui to flow definition and execution built using rust and WebAssembly with wasm-bindgen - just a "hello world" skeleton that compiles and runs at the moment
* 'electron' - intended as a standalone application, to be built wrapping the 'web' ui above, currentll stalled and about to be re-started around WebAssembly and new 'web' example