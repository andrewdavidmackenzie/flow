### Developing a sample
To develop a new sample, just create a new folder under 'samples' with your sample name. 

Add the context.toml and any other included flows and describe them.

Add a DESCRIPTION.md file that describes what the sample does and what features of flow it uses.

Add an entry in the guide's "samples" section that will include the DESCRIPTION.md file above.

### Running samples
To run all the sample flows use `cargo run -p flowsamples`

This will run all samples it finds in sub-folders of the `samples` folder, 
the output will show on standard output.

When running them, it uses:
* test.arguments - arguments passed to the flow on the command line when executing it
* test.input - test input to send to the sample flow using STDIN

To run a specific sample only use `cargo run -p flowsamples {sample-name}`

### Testing
Testing samples consists of running them as above with the pre-defined arguments and standard input,
but it also gathers the standard output, standard error and files generated.

If there is any standard error found in the file test.err then the test will fail.
If there is no standard error then it compares standard output captured in test.output to expected.output
and fails if there is a difference.

If an expected.file exists then it compares it to file output in test.file and fails if there is any 
difference with the expected file.

You can test all samples by using `cargo test -p flowsamples -- --test-threads 1`, 
tests will be run one at a time.
```
cargo test -p flowsamples -- --test-threads 1 
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