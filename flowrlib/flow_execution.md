# Flow Execution

## Lazy Execution
Execution should be as lazy as possible, and only functions/values with all their inputs
satisfied and their outputs free are run, and output produced. 
This output may satisfy another's inputs and they in turn run, and so on and so forth.

## Execution States
Runnables (Values and Functions) can be in one of two states:
- blocked (either pending an IO or output is full)
- runnable (inputs are satisfied, output is free and it can be run anytime)

# Process
## Loading
All functions and values are loaded, and initially placed in "blocked" state.

## Initialization
Initial values are made available in the inputs to the values and hence their inputs are
satisfied and their state set to runnable.

## Values
When a value is made available to a value's input (either via initial value or the output
of something else), if it's output is free then the value's value is written (consuming the 
value from the output) and it's value is then made available on it's output.




When a value is made available on it's output, a copy is sent to each connection (other IO referenced)
on that output - even if only one of the consumers is ready to consume it. Thus values need to be
buffered on the outputs. So, a queue of values is maintained on the output, for each connection, or 
if we implement the actual connection - on it.

## Execution Loop
Outputs produced are made available to all connected inputs
Status of Functions/Values are updated based on availability of data on all inputs
Functions/Values with status "runnable" are run, producing outputs.


