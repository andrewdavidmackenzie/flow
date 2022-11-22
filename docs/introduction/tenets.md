## Fundamental tenets of 'flow'?

The 'tenets', or fundamental design principles, of `flow` that I have strived to meet include:

## No Global or shared memory
The only data within `flow` is that flowing on the connections between processes. There is no way to 
store global state, share variables between functions nor persist data across multiple function invocations.

## Pure Functions
Functions have no side-effects (except `context functions` which I'll describe later). Jobs for functions
are created with a set of inputs and they produce an output, and the output should only depend on the input,
along the lines of "pure" functions in Functional Programming. Thus a function should be able to be 
invoked multiple times and always produce the same output. Also, functions can be executed by different
threads, processes, machines and machines architectures and always produce the same output.

This helps make flow execution predictable, but also parallelizable. Functions can be ran in parallel or 
interleaved without any dependency on the other functions that may have ran before, those running at the 
same time, or those in the future - beyond their input values.

This can enable novel tracing and debugging features also such as "time travel" (going backwards in a program)
or "un-executing" a function (stepping backwards).

## Encapsulation
The complexity of a process is hidden inside it's definition and you don't need to know it's 
implementation to know how to use it. 
- Public specification of a `process`: inputs and outputs for the compiler and user and a text description 
of what processing it performs on its inputs and what output(s) it produces, for the human programmer.
- Private implementation. A `process` implementation can be a `function` implemented in code or an entire 
sub-flow containing many sub-layers and eventually functions.

A process's implementation should be able to be changed, and changed from a function to a sub-flow or
vice versa without affecting flow programs that use it.

## Re-usability
Enabled by encapsulation. A well defined process can be used in many other flows via references to it.
Facilitate the "packing" of processes (be they functions or sub-flows) for re-use by others in other
flows.

## Portability
The intention is that the run-time can run on many platforms. The libraries have been written to
be able to compile to WASM and be portable across machines and machine architectures.

The function implementations in libraries are compiled to native for performance but also to WASM for
portability. Function implementations provided by the user as part of a flow are compiled to WASM once, 
then distributed with the flow and run by any of the run-times, making the flow portable without re-compilation.

## Polyglot
Although the compiler and runtimes are written in one language (rust), others versions could be written in 
other languages, there should be nothing in flow semantics or flow definition specific to one language.

Process implementations supplied with a flow could be written in any language that can compile to WASM, 
so it can then be distributed with the flow and then loaded and run by any run-time implementation.

## Functional Decomposition
Enable a problem to be decomposed into a number of communicating processes, and those in 
turn can be decomposed and so on down in a hierarchy of processes until functions are used. Thus the 
implementation is composed of a number of processes, some of which maybe reused from elsewhere and some specific to
the problem being solved.

## Structured Data
Data that flows between processes can be defined at a high-level, but consist of a complex structure or 
multiple levels of arrays of data, and processes and sub-processes can select sub-elements as input for their 
processing.

## Inherently Parallel
By making data dependencies between processes the basis of the definition of a flow, the non-parallel 
aspects of a flow (when one process depends on data from a previous process) are explicit, leading 
to the ability to execute all processes that *can* execute (due to the availability of data for them to 
operate on) at any time, in parallel with other executions of other processes, or of other instances 
of the same process.

The level of concurrency in a flow program depends only on the data structures used and the 
connections between the processes that operate on them. Then the level of parallelism exploited in its
execution depends on the resources available to the flow runner program running the flow.

## Distributable
As the functions are pure, and only depend on their inputs, they maybe executed across threads, cores,
processes, machines and (via portability and WASM) even a heterogeneous network of machines of different
CPU architectures and operating systems.

## Separate the program from the context
There is an explicit separation between the flow program itself, and the environment in which it runs.
Flows contain only pure functions, but they are run by a "flow runner" program (such as `flowr`) that
provides "impure" `context functions` for interacting with the context in which it is runs, for things
like STDIO, File System, etc.

## Efficiency
When there is no data to process, no processes are running and the flow and the flow runner program 
running it are idle.