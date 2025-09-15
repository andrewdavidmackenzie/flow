# Flow Examples
`examples` contains a set of example flows used to demonstrate flow (flowc, flowr, flowstdlib), the different 
semantics and characteristics of flows that can be written, and test them to ensure they continue to run
correctly over time.

Each subdirectory holds a self-contained flow example, with flow definition, docs etc and some of 
them provide their own function implementations that get compiled to WASM by flowc when the flow is compiled.

Flow enables higher levels of parallelization of the execution of 'jobs' within flows by allowing many jobs to be
run in parallel, which then may be executed out of order. This can lead to unpredictable ordering of the output 
values of some operations. To embrace this, the examples typically avoid requiring a specific ordering of the output
values.

## Environment Variable Requirements
If you are using `make`, then temporary additions to $PATH will be made for you so that the required
flow executables (`flowc`and `flowr`) are found.

However, if you wish to run an example from the command line, then you will need to make sure the `flowc` and
`flowr` executables (built by the Makefile) are in your path (or use the full path when running them).

You can do this using:

`export PATH="target/debug:target/release:$PATH"`

from the project root directory.

## Building all examples 
`cargo test` 

Builds all examples to make sure they compile, but they are not run.

`cargo build --examples`

Builds all examples

## Running one example
`cargo run --example $example-name"`

This can be run from the root folder or the flowr folder.
The named example is built and run.

The flow will be run with the arguments and standard input defined in files within each directory
(if they are not present, then those args or input is zero).

## Testing one example
`cargo test --example $example-name` 

The flow will be run with the arguments and standard input defined in files within each directory
(if they are not present then those args or input is zero) and the output compared to the expected
output (defined in files in the directory). If the output does not match the expected output then the test fails.

## Testing all examples
`cargo test --examples` 

Will run the tests in all examples.
