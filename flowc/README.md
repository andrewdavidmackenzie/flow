# `flowc`
See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowc/index.html)

`flowc` is the "compiler and linker" for flows and flow libraries, although it is not 
very similar to what you might be familiar with as a compiler or linker.

It loads flow definition files, and builds the flow hierarchy reading from referenced
flows/functions and library references, and builds the flow in memory.

Then it connects all functions via data flows through the hierarchy and removes most of the
flow structure leaving a "network of functions" which it then optimizes (removing
unused functions and connections).

It checks that types match and required connections exist. 

It also checks for some illegal or cases that would prove problematic at runtime
(specific types of "loops" or contention for a connection)

Lastly it generates a manifest describing the flow, which can be executed by `flowr`.

It then may (depending on the command line options used) invoke `flowr` (using cargo to ensure
it is up to date and built).

# `flowclib`
See also: [Code docs](http://andrewdavidmackenzie.github.io/flow/code/doc/flowclib/index.html)

This library contains most of the compilation and linking logic. 
See more in [README.md](src/lib/README.md)