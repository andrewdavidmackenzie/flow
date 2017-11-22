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
- entity     - 0 or more entities references
- io         - 0 or more input/outputs of this flow to any parent
- connection - 0 or more connections between entities, sub-flows and ios (e.g. to parent)

## Name
A string used to identify an element.

## Flow Reference
A reference to a flow defined elsewhere
name - a String that is used for display and referencing purposes within the flow it is used in.
source - the location where the flow is defined.

## Entity Reference
name - a String that is used for display and referencing purposes within the flow it is used in.
source - the location where the flow is defined. (library entities?)

## IO
IOs produce or consume data of a specific type.

name - the IO Reference that is used to identify it in connections to/from it
datatype - the type of data it consumes or produces

## IO Reference
This uniquely identifies an IO from a flow or an entity and is used to define connections between them.

from = <entity>/<entity name>/<entity "port"> ??
to = <entity>/<entity name>/<entity "port"> ??

## Connection
name - an Optional name for the flow
from = IO Reference that this connection comes from
to = IO Reference that this connection goes to

The type of a data flow is inferred from the two IOs that it connects (after checking they coincide)

An input IO can receive data from (i.e. be connected to) multiple outputs.

An output IO can be connected to multiple inputs (the data is copied to each one when produced).

## Entity Definition
An Entity can provide value(s) via function or value definitions, or it can consume value(s) via a
function that interacts with the run-time.
They exist at the very bottom (or leaves) of the flow definition.

name - the name of the entity
io   - one or more IOs

Unlike IOs for flows, which as just points to connect different levels, an Entity is actually responsible
for generating the output (or processing the input) on an IO. So, it needs an implementation.

There are two types of implementations:
- value
- function

### Value
A static value of the specified type that is always available on an IO.

### Function
A function that can consume, or produce, data on an IO.

Consumes a data item from an input and processes it, interacting with the run-time, 
or produces a data item on an output that goes somewhere else.

Functions by definition can be run in parallel, with no side effects, acting on their inputs (if any) and generating
their outputs.

TODO how to define if it consumes or produces?
      
## Flow Definition
A flow contains:
- name       - String naming this context (obligatory)
- io         - a series of IOs of this flow to the parent flow/context
- flow       - 0 or 1 contained flows
- entity     - A series of entities in the context
- connection - A series of connections between IOs on entities or flows

# Implementation

## Types
By default flow supports rust types, but a package can provide additional named types (structs) building on
rust ones, or others.... providing the type definitions and functions using them can be compiled.

## Function Implementations
Must be able to be invoked by flow, and implement a defined interface to be able to invoke them and get the results.
Rust or rust ffi to use functions from other languages?

