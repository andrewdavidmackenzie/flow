# Sample flows
The project includes a number of sample 'flows' that have been developed during the development
of the compiler and the run-time itself to drive the project development and then to demonstrate 
it working, and laterly as a type of regression test to make sure we don't break any of the 
semantics that the samples rely on.

They range from the extremely simple "hello-world" example to more complex ones like generation of
a fibonacci series, and others are in different stages of development and driving the needs for new
primitive functions and flow semantics.

## List of samples
* [args](args/DESCRIPTION.md)
* [arrays](arrays/DESCRIPTION.md)
* [fibonacci](fibonacci/DESCRIPTION.md)
* [first](first/DESCRIPTION.md)
* [hello-world](hello-world/DESCRIPTION.md)
* [hello-world-include](hello-world-include/DESCRIPTION.md)
* [mandlebrot](mandlebrot/DESCRIPTION.md) (WIP)
* [pipeline](pipeline/DESCRIPTION.md)
* [prime](prime/DESCRIPTION.md) (WIP)
* [primitives](primitives/DESCRIPTION.md)
* [range](range/DESCRIPTION.md)
* [reverse-echo](reverse-echo/DESCRIPTION.md)
* [router](router/DESCRIPTION.md) (WIP)

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