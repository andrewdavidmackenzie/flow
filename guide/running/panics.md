### Exceptions and Panics
Currently, there are no special provisions for handling or recovering from runtime exceptions.
The functions are implemented in rust and when they fail they will panic as usual in rust.

The runtime does catch the panic, report it via an ERROR log statement, with details, and then exit.