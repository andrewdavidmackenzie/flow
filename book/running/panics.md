### Exceptions and Panics
Currently, there are no special provisions for handling or recovering from run-time exceptions.
The functions are implemented in rust and when they fail they will panic as usual in rust. 
The panic will be caught by the runtime and a crash avoided, and an error logged, but nothing else is done.

This may cause the result of the flow to not be what is expected, or to terminate early due to lack of jobs
to execute.