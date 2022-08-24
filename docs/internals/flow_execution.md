## Flow Execution
In this section we describe how a flow is executed by a flow runner.

### Components of Execution
A compiled flow consists of:
- Context functions - references to functions provided by the flow runner
- Flow functions - references to functions used in the flow
- Connections - connections between a function's output and one or more other functions' inputs
- Jobs - created by the runner as execution progresses

#### Context Functions
Context functions are functions provided by the flow runner program for interacting with the surrounding
execution environment, such things as standard IO, file system, etc. 

These are "impure" functions where the outputs are not derived solely from the inputs. 
Some of them will have inputs only (e.g. a stdout "print" function).
Some of them will have outputs only (e.g. a stdin "readline" function)
None of them will have inputs AND outputs.

#### Flow Functions
A flow is a graph of connected functions (including Context functions) where outputs of one function are
connected to inputs of another. A flow references the functions used, that may come either from a library 
or provided by the flow itself via custom source functions that are compiled to WASM for running by the
flow runner.

These functions are all "pure" with no side-effects and the outputs are solely derived from the inputs, 
in a reliable way.
A function does not store any value or state.

Such functions must have one or more defined inputs and an output, as a (non-Context) function without
an input cannot receive data to run with and will never be invoked and an (non-Context) function
without an output has no effect and does not need to be run.

A function has only one output. But the output may produce structured data, and a connection can be
made from a data element to another function's input using the "output route" selector.

A functions output will be connected to one or more other functions inputs. It is possible to connect 
a functions output back to one of it's input for the purpose of recursion or iteration. These are
called "loopback" connections.

A Function can only run when a value is available at each of it's inputs and the destinations it sends values
to are all free to send to. It is blocked from running until these conditions are met.

When the (job that uses the) function completes, it will produce an optional output value. If so, a 
copy of this output value (or part of it via an "Output route") will be sent to each connected 
destination function's input, possibly enabling them to run.

##### "RunAgain" and the Completed State
A function also returns a "RunAgain" value that indicates if it can/should be run again by the runner. 
This is usually for use in Context functions, such as the case of reading from standard in, say using "readline".

The "readline" function can be invoked many times, each time it will read a line of text from the standard
input and return TRUE for RunAgain, until EOF is reached when it will return FALSE for RunAgain.

When that happens the runner will put the function in the `Completed` state and it will not be invoked
again for the remained of this flow's execution.

##### Input Initializers
A functions inputs in a flow can be configured with an "Input Initializer", they are not part of the function's
definition, allowing them to be re-used in multiple flows or locations in a flow with different 
initializers.

An initializer can be of type "Once" where the input value is initialized with the provided value just once,
or of type "Constant", in which case the input is re-initialized with the provided value each time after
it has run. Once an input is supplied the value from the initializer, everything functions the same as if
the value had come from another function's output.

#### Connections
Values in the flow graph proceed via connections between function's outputs and functions inputs.
An output can be the entire value produced or a part of it selected via an "Output Route".
On a given execution a function may produce or not an output.
Also, the output data structure may vary and an "Output Route" may or may not have data in it.

If no value present, then nothing is sent to the destination function's input and it will remain waiting.

#### Jobs
Jobs are created in order to execute a function, with a given set of inputs.
Initially they contain inpout values and a reference to the function to execute.
Once run, they will also contain the results ("RunAgain" and an Optional value produced)

### Generalized Rules
- Functions can have zero or more inputs
- Each input can be connected to one or more Outputs that may send values to it during execution
- Each (non-Context) function has one output, that can send values to one or more destinations
- Non-Context functions must have 1 or more inputs and an output
- Connections to destinations may consume the whole output value/struct, or may select a portion of it using a route
- If no output is produced, or there is no value at the selected route, then nothing is sent to destinations
- Can only be run (via a Job) once a value is available at each of the inputs and the output is
  free to send to the destinations it is connected to. Is blocked from running until these conditions are met
- Once ran, it produces an output that is sent to all destinations connected to its output
- Each of the destinations consumes (a copy of) the output value only once
- Once the output has been consumed (once) by all of the destinations, then the function may be ran again
- The only thing that determines if a function is available to run is the availability of data at its inputs, and the ability to produce the result at its output by the destination inputs being free.
- If a destination function hasn't consumed its input, then the first function will be blocked. 
- A flow's execution ends when there are no functions left in the "ready" state available for execution

### Parallelized Execution
A core goal of 'flow' is to enable parallel execution of programs, without explicitly programming the
parallel execution, but allowing the inherent parallelism in an algorithm to occur.

