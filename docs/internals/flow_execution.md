## Flow Execution

### Lazy Execution
Execution should be as lazy as possible.

The only thing that determines if a function is run is the availability at its inputs of the data
for the run, and the ability to produce the result at its output by having the output free.

If the output is free because a second function is running and hasn't consumed its input, then 
the first function will be blocked and computing resources will be used on the functions that most 
need it. When they complete and produce an output, that output may satisfy another functions' 
inputs which in turn will run, and so on and so forth.

### Execution States
Functions can be in one of two states:
- blocked (either input is pending or output is blocked)
- ready (inputs are satisfied, output is free and it can be run anytime)

### Value Rules
A Value has only one input, but that can be connected to a value offered by multiple "writers".
It has only one output, but that can be connected to and listened on by multiple "listeners"

It stores a value, that can be initialized when the program is loaded to an initial value.

When the value is empty it can be updated.
When it is updated, the value is made available to all "listeners" at its output.
While it stores a value, it cannot be updated, and any writer will be blocked until the value is 
consumed and can be updated again.

Each of the listeners can read or "consume" the value once.

When it has been consumed by all listeners, the value becomes empty (`None`) and can be updated again.
It does not become empty until all listeners have consumed the value.

You can think of Values as a FIFO of size 1 for each listener connected to it.

### Function Rules
The operation of a Function is similar to that of a value, except that it can have multiple inputs
and it is not run until they are all satisfied.

A function does not store any value or state, beyond making it's output available to listeners
asynchronously.

A Function can have zero or more inputs. 
Each input can be connected to and written to by multiple "writers".
It has only one output, but that can be connected to and listened to by multiple "listeners".

A Function can only run when a value is available at each of it's inputs and it's output is 
free to write to.. It is blocked from running until these conditions are met.

When a function runs, it produces an output that is made available to all "listeners" at it's output.

Each of the listeners can read or "consume" the output value once.

### Generalized Rules 
If we consider a value to be like a null function that does no calculation, but just passes the input 
value to it's outputs - then we can state some general rules that apply to the "running" of both.

- Can have zero or more inputs (max 1 for a Value)
- Each input can be connected to and values offered to it by multiple "writers".
- Has one output, that can be listened on by multiple "listeners".
- Can only be run (updated) when a value is available at each of the inputs and the output is 
free to write to. Is blocked from running until these conditions are met.
- When ran, it produces an output that is made available to all "listeners" at its output.
- Each of the listeners can read or "consume" the output value only once.
- Once the output has been consumed (once) by all of the listeners, then the output is free to be
written again.

## Execution Process
### Loading
All functions and values are loaded.

### Initialization
Any values with initial values are initialized with them and hence make them available on their 
output, and hence they are made available on the input of all connected objects (values and functions).

Now, the execution loop can be started.

### Execution Loop
Next ready Function is run
- Next function on the read list is run
    - this consumes all its inputs
        - this may unblock another function which was blocked sending to this functions as it's input was full
- Any data produced is made available on the output
- Outputs are made available to all connected inputs on other functions
    - The data on any output is made available to all connected inputs, copied if necessary to multiple.
    - This may satisfy the inputs of the other function, causing it to be added to the ready list

### Parallel Execution
A core goal of 'flow' is to enable parallel execution of programs, with the parallelism being described
inherently in the flow description, via data dependencies and functions with zero side effects.

Currently, the run-time only executes one function at a time, but that is destined to change as soon
as it can be implemented, both in multiple threads in the same process on one machine, then multiple
processes on one machine and then across machines across a network.

### Termination
The execution of a flow terminates when there are no functions left on the ready list
