# Execution of a flow program

## Init
Any values with initial values make those available on their output, and hence they are 
made available on the input of all connected objects (values and functions).

Now, the execution loop can be started.

## Execution Loop
Statuses updated
- Status of functions/values (runnable) are updated based on availability of data at their inputs.
If they have data available at all inputs then their status is changed to runnable.

Run
- Functions/Values with status "runnable" are run
- Any data produced is made available on the outputs of the value or function
- The data on any ouput is made available to all connected inputs, copied if necessary to multiple.