This is possible due to a flow definition being done by describing functions on data, with the data dependencies
being explicit via "Connections" between functions and execution not occurring until data is available.

Thus, multiple instances of functions (via Jobs containing input data then output results) maybe executing 
in parallel as governed by the data dependency and execution rules above, and in fact multiple instances of
the same function (in different jobs) maybe executing in parallel.

The level of parallelism is determined by the algorithm as defined in the flow, the flow execution rules
and the number of cores in the execution machine(s) executing jobs.

#### Execution Order
Dataflow execution like that done by 'flow', and especially if parallel execution is performed, does not
guarantee any specific order of function/job execution or completion. Data dependencies expressed in the flow
should govern results.

This requires some unlearning of rules learned in previous procedural languages and some assumptions are
no longer valid. e.g. a Range of numbers from 0..10, could "appear" as data values in the graph as
3,8,1,2,0,9,6,5,7,4 instead of the expected 0,1,2,3,4,5,6,7,8,9. 

If a specific order is required in output data, then either the algorithm should enforce it inherently,
or some specific functions that impose order can be used (preferably just prior to output) at the expense
of parallelism.

At a given time, in the flow graph there can be a number of functions ready for execution and having 
Jobs created for them. They maybe executed in different orders by the runner, while still producing
"correct" output (e.g. if order of output is not important, two different orders of output values are both
considered "correct"). 

The 'flowr' runner two `ExecutionStrategy` that affect the order of job execution:
- "InOrder" - execution is in the order that they became ready to execute - first come first served
- "Random" - functions are selected at random from within the set of those `Ready`

Note that the time taken to execute different jobs may be different, and each may vary on a given machine
and of the flow is distributed across a network then other effects and other machines can affect Job execution, and 
hence Job completion time. So, beyond the different execution orders mentioned above, there are also no 
guarantees about job completion order. Flow programs should be programmed to be robust to this.

### Execution States
Prior to initialization, all functions will be in the `Initial` state. 

The Initialization step described below is run, after which all functions will be in one or more of the 
following states (see `State` struct in `run_state.rs`):
- `Ready` - Inputs are satisfied, the Output destinations are free and it can be run
- `Blocked`- One or more destination inputs this functions sends to is full, blocking execution
- `Waiting` - One or more of the inputs lack data, so the function cannot run
- `Running` - There is at least one job running that is using this function
- `Completed` - The function has returned FALSE for "RunAgain" and is not available for execution

## Execution Process
### Submission
A flow is sent for execution by a client application sending a `Submission` containing a reference to the 
compiled flow manifest to the runner application.

### Loading
All functions are loaded as they are read from the flow manifest. If they refer to library functions, then
they are loaded from the library reference (either a pre-loaded native implementation or a WASM implementation).

If they are WASM implementations supplied by the flow itself, then they are also loaded.

### Initialization
Any functions with "Input Initializers" ("Once" or "Constant" types) have the relevant inputs initialized 
with the specified value.

This may satisfy the function's need for input data. If it is not blocked sending to some destination then it
will be set into the `Ready` state.

Since the function's input is full, this may cause a block on other functions pending to send to that input.

Some Context functions that have no inputs (e.g. stdin "readline") may be placed immediately into the `Ready`
state (they are always ready until they return FALSE to "RunAgain).

Now, the execution loop is started.

### Execution Loop
A function in the `Ready` state is selected to run (depending on the `ExecutionStrategy` discussed above).

A Job is created using the function's available input values and is sent for execution.
- this may unblock another function which was blocked sending to this functions as it's input was full

Jobs are created until either no function is available in the `Ready` state, or a maximum number of pending Jobs
is reached.

A blocking wait on completed jobs is performed. 
For each completed job that is received:
- Any output value in the Result (whole or using an "Output Route to select part of the data) is made available to 
  inputs on connected functions
    - This may satisfy the inputs of the other function, causing them to transition to the `Ready` state

If the function has any "Constant" initializer on any of it's inputs, it is run, possible refilling one or more
of its inputs.
According to the availability of data at its inputs and ability to send to its outputs a function may transition
to the `Ready` or `Waiting` (for inputs) or `Blocked` (on sending) state. 

The loop continues until there are no functions in the `ready`state, and the flow is terminated.

### Termination
The execution of a flow terminates when there are no functions left on the ready list.
Depending on options used and the runner, this may cause the output of some statistics, unloading
of loaded objects and either runner program exit, or return to wait for a `Submission` and the whole
process starts again.