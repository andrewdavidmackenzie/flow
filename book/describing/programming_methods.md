## Programming Methods
`flow` provides the following facilities to help programmers create flows:

### Encapsulation
Functionality can be encapsulated within a function at the lowest level by implementing it in code, defining
the function via a function definition file with it's inputs and outputs and describing the functionality 
provided by it in an associated markdown file.

Sets of functions, combined together to provide some defined functionality, can be grouped together and connected
in a graph in a flow, described in a flow definition file. This "flows"'s functionality can be defined via it's inputs
and outputs just like a function, and its functionality described in an associated markdown file.

Flow definitions in turn can reference and incorporate other flows, alongside functions, until the desired 
functionality is reached.

Thus functionality is encapsulated via a "process" definition file, where a "process" can be defined as a function or
a flow.

The mechanism to reference a process in a flow definition file is common for both types, and in fact the flow does
not "know" if the process referenced is implemented as a function or a flow. At a later date the functionality of 
the sub-process should be able to be changed from being a function to a flow (or vice versa) with no semantic difference
and no required change on the program and no impact to its execution result.

### Semantics of Output sending
WHen a job executes, and it's results received by the runtime, the ouput values (if any) are sent onto the
destination functions at the same time, before any other job's results are processes, and before creating any new 
jobs or dispatching new jobs.

The outputs of a function's jobs are all handled, and sent to their destinations at the same time.

### Value deserialization
If the output of a job's function is (say) and array of numbers (`array/number`) and it is connected in the flow 
graph to another function who's input is of type `number`, then that array may be deserialized into a stream of numbers
and sent to the destination one after another (all when the job result is being processed).

This can mean that the destination function's input gathers rapidly a number of inputs able to be used in job creation.

The values are sent in order of their appearance of the "higher order structure" (array) that contains them.

### Value wrapping
Conversely, if the output value is of lower order that the destination (say a `number` being sent to an input that
accepts `array/number`) then the runtime may "wrap" the single value in an array and send it to the destination.

### Job Creation
Jobs are created by gathering a set of input values from a function's inputs. The job is put into the ready_jobs queue
with the values, and a reference to the function's implementation.

The inputs values order at the function's inputs is the order the values were sent to those inputs. The order of jobs
created respects this order. So, the order of job creation for a function follows the order of values sent to that 
function.

When creating jobs, a runtime may decide to create as many jobs as can be created, and increase the potential for
parallel execution later.

Thus, for a stream of deserialized values at the function's input, the runtime may attempt to maximize parallelization
and creates as many jobs as inputs sets of values it can take. The order of the jobs created will follow the order of
the deserialized stream.

### Job Dispatch
Different jobs for the same function are independent of each other. They will be dispatched in the order of jobs
creation (which follows the order of input value arrival).

When dispatching jobs, a runtime can decide to dispatch as many jobs as possible, or limit the number, in order to
increase the potential for parallel execution of the jobs later.

This, many jobs maybe created from the deserialized stream, but the order of the jobs will follow the order of
the deserialized stream.

### Job Completion Order and Determinism
Jobs maybe executed by the same or a different executor, on the same or a different machine, with the same or 
different CPU architecture, with jobs being sent and results being received back over the network.

Thus, the order of job completion is not guaranteed to match the order of job creation. 

In the deserialized stream case, here the order maybe lost. Thus algorithms exploiting this parallelism in the 
execution, but requiring to preserve order of the stream for some reason may have to handle the order and preserving
it themselves (e.g. adding an index and later combining results using that index).

The order of a flow or sub-flow's output is determined by the data dependencies of the flow expressed in the graph.

Examples of ways to create determinism are:
- [fibonacci example](../../flowr/examples/fibonacci/root.toml) use of a feedback connection so that one value is used
in the calculation of the next value, thus guaranteeing the order of the series.
- [sequence example](../../flowr/examples/sequence/root.toml) use of a "data flow control" function (`join`) to ensure
that a string is not sent to the `stdout` function until a specific condition (`end-of-sequence`) is met.
  ```
  # Output a string to show we're done when the Sequence ends
  [[process]]
  source = "lib://flowstdlib/control/join"
  input.data = {once =  "Sequence done"}
  ```

