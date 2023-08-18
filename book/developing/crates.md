## Crates in the flow project

The flow project is split into a number of different crates for a number of reasons:
- proc macros need to be in their own crate
- sharing of structures and code across compiler and runner crates
- desire to separate core functionality in libraries from CLI binaries and enable UI applications using only the 
  libraries
- provide CLI versions of compiler and runner
- avoid cyclic dependencies between parts
- allow to compile optionally without some features, not using code in a crate
- separate library implementation from compiler and runner

The following sections provide a brief description of the crates in the project.