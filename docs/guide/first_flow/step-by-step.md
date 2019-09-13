## Step-by-Step

Execution is in terms of Functions. Values are in fact a specific implementation of a
Function, as Values have to store values, unlike functions.

### Init
The list of Functions is loaded and all are initialized. 
This includes making the initial values available (just once) at 
the inputs of any values that have initial values specified in the flow description.

Status (ready to run, pending inputs, blocked etc) of all functions is set based on availability of their inputs and 
not being blocked from sending its output.

### Execution Loop
In general, the execution loop takes the next function that is in the ready state 
(has all its input values available, and is not blocked from sending its output by other functions) and runs it.

That consumed the inputs and sends the output value to all functions connected to the output. That makes that input
available to the other function connected to the output, and it may make that other function ready to run.

When there are no more functions in the ready to run state, then execution has terminated and the flow ends.

### Specific Sequence for this example
#### Init:
* Initial values of 1 are made available in the inputs of "HEAD" and "HEAD-1" values.
* HEAD-1 has input (1) available and is not blocked from sending its outputs, so it is made ready to run.
* HEAD has input (1) available and is not blocked from sending its outputs, so it is made ready to run.
* STDOUT does not have an input value available so it is not "ready"
* SUM does not have its inputs available so it is not "ready"

#### Loop Starts
ReadyList = HEAD-1(1), HEAD(1)

Next function with status "ready" is run:

- HEAD-1 is run with input 1
    - HEAD-1 makes the value 1 available on its output (to STDOUT and SUM)
        - HEAD-1 is now blocked from running again until its output to SUM is free
        - STDOUT has all inputs available (from "HEAD-1") so is made "ready"
        - SUM(1,_) only has one of its inputs available, so it is not made "ready"

ReadyList = HEAD(1), STDOUT(1)

Next function with status "ready" is run:

- "HEAD" is run with input 1
    - This updates its value and makes the value 1 available on its outputs (to HEAD-1 and SUM)
        - SUM(1,1) now has both inputs available (from HEAD and HEAD-1) so it is made "function"
        - HEAD-1(1) has an input value available (from HEAD, but it cannot run as its output is blocked by SUM, 
    so it is "blocked on output" and not "ready".

ReadyList = STDOUT(1), SUM(1,1)

Next function with status "ready" is run:

- "STDOUT" runs with input 1. It prints "1" on the stdout of the runtime.
> 1

ReadyList = SUM(1,1)

Next function with status "ready" is run:

- "SUM" runs with inputs 1 and 1. It produces the value 2 on its output (to HEAD)
    - SUM running consumes its input and unblocks HEAD-1(1) from running
    - HEAD has its input available so is made "ready" with input 2

ReadyList = HEAD-1(1), HEAD(2)

Next function with status "ready" is run:

- HEAD-1 is run with input 1. It produces 1 on its output (to STDOUT and SUM)
    - STDOUT(1) has its input available so is made "ready"
    - SUM(1, _) only has one input available and so is not "ready"

ReadyList = HEAD(2), STDOUT(1)

Next function with status "ready" is run:

- "HEAD" is run with input 2. It produces 2 on its output (to HEAD-1 and SUM)
    - SUM(1,2) is made "ready"
    - HEAD-1(2) is blocked on sending by SUM

ReadyList = STDOUT(1), SUM(1,2)

Next function with status "ready" is run:

- "STDOUT" runs with input 1. It prints "1" on the stdout of the runtime.
> 1

ReadyList = SUM(1,2)

Next function with status "ready" is run:

- SUM runs with inputs 1 and 2. It produces the value 3 on its output (to HEAD)
    - HEAD-1(2) has its output unblocked by SUM and so is made "ready"
    - HEAD(3) has its input available so is made "ready"

ReadyList = HEAD-1(2), HEAD(3)

- HEAD-1(2) is run. It produces 2 on its output (to STDOUT and SUM)
    - STDOUT(2) has its input avaialble so is made "ready"
    - SUM(2, _) lacks an input and is not ready

ReadyList = HEAD(3), STDOUT(2)

- HEAD(3) is run. It produces 3 on its output (to HEAD-1 and SUM)
    - SUM(2, 3) is made "ready"
    - HEAD-1(3) is blocked on SUM so not "ready"

ReadyList = STDOUT(2), SUM(2,3), HEAD-1(3)

Next function with status "ready" is run:

- STDOUT(2)) runs. It prints "2" on the stdout of the runtime.
> 2

ReadyList = SUM(2,3)

Next function with status "ready" is run:

- SUM(2,3) is run. It produces the value 5 on its output (to HEAD)
    - HEAD-1(3) has its output unblocked by SUM and so is made "ready"
    - HEAD(5) has its input available so is made "ready"

ReadyList = HEAD-1(3), HEAD(5)

Next function with status "ready" is run:

- HEAD-1(3) is run. It produces 3 on its output (to STDOUT and SUM)
    - STDOUT(3) has its input avaialble so is made "ready"
    - SUM(3, _) lacks an input and is not ready

ReadyList = HEAD(5), STDOUT(3)

Next function with status "ready" is run:

- HEAD(5) is run. It produces 5 on its output (to HEAD-1 and SUM)
    - SUM(3, 5) is made "ready"
    - HEAD-1(5) is blocked on SUM so not "ready"

ReadyList = STDOUT(3), SUM(3,5)

Next function with status "ready" is run:

- STDOUT(3)) runs. It prints "3" on the stdout of the runtime.
> 3

and so on, and so forth.... producing a fibonacci series on the standard output of the runtime:
> 1, 1, 2, 3, 5, 8 ...
