## Function Definitions
A function is defined in a definition file that should be alongside the function's implementation files (see later)

### Function Definition Fields
* `function`   - Declares this files is defining a function and defines the name of the function.
  This is required to link the definition with the implementation and allow the flow compiler to be able to find
  the implementation of the function and to include it in the generated project. `name` must match exactly the name
  of the object implemented.
* `source` - the file name of the file implementing the function, relative to the location of the definition file
* `docs` - a markdown file documenting the function, relative to the location of the definition file
* `input`  - zero (for impure)|one (for pure) or more inputs (as per [IO](ios.md))
* `output` - zero (for impure)|one (for pure) or more outputs (as per [IO](ios.md))
* `impure` - optional field to define an impure function

### Types of Function Definitions
Functions may reside in one of three locations:
- A `context function` provided by a flow running applications, as part of a set of functions it provides to flows
to allow them to interact with the environment, user etc. E.g. `readline` to read a line of text from STDIN.
- A `library function` provided by a flow library, that a flow can reference and then use to help define the overall
flow functionality. E.g. `add` from the `flowstdlib` library to add two numbers together.
- A `provided function` where the function's definition and implementation are provided within
the flow hierarchy. As such they cannot be easily re-used by other flows.

### `Impure` (or `context`) functions
An impure function is a a function that has just a source of data (e.g. `stdin` that interacts with the execution 
environment to get the data and then outputs it) or just a sink of data (e.g. `stdout` that takes 
an input and passes it to the execution environment and produces no output in the flow).

The output of an impure function is not deterministic based just on the inputs provided to it but depends on the
system or the user using it.
It may have side-effects on the system, such as outputting a string or modifying a file.

In `flow` these are referred to as `context functions`because they interact with (and are provided by) the
execution context where the flow is run. For more details see [context functions](context_functions.md)

Impure functions should *only* be defined as part of a set of `context functions`, not as a function in a 
library nor as a provided function within a flow.

Impure functions should declare themselves impure in their definition file using the optional `impure` field.

Example, the `stdin` context function declares itself impure
```
function = "stdin"
source = "stdin.rs"
docs = "stdin.md"
impure = true
...
```

### `Pure` functions
Functions that are used within a flow (whether provided by the flow itself or from a library) must be `pure`
(not depend on input other than the provided input values nor have no side-effects in the system) and have 
at least one input and one output.
- If they had no input, there would be no way to send data to it and it would be useless
- If it had no output, then it would not be able to send data to other functions and would also be useless

Thus, such a `pure` function can be run anytime, anywhere, with the same input and it will produce the same
output.

### Function execution
Functions are made available to run when a set of inputs is available on all of its inputs. Then a job is 
created containing one set of input values (a value taken from each of it's inputs) and sent for execution.
Execution may produce an output value, which using the connections defined, will be passed on to the connected
input of one or more other functions in the function graph. That in turn may cause that other function to run
and so on and so forth, until no function can be found available to run.

### Default inputs and outputs
If a function only has one input or one output, then naming that input/output is optional. 
If not named, it is referred to as the default input. Connections may connect data to/from this input/output just
by referencing the function.