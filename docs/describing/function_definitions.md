## Function Definitions
A function is defined in a definition file (e.g. `add`) that should be alongside the function's
implementation files (see later)

A function can consume data on 0 or more inputs (it must have all available in order to run)
and then can produce data on 0 or more outputs. 

A function can be a pure source of data (e.g. `stdin` that interacts with the execution environment 
to get the data and then outputs it) or a pure sink of data (e.g. `stdout` that takes 
an input and passes it to the execution environment and produces no output in the flow).

### Function Definition Fields
* `name`   - the name of the function. This is required to link the definition with the 
implementation and allow the loader/compiler to be able to find the implementation of the
function and to include it in the generated project. `name` must match exactly the name of the 
object implemented.
* `input`  - zero or more inputs (as per [IO](ios.md))
* `output` - one or more outputs (as per [IO](ios.md))

### Run-time semantics
A Function won't be run by the run-time until all its inputs are available.
When it is run it may produce a data value on it's output.

_TODO_
- default output
- named outputs

_TODO_ 
- Describe destructuring output

### Function Implementations
Must be able to be invoked by flow, and implement a defined interface to be able to invoke them and get the results.
Rust or rust ffi to use functions from other languages?

_TO Consider_
- specifying data types at all levels, or optionally, maybe at top level to make it very easy to 
determine the input/output "contact" of flow without having to load all the levels all the way done.