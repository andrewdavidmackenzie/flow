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

## Compiling the Samples
The samples set has now been converted to a rust crate with a custom build script.

Using `cargo build -p flowsamples` causes the build script to run, and it compiles in-place the samples
using the `flowc` compiler.

## Running the Samples
Using `cargo run -p flowsamples` causes the sample runner in main.rs to run. It looks for sub-folders in
the samples folder and then executes the sample within. 

When running them, it uses:
* test.arguments - arguments passed to the flow on the command line when executing it
* test.input - test input to send to the sample flow using STDIN

The output is sent to standard output.

To run a specific sample only use `cargo run -p flowsamples {sample-name}`

## Testing the Samples
You can test all samples by using `cargo test -p flowsamples`,
it will run each one in turn with the pre-defined arguments and standard input.

It also gathers the standard output, standard error and files generated and checks for correctness 
by comparing them to previously generated content distributed with the package.
* If there is any standard error found in the file test.err then the test will fail.
* If there is no standard error then it compares standard output captured in test.output to expected.output
and fails if there is a difference.
* If an expected.file exists then it compares it to file output in test.file and fails if there is any 
difference with the expected file.

```
cargo test -p flowsamples 
    Finished test [unoptimized + debuginfo] target(s) in 0.11s
     Running target/debug/deps/samples-9e024e2c420db146

running 16 tests
test test::test_all_samples ... ignored
test test::test_args ... ok
test test::test_arrays ... ok
test test::test_factorial ... ok
test test::test_fibonacci ... ok
test test::test_hello_world ... ok
test test::test_mandlebrot ... ok
test test::test_matrix_mult ... ok
test test::test_pipeline ... ok
test test::test_prime ... ok
test test::test_primitives ... ok
test test::test_range ... ok
test test::test_range_of_ranges ... ok
test test::test_reverse_echo ... ok
test test::test_router ... ok
test test::test_tokenizer ... ok

test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out
```

NOTE: Until multiple instances of the client/server pair for running flows can be run at once, we 
need to restrict the test framework to only run one test at a time, otherwise by default it will run
multiple tests at once, and some will fail.

NOTE: At the moment, to make the progress more visible, each sample has a test manually added to it
in `samples/main.rs`, so for a new sample a test needs to be added by the author.

To test just one sample use `cargo test -p flowsamples {test-name}`
```
cargo test -p flowsamples test_factorial
    Finished test [unoptimized + debuginfo] target(s) in 0.12s
     Running target/debug/deps/samples-9e024e2c420db146

running 1 test
test test::test_factorial ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out
```

## Default workspace member crate
The `samples` crate is one of the `default-members` of the `flow` workspace project, so it is used if no 
particular package is supplied, thus the samples can also be built and run using:
* `cargo build` : compile the samples using `flowc`
* `cargo run`   : run the samples using `flowr`
* `cargo test`  : run the samples using `flowr` and check the generated output is correct

As other `default-members` are added to the workspace over time, those commands may do other things, so
just be aware that if you only want to run the samples the `-p samples` option above will be safer.

## `flowsamples` executable
There is also an executable (`bin` or binary) installed with the library called `flowsamples` that if run
without any arguments will run all the samples. You can supply it the name of a sample (the name of the folder
under `samples` where the sample is) to run just that one sample.

## Developing a new sample
To develop a new sample, just create a new folder under 'samples' with your sample name. 

Add the context.toml and any other included flows and describe them.

Add a DESCRIPTION.md file that describes what the sample does and what features of flow it uses.

Add an entry in the guide's "samples" section that will include the DESCRIPTION.md file above.
