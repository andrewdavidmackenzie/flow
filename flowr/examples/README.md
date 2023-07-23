# Flow Examples
`examples` contains a set of example flows used to demonstrate flow (flowc, flowr, flowstdlib), the different 
semantics and characteristics of flows that can be written, and test them to ensure they continue to run
correctly over time.

Each subdirectory holds a self-contained flow example, with flow definition, docs etc and some of 
them provide their own function implementations that get compiled to WASM by flowc when the flow is compiled.

## Environment Variable Requirements
If you are using the makefile, then temporari additions to $PATH will be made for you so that the required
flow executables (`flowc`and `flowr`) are found.

However, if you wish to run an example from the command line, then you will need to make sure the `flowc` and
`flowr` executables (built by the Makefile) are in your path.

You can do this using:

`export PATH="target/debug:target/release:$PATH"`

from the project root directory.


In order for the `flowc` compiler to find context functions used during compilation
you will also need `FLOW_CONTEXT_ROOT` to be set correctly (this is also done by
the Makefile)

`export FLOW_CONTEXT_ROOT="/Users/andrew/workspace/flow/flowr/src/bin/flowrcli/context"`

## Building all examples 
`cargo test` 

Builds all examples to make sure they compile, but they are not run.

`cargo build --examples`

Builds all examples

## Running one example
`cargo run --example $example-name"`

This can be run from the root folder or the flowr folder.
The named example is build and run.

The flow will be run with the arguments and standard input defined in files within each directory
(if they are not present then those args or input is zero).

## Testing one example
`cargo test --example $example-name` 

The flow will be run with the arguments and standard input defined in files within each directory
(if they are not present then those args or input is zero) and the output compared to the expected
output (defined in files in the directory). If the output does not match the expected output then the test fails.

## Testing all examples
`cargo test --examples` 

Will run the tests in all examples.
