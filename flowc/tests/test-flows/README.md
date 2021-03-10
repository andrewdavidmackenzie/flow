## Test flows

These are a number of test flows to exercise specific features and are not intended to be "real world programs" 
or solutions to problems.

They are contained in the [flowc/tests/test-flows](https://github.com/andrewdavidmackenzie/flow/tree/master/flowc/tests/test-flows)
folder in the code.

### Structure of each test
Each test flow of name 'text-name' is in its own folder, which includes:
* A ```test-name.toml``` file that contains the flow description
* Files used in the automated testing of each sample:
    * ```test-name.args``` the arguments to be passed to the flow when running it
    * ```test-name.stdin``` the input supplied to the flow on standard input when running it
    * ```test-name.expected``` the output that the flow is expected to produce when invoked with 
```test-name.args``` as command line arguments (via ```flowr```) and ```test-name.stdin``` 
sent to standard input

### Execution of each test
* The test flow will be compiled by ```flowc``` and the manifest generated in a file names ```test-name.json```
* The ```test-name.json``` manifest will be executed by ```flowr```, passing ```test-name.args``` as it's arguments
  and piping the contents of ```test-name.stdin``` to standard input, and standard output of the flow execution will 
  be captured.
* The standard output will be compared to ```test-name.expected``` and if identical the test will pass.
Any differences and the test will fail.