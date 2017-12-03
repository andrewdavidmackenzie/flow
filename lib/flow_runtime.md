# Flow Execution

## Lazy Execution
Execution should be as lazy as possible, and only functions/values with all their inputs
satisfied are run, and output produced. This may satisfy another's inputs and they in turn run,
and so on and so forth.

Outputs that are not connected: the values are just discarded.

## Values
when a value's value is written to (initial value or an update) it is then made available on 
it's output.

When a value is made available on it's output, a copy is sent to each connection (other IO referenced)
on that output - even if only one of the consumers is ready to consume it. Thus values need to be
buffered on the outputs. So, a queue of values is maintained on the output, for each connection, or 
if we implement the actual connection - on it.

## Initialization
Initial values are made available in the inputs to the values and their state set to runnable.

## Execution Loop
Functions/Values with status "runnable" are run
Output produced is made available to all connected inputs
Status of Functions/Values are updated based on availability of data on all inputs