In imperative, procedural programming we often either assume, or can rely on order, such as the order of execution
of statements within a for loop. But with `flow` and its focus on concurrency this is much less so. A series of jobs
(similar to the for loop example) to calculate a number of values, but they maybe all generated at once (or soon
after each other) and executed in parallel, with the calculations completing out of order.

Also, in flow libraries, such as `flowstdlib`, some functions are written differently from what you might expect,
don't assume order, and the results maybe different from what you expect. This is reflected in the naming of functions
also, such as `sequence` that is named carefully to communicate that the values are generated in a specific order.
The `range` function does not guarantee order, only that all the numbers in the range will be output.
This it may generate the numbers in the range out of order, unlike what one would expect from a procedural language.

### Re-use
`flow` provides a number of mechanisms to help re-use, namely:
- definition and implementation of a function once, and then be able to incorporate it into any number of flows later
via a [process reference](process_references.md)
- definition of a flow, combining sub-flows and/or functions, into a flow and then be able to incorporate it into any 
number of flows later via a [process reference](process_references.md)
- definition of portable libraries containing flows and/or functions that can be shared between programmers and
  incorporate it into any number of flows later via [process references](process_references.md)

### Connection "branching"
As described in more detail in [connections](connections.md), a connection within a re-used flow to one of its
outputs can be "branched" into multiple connections to multiple destinations when the flow is compiled, without 
altering the definition of the original flow.

### Control flow via Data flow
In `flow`, everything is dataflow, and dataflow is everything. There are no other mechanisms to produce values,
or coordinate activity. There are no loops, if-then-else or other logic control mechanisms.

The [flowstdlib](../../flowstdlib/README.md) library provides the `control` module with a a series of 
functions and flows that you can use to control the flow of data, and hence program "control". 
These are functions such as:
- [compare_switch](../../flowstdlib/src/control/compare_switch/compare_switch.md)
- [index](../../flowstdlib/src/control/index/index.md)
- [join](../../flowstdlib/src/control/join/join.md)
- [route](../../flowstdlib/src/control/route/route.md)
- [select](../../flowstdlib/src/control/select/select.md)
- [tap](../../flowstdlib/src/control/tap/tap.md)

### Looping
Looping is not a natural construct in `flow`. If we look at how we would translate some use of loops from a 
procedural language to flow it might illustrate things.

For example, to perform an action or calculation 'n' times, we might well generate a range of 'n' values, create a
process that does the desired action or calculation, and then combine the two with a 'data flow control' function
such as `join`. Thus, the action/calculation can only produce an output for use downstream 'n' times, triggered
(possibly all in parallel) by the 'n' values that "gate" it's output.

### Accumulating 
In procedural programming a loop can be used to accumulate a value (such as the total of the values in an array).

In `flow` there i sno global state and no variables that are persistent for a function across multiple invocations 
of it.

The mechanism we use to do this in `flow` is to use the `add` function, initializing one input `Once` with zero, 
sending values to the other input, looping back the output (the partial sum) to the first input, so that the sum 
(initialized to zero) is accumulated as values flow through it.

The same technique can be used to gather values into "chunks" of a determined size. One input of `accumulate` is 
initialized with an empty array (`[]`), the other input receives the elements to gather, and we feed back the 
array of elements gathered so far, and so on until the desired size of chunk is accumulated. 

### Nested Loops
What would be a nested for loop in a procedural program can be implemented by putting two flows in series, with
one feeding the other. 

For example in the [sequence-of-sequences](../../flowr/examples/sequence-of-sequences/root.toml) 
example a first instance of a `sequence` flow generates a series of "limits" for sequence of sequences to count up to.

A value for the start of each sequence, and the series of sequence limits is fed into another instance of the 
`sequence` function. This second flow generates a sequence each time it receives a set of inputs specifying the start
and end of the sequence.
- a first sequence is defined with start=1, end=10, step = 1 and hence generates: 1..10
- a second sequence is defined
  - the start input is initialized always to 0
  - the step input is initialized always to 1 
- a connection is defined from the output of the first sequence to the `end` input of the second sequence
  - thus it generates 0,1,0,1,2,0,1,2,3    ending    0,1,2,3,4,5,6,7,8,9,10

### Wrapping processes for convenience
Another mechanism used for convenience (it may abbreviate written flows) is to have a simple flow to wrap a function or
process for a common use case, maybe initializing an input with a pre-defined value or creating feedback loops around 
the process to create a specific behaviour.