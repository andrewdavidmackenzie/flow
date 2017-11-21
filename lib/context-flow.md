# Flow Definition
A full flow program must start with a context at the root. But flows can be defined as reusable components to be 
included in multiple contexts, or flows (as sub-flows)

## Name
A string used to identify an element.

## Element Types
The elements that can make up a flow program are:
- Context
- Flow
- Entity
- Connection

## Context Definition
This defines the entities external to the program with which it interacts and should provided 
by the run-time, or bundled via library.

By definition, nothing enters or leaves the context, at most things flow between the contained 
flow and entities in the context, as defined by the connections.

It can contain 0 or one sub-flows which are described in separate "flow" files.
The entities in the context are connected to inputs and outputs
of the sub-flow by connections.

A context contains:
- name       - String naming this context (obligatory)
- flow       - 0 or 1 contained flows (as flow references)
- entity     - A series of entities in the context
- connection - A series of connections between IOs on entities or flows

## Flow Reference
A reference to a flow defined elsewhere
name - a name that is used for display and referencing purposes within the context/flow it is found.
source - the location where the flow is defined.

## Entity Reference
name
source ?? needed if it's provided by the runtime?
library entities?

## Connection
name - an Optional name for the flow
from = <entity>/<entity name>/<entity "port"> ??
to = <entity>/<entity name>/<entity "port"> ??

The type of a data flow is inferred from the two IOs that it connects (after checking they coincide)

An input IO can receive data from (i.e. be connected to) multipe outputs.

An output IO can be connected to multiple inputs (the data is copied to each one).

## Entity Definition
An Entity can provide value(s) via a function or value definition, or it can consume value(s) via a
function that interacts with the run-time.

name - the name of the entity
io   - one or more IOs

## IO
IOs produce or consume data of a specific type, and maybe implemented as values or functions.

name - the name of the IO
type - the type of data it consumes or produces
value: It can define a static value of one of the specified type so that no function is needed to be implemented
or
function: ?? point to an implementation of the function (that doesn't take any input arguments?)

### Values
A static value of the type defined for the IO.

### Functions
Functions (along with Values) are the most basic building block of a implementation. They exist at the very bottom 
(or leaves) of the flow definition.
Functions by definition can be run in parallel, with no side effects, acting on their inputs (if any) and generating
their outputs.
      
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

