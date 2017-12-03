# Fibonacci Series

## Init
Initial values are made available in the inputs to ("HEAD" and "HEAD-1") by the run-time.

## Operation
Sstatuses updated
    :Status of functions/values (runnable) is updated based on availability of their inputs.
    - "HEAD" and "HEAD-1" are both made runnable

Functions/Values with status "runnable" are run:
    - "HEAD-1" is run. This updates it's value and makes the value available on it's outputs.
    - "HEAD" is run. This updates it's value and makes the value available on it's outputs.

Loop:
Statuses updated
    :Status of functions (runnable) is updated based on availability of their inputs.
    - "STDOUT" has all inputs available (from "HEAD-1") so is made "runnable".
    - "SUM" has inputs satisfied (from "HEAD" and "HEAD-1") so is made "runnable".
      
Functions/Values with status "runnable" are run:
    - "STDOUT" runs. No outputs produced. 
    - "SUM" runs. It produces a value on it's output.

Statuses updated
    :Status of functions/values (runnable) is updated based on availability of their inputs.
    - Status of functions (runnable) is updated based on availability of their inputs.
    - "HEAD" has inputs satisfied so is made runnable.

Functions/Values with status "runnable" are run:
    - "HEAD" is run. It produces a value in it's output.

Statuses updated
    :Status of functions/values (runnable) is updated based on availability of their inputs.
    - "HEAD-1" has input available so is made runnable.
    - "SUM" only has data on one input so is not runnable.
     
Functions/Values with status "runnable" are run:
    - "HEAD-1" is run. It produces a value in it's output.
    
Goto Loop:
