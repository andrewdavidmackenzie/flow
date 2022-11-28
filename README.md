[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)
[![codecov](https://codecov.io/gh/andrewdavidmackenzie/flow/branch/master/graph/badge.svg)](https://codecov.io/gh/andrewdavidmackenzie/flow)
[![Generic badge](https://img.shields.io/badge/macos-supported-Green.svg)](https://shields.io/)
[![Generic badge](https://img.shields.io/badge/linux-supported-Green.svg)](https://shields.io/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# Welcome!
Welcome to `flow`, for defining, compiling and running parallel, 
[dataflow programs](https://en.wikipedia.org/wiki/Dataflow_programming) like the one below, that is a visual 
representation generated by the compiler from the flow definition and rendered with graphviz) of a 
flow program to generate a sequence of fibonacci numbers.

If you are a programmer, your intuition will probably tell you a lot already about how `flow` works
without any explanation.
![First flow](first.svg) 

This is one of many samples that can be found in the flowsamples crate in `flow`, and the first thing
I got working (much to my own delight!).

## What is a `dataflow program`?
A data flow program consists of a graph of processes (hierarchical in this case, as a process within it can be another
graph of processes, and so on down) that act on data that flow between them on defined connections. 
- it is declarative and defines what processes are used, and how they are connected
- processes are small and single purpose and "pure". They get a series of inputs, execute an algorithm
  (probably written in some procedural language) and produce an output.
- The application used to run a flow (a "flow runner") provides ways for it to interact with the
execution environment via "impure" functions, for things like Stdio, File System, etc.

## What characteristics do they have?
Why is writing a `dataflow program` something interesting to explore in the first place? 

Well, data flow programs define the program in terms of the processing steps that needs to be done on data
and the dependencies between the data, making them inherently parallelizable and distributable (and in
my mind, kind of the minimal essence or expression of the algorithm). 

Processes only run on data when it is available, making them "event driven" (where the "event" is the availability
of data...or alternatively, the data expresses an event that needs processing done on it and some output created).
They are not focussed so much on the procedural steps that need to be done and the control flow of the same, 
but on the required transformations to the data and on data flow _through_ the program.

## What does the `flow` project include?
Currently, flows are defined declaratively in a text file (toml, json or yaml are supported) that is then compiled
to a flow manifest, which is executed.

The `flow` project includes:
- Compiler: a library and a binary (`flowclib` and `flowc`) for compiling flows
- Runner: a library and two binaries (`flowrlib`, `flowr` and `flowrex`) for running flows, including a 
command line debugger for debugging flows.
- Standard Library: `flowstdlib` library of pre-defined flows and functions that can be re-used in flows
- Samples: A set of sample flows to illustrate flow programming (more to come!)
- Docs: Extensive documentation in the [book](SUMMARY.md) documentation on defining flows, the runtime semantics, a 
  programmers guide, docs on tool command line options and how to use them, the `flowstdlib` library
functions and flows, `flowr`'s context functions and more. The guide, including linked Code docs and rust
"doc tests" are all published together online [here](https://mackenzie-serres.net/flow/docs/book_intro.html).
- How to build flow locally and contribute to it
- Internal design and how some things are implemented
 
## What made me want to do it?
You can read more about what made me want to do this project, based on ideas gathered over a few decades
on and off (combined with looking for a "real" project to use to learn rust!) in the guide's 
[Inspirations for flow](docs/introduction/inspirations.md) section. The core reason is: I wanted to know
if I could do it and make it work, having stopped being a Software Engineer many years ago, based on rough ideas and intuition I 
had in my head (no real formal knowledge in this area or reading of books and papers - that came later *after* I did it).

I implemented the runtime "semantics" as needed as I implemented the samples. It's been a journey of discovery:
of writing something like this (for me), learning rust in the process and learning how such a programming 
paradigm could work. I learned it could work, but requires a change in how you think about programming 
(with procedural programming so ingrained in us). Sometimes I struggled to think about relatively simple
algorithms in a completely new way. This reminded me of when I got stuck trying to write a loop in Prolog, in
University. If you're trying to write a loop ...."you're thinking about it wrong".

## Building `flow`
For more details on how to build flow locally and contribute to it, please see [building flow](docs/developing/building.md)
Install the dependencies with `make config`, then run `make`, which builds everything and installs the `flowc` and `flowr` and 
`flowrex` binaries.

## Running your first 'flow'
With `flowc` and `flowr` installed, you can run the 'fibonacci' sample flow using:

`flowc flowsamples/fibonacci`

You should get a fibonacci series of numbers output to the terminal.

The [first flow](docs/first_flow/first_flow.md) section of the guide walks you through it.

## Are we GUI yet?
Data-flow programming, declaratively defining a graph of `processes` (nodes) and `connections` (edges), fits
naturally with visualization of the graph (not the current text format). 
The ability to define a flow, execute it and view its execution and debug it with a visual tool would be great! 
This tool would avoid the "hard work" of writing flow definition text files, just producing the flow definition files formats
supported by the `flowc` compiler. I have ideas for an IDE and experimented a little, but that remains one big chunk of work
I'd like to work on at some point.

## What's next?
I generate ideas for ways to improve the project faster than I can implement things in my spare time,
so over time I accumulated many many issues in Github, and had to organize them into a
[github project](https://github.com/users/andrewdavidmackenzie/projects/2/views/1) with columns and 
to attack them kanban-style, to stop me going mad. 
I still have plenty left and continue to generate new ones all the time.

Probably the most important ones for external observers will be ones related to producing a GUI to make it
more approachable, adding new `context functions` to allow integrations with the wbe and other systems
being used, and providing more compelling samples closer to "real world problems"

## Feedback and/or Encouragement
You can open an issue or email me to let me know what you think.

If you want to encourage me, even with a "token gesture", you can ["patreonize me"](https://www.patreon.com/andrewmackenzie)

Thanks for Reading this far!

Andrew
