## Function Definitions
Stored in the definition file referred to by the Function Reference's "source" field:
* `name`   - the name of the function
* `input`  - zero or more inputs (as per [IO](ios.md))
* `output` - one or more outputs (as per [IO](ios.md))
* `implementation` - where to find the implementation of the function, or inline?

A function can consume data on 0 or more inputs (it must have all available in order to run)
and then can produce data on 1 or more outputs.

### Runtime semantics
A Function won't be run by the runtime until all inputs are available.
When it is run it may produce a data value on it's output.

TODO

   - default output
   - named outputs

TODO 

think how to bundle multiple functions (like STDIO has 3).

TODO 

Describe destructuring output

### Function Implementations
Must be able to be invoked by flow, and implement a defined interface to be able to invoke them and get the results.
Rust or rust ffi to use functions from other languages?

TO Consider

specifying data types at all levels, or optionally, maybe at top level to make it very easy to 
determine the input/output "contact" of flow without having to load all the levels all the way done.