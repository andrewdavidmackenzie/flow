## Flowr Execution Test Flows

These are a number of test flows to exercise specific flow execution features and are not intended to be "real world 
programs" or solutions to problems.

They are contained in the [flowc/tests/test-flows](https://github.com/andrewdavidmackenzie/flow/tree/master/flowr/tests/test-flows)
folder in the code.

### Structure of each test
Each test flow of name 'test-name' is in its own folder, which includes:
* A `root.toml` file that contains the flow description
* Files used in the automated testing of each sample:
    * `test.args` any arguments to be passed to the flow when running it. If doesn't exist, no arguments are passed.
    * `test.stdin` any input supplied to the flow on standard input when running it. If doesn't exist, no input is sent.
    * `expected.stdout` the output that the flow is expected to produce when run

### Execution of each test
Each test flow will be compiled by `flowc` and the manifest generated in a temporary directory, then that manifest
will be ran by `flowr`, passing in the arguments supplied (if any) and piping in the standard input supplied
(if any). 

If any standard error is produced, the test will fail.

The standard output is captured and compared to `expected.stdout`. If they don't match the test will fail.