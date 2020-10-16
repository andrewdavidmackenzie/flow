## Test Sample flows

These are a number of test flows to ensure many things are still working.
 
In general they are very simple samples more related with exercising specific features that
demonstrating "real work programs" or solutions to problems.

They are contained in the flowc/tests/samples folder in the code.
[Github link](https://github.com/andrewdavidmackenzie/flow/tree/master/flowc/tests/samples)

### Structure of each test
Each sample of name 'text-name' is in its own folder, which includes:
* A test-name.md file that:
    * describes what it does
    * lists the features of 'flow' that this sample uses and demonstrates
* A ```test-name.toml``` file that contains the flow description
* Files used in the automated testing of each sample:
    * ```test-name.args``` the arguments to be passed to the flow when running it
    * ```test-name.stdin``` the input supplied to the flow on standard input when running it
    * ```test-name.expected``` the output that the flow is expected to produce when invoked with 
```test-name.args``` as command line arguments (via ```flowr```) and ```test-name.stdin``` 
sent to standard input

### Execution of each test
* Flow will be compiled with ```flowc``` and the manifest generated in a file names ```test-name.json```
* Flow will be executed with ```flowr``` and the standard output captured
* The standard output is compared to ```test-name.expected``` and if identical the test will pass. 
Any differences and the test will fail.