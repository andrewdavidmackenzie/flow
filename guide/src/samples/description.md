## Sample flows

The project includes a number of sample 'flows' that have been developed during the development
of the compiler and the run-time itself to drive the project development and then to demonstrate 
it working, and laterly as a type of regression test to make sure we don't break any of the 
semantics that the samples rely on.

They range from the extremely simple "hello-world" example to more complex ones like generation of
a fibonacci series, and others are in different stages of development and driving the needs for new
primitive functions and flow semantics.

They are contained in the [samples](https://github.com/andrewdavidmackenzie/flow/tree/master/samples) 
folder in the code.

### Structure of each sample
Each sample resides in it's own sub-folder of 'samples', and each one contains:
* A DESCRIPTION.md file that:
    * describes what it does
    * lists the features of 'flow' that this sample uses and demonstrates
* A ```context.toml``` file that is the root file of the flow description
* Files used in the automated testing of each sample:
    * ```test_arguments.txt``` the arguments to be passed to the flow when running it
    * ```test_input.txt``` the input supplied to the flow when running it
    * ```expected_output.txt``` the output that the flow is expected to produce when invoked with 
"text_arguments.txt" and input "test_input.txt"