# `flowc`

See also: [Code docs](../code/doc/flowc/index.html)

`flowc` is the "compiler and linker" for flows and flow libraries, although it is not 
very similar to what you might be familiar with as a compiler or linked.

It loads flow definition files, and builds the flow hierarchy reading from referenced
flows or functions and library references, and builds the flow in memory.

Then it connects all functions with data flows through the hierarchy and removes the
flow structure leaving a "network of functions" which it then optimizes (removing
unused functions and connections).

It checks that types match and required connections exist.

Lastly it generates a manifest describing the flow, which can be executed by `flowr`.

Depending on the command line options used it then invokes `flowr` (using cargo to ensure
it is up to date and built).

Most of the compilation and linking logic is in the `flowclib` library, which can be used
to build other command line utilities for compiling, or an IDE.