## Project Components
Major components of the project and their status are:
* `flowc` - the flow "compiler" that is a CLI built around the 'flowclib' and that takes a number of command line arguments and source files or URLs.
* `flowclib` - a rust library for parsing and "compiling" flow descriptions from toml files, producing generated output projects that can be compiled and run.
* `flowr` - A flow-runner standalone binary that can be used to run and debug flows compiled with `flowc`.
* `flowrlib` - the flow "run-time" library that is currently compiled and linked with a generated flow and takes care of executing it.
* `flowruntime` - core functions for interacting with the environment and IO
* `flowstdlib` - the flow "standard library" which contains a set of pre-defined functions that can be used by flows being defined by the user
* `flowide` - the first steps for a project to provide a ui to flow definition and execution built using rust and gtk3+
* `provider` - A small crate for fetching content of flows from different types of sources