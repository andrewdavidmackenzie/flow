# Sample flows
The project includes a number of sample 'flows' that have been developed during the development
of the compiler and the run-time to drive the project development and demonstrate it working. 

They also serve as a type of regression test to make sure we don't break any of the 
semantics that the samples rely on.

They range from the extremely simple "hello-world" example to more complex ones like generation of
a fibonacci series or a mandlebrot set image.

## Structure of each sample
Each sample directory contains:
* A `DESCRIPTION.md` file that:
    * describes what the `Flow` does
    * lists the features of `flow` that this sample uses and demonstrates
* A ```context.toml``` file that is the root file of the flow description
* Files used in the automated testing of each sample:
    * ```test_arguments.txt``` the arguments to be passed to the flow when running it
    * ```test_input.txt``` the input supplied to the flow when running it
    * ```expected_output.txt``` the output that the flow is expected to produce when invoked with 
```text_arguments.txt``` and input ```test_input.txt```

## Compiling and Running the Samples
The samples set has now been converted to a rust crate with a custom build script.

Using `cargo build -p samples` causes the build script to run, and it compiles in-place the samples
using the `flowc` compiler.

Using `cargo run -p samples` causes the sample runner in main.rs to run. It looks for sub-folders in
the samples folder and then executes the sample within, and compares the actual output with the 
expected output and fails if they are different.

The `samples` crate is one of the `default-members` of the `flow` workspace project, so it is used if no 
particular package is supplied, thus the samples can also be built and run using:
* cargo build : compile the samples using `flowc`
* cargo run   : run the samples using `flowr`

As other `default-members` are added to the workspace over time, those commands may do other things, so
just be aware that if you onyl want to run the samples the `-p samples` option above will be safer.