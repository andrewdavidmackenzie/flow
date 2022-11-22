# What is 'flow'

`flow` is a system for defining and running inherently parallel, data-dependency-driven 'programs'.

Wikipedia defines [dataflow programs](https://en.wikipedia.org/wiki/Dataflow_programming) as 

    "dataflow programming is a programming paradigm that models a program as a directed 
    graph of the data flowing between operations"

which pretty much sums it up.

A `flow` program is created by defining a directed graph of `processes` that process data and that are
connected by `connections.`

A `process` can have zero or more inputs and produces zero or one output. They have no side-effects.
There is no shared-memory.

In `flow` a `process` is a generic term. A `process` can be a `function` that directly implements the 
processing on data, or it can be a nested "sub-flow". 
i.e. Another `flow` definition, that in turn may contains `functions` and/or other `sub-flows`.
When we wish to refer to them indistinctly, we will use the term process `process`. When distinctions
need to be made we will use `function`, `flow` or `sub-flow`.

Thus, a `flow` is an organization object, used to hierarchically organize sub-flows and functions,
and `functions` are what actually get work done on data.

Flows can be nested infinitely, but eventually end in `functions`. Functions consist of a definition
(for the compiler and the human programmer) and an implementation (for the runtime to use to process data).

The `connections` between processes are explicit declarations of data dependencies between them.
i.e. what data is required for a process to be able to run, and what output it produces.

Thus a `flow` is inherently parallel, without any further need to express the parallelism of the described 
algorithm.

As part of describing the `connections`, I would like `flow` to be also visual, making the data 
dependencies visible and directly visually "author-able", but this is still a work in progress and a 
declarative text format for flow definitions was a step on the way and what is currently used.

Functions and sub-flows are interchangeable and nestable, so that higher level programs can be
constructed by combining `functions` and nested `flows`, making flows reusable.

I don't consider flow a "programming language", as the functionality of the program is created from the 
combination of functions, that can be very fine grained and implemented in many programming 
languages (or even assembly, WebAssembly or something else). 

Program logic (control flow, loops) emerges from how the processes are "wired together" in 'flows'. 

I have chosen to implement the functions included with `flow` (in the `flowstdlib` standard 
library and the `context functions` of the `flowr` flow runner) in in rust, but they could be in other
languages.

I don't consider `flow` (or the flow description format) a DSL. The file format is chosen for describing 
a flow in text. The file format is not important, providing it can describe the flow (`processes` and
`connections`).

I chose TOML as there was good library support for parsing it in rust and it's a bit easier on the eyes 
than writing JSON. I later implemented multiple deserializers, so the flow description can be in other 
formats (including json and yaml) and even to be able to mix and combine descriptions in multiple formats.

Q. Is it high-level or low-level? 

A. "Well...yes". 

The level of granularity chosen for the implementation of `functions` that flows are built from is arbitrary. 
A `function` could be as simple as adding two numbers, or it could implement a complex algorithm.

## Interchangeability of `functions` and `sub-flows` as `processes`
A number of simple primitive functions can be combined together into a flow which appears as a complex 
process to the user, or it could be a complex `funtion` that implements the entire algorithm in code in
a single function.

The users of the process should not need to know how it is implemented. 
They see the process definition of it's inputs and outputs, a description of the processing it performs,
and use it indistinctly.