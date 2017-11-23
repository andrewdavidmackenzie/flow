# Flow Definition

A flow can define the entities external to the program with which it interacts and should be provided by the run-time, 
or bundled via a library. i.e. they are not included in the flow, but the flow interacts with them as part of it's
run-time "context".

By definition, nothing enters or leaves the top-level flow, at most things flow between the contained 
flow and entities referenced, as defined by the connections.

A flow contain 0 or more sub-flows which are described in separate "flow" files.
The entities referenced in the top-level flow definition are connected to inputs and outputs of any sub-flow by 
connections.

So, valid entries in a flow definition include:
- name       - String naming this flow (obligatory)
- flow       - 0 or more contained flow references
- io         - 0 or more input/outputs of this flow to any parent
- connection - 0 or more connections between entities, sub-flows and ios (e.g. to parent)
- function   - 0 or more functions referenced in this flow.
- value      - 0 or more values contained in this flow

TODO
?? Function implementing what's inside the flow entirely????

## Name
A string used to identify an element.

## Flow Reference
A reference to a flow defined elsewhere
name - a String that is used for display and referencing purposes within the flow it is used in.
source - the location where the flow is defined.

## IO Reference
This uniquely identifies an IO from a flow/value/function and is used to define connections between them.
e.g. ObjectType/Flow.name/IO.name
e.g. ObjectType/Value.name
e.g. ObjectType/Function.name/IO.name

Where ObjectType can be "flow", "value", "function".
For IOs within the existing flow, "this" is used as the Flow.name.

IO References are only used in specifying connections.

## IO
IOs produce or consume data of a specific type, and are where data enters/leaves a flow/value/function.

name - the IO Reference that is used to identify it in connections to/from it

## Connection
name - an Optional name for the flow
from = IO Reference that this connection comes from
to = IO Reference that this connection goes to

The type of a data flow is inferred from the two IOs that it connects (after checking they coincide)

An input IO can receive data from (i.e. be connected to) multiple outputs.

An output IO can be connected to multiple inputs (the data is copied to each one when produced).

## Function Reference
name - the name of the function.
source - the source file where it is implemented

A function can consume data on 0 or more IOs (it must have all available in order to run)
and then can produce data on 1 or more IO.

TODO
Pure functions (no side effects?)
IO functions that interact with the system it's running on (like Haskell)?

### Value
A static value of the specified type that is always available on an IO.
name - the name of the value
datatype - the type of the value
value - it's value

## Data Types
By default flow supports rust types, but a package can provide additional named types (structs) building on
rust ones, or others.... providing the type definitions and functions using them can be compiled.

## Function Definitions
Stored in the definition file referred to by the Function Reference's "source" field.

name   - the name of the function
input  - zero or more inputs
output - one or more outputs
implementation - where to find the implementation of the function, or inline?

inputs and outputs must have:
name - input/output name
datatype - what type this input/output consumes/produces

A Function is responsible for accepting input on it's inputs, waiting until all are fullfilled,
then running and producing data values on it's outputs.

TODO think how to bundle multiple functions (like STDIO has 3).

## Function Implementations
Must be able to be invoked by flow, and implement a defined interface to be able to invoke them and get the results.
Rust or rust ffi to use functions from other languages?

TO COnsider
specifying data types at all levels, or optionally, maybe at top level to make it very easy to 
determine the input/output "contact" of flow without having to load all the levels all the way done...