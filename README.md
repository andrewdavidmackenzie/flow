[![Build Status](https://travis-ci.org/andrewdavidmackenzie/flow.svg?branch=master)](https://travis-ci.org/andrewdavidmackenzie/flow)
[![codecov](https://codecov.io/gh/andrewdavidmackenzie/flow/branch/master/graph/badge.svg)](https://codecov.io/gh/andrewdavidmackenzie/flow)
[![Generic badge](https://img.shields.io/badge/macos-supported-Green.svg)](https://shields.io/)
[![Generic badge](https://img.shields.io/badge/linux-supported-Green.svg)](https://shields.io/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# Welcome!
Welcome to `flow`, for defining, compiling and running parallel, 
[dataflow programs](https://en.wikipedia.org/wiki/Dataflow_programming) like the one below (that is a visual 
representation generated by the compiler from the flow definition and rendered with graphviz) of a 
flow program to generate a sequence of fibonacci numbers.

If you are a programmer, your intuition will probably tell you a lot already about how `flow` works
without any explanation.
![First flow](first.svg)
This flow program generates a fibonacci series on standard output.
It is one of the examples ([fibonacci](flowr/examples/fibonacci/DESCRIPTION.md)) in the `flowr` crate 
that is part of the`flow` project, and the first thing I got working (much to my own delight!).

The two inputs to `add` (`i1` and `i2`) are initialized "once" (at startup) with 0 and 1. 
The output (`sum`) is then fed back to input `i2` and the value presented at input `i2` previously is fed back to
input `i1`. 
The output (`sum`) is also sent to the default (unnamed) input of the `stdout` function which prints
the value to standard output.
The program runs until integer overflow causes no output to be produced and it stops.

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
- Runner: a library (`flowrlib`) and two binaries for running flows:
  - `flowrcli` - default command line runner and debugger to use from a terminal
  - `flowrgui` - a GUI application for running and debugging flows
- Job executor: `flowrex` binary can be discovered (on same machine or local network) 
by a runner and used to execute jobs, distributing execution in a basic fashion
- Standard Library: `flowstdlib` library of pre-defined flows and functions that can be re-used in flows
- Examples: A set of example flows to illustrate flow programming (more to come!)
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
if I could do it and make it work, having stopped being a Software Engineer many years ago, based on rough ideas 
and intuition I had in my head (no real formal knowledge in this area or reading of books and papers - 
that came later *after* I did it).

I implemented the runtime "semantics" as needed as I implemented the examples. It's been a journey of discovery:
of writing something like this (for me), learning rust in the process and learning how such a programming 
paradigm could work. I learned it could work, but requires a change in how you think about programming 
(with procedural programming so ingrained in us). Sometimes I struggled to think about relatively simple
algorithms in a completely new way. This reminded me of when I got stuck trying to write a loop in Prolog, in
University. If you're trying to write a loop ...."you're thinking about it wrong".

## Installing
You can install many of the crates from crates.io, but due to unresolved issues in packaging
non-source files, a total working installation cannot yet be achieved using `cargo install`.

The workaround in the meantime is to clone the repo and build all from source (see below).

## Building `flow`
For more details on how to build flow locally and contribute to it, please see
[building flow](docs/developing/building.md)
Install the dependencies with `make config`, then run `make`, which builds everything and installs the `flowc` and
`flowr` and`flowrex` binaries.

NOTE: Building of `flowstdlib` the first time will take a long time, as it is compiling
many rust functions to WebAssembly.

## Running your first 'flow'
With `flowc` and `flowr` installed, you can run the 'fibonacci' example flow using:

`cargo run --example fibonacci`

You should get a fibonacci series of numbers output to the terminal.

The [first flow](docs/first_flow/first_flow.md) section of the guide walks you through it.

## Tech decisions
### Job/Work Distribution - with Threads
Flow was started before async landed in rust, and so it uses a manually managed thread pool for executing 
"jobs" (functions with their set of inputs). Rewriting in async rust would make sense in some areas but
be quite a chunk of disruptive work, so I haven't done it yet.

### Message Passing - with Zero MQ
I started with channels for distributing Jobs and results between threads.
I wanted to enable distributing work across the (local for now) network and so moved to ZeroMQ message queues 
and passing messages. This is used for inter-thread and inter-process message passing indistinctly. 
ZeroMQ rust bindings don't support all socket types (at the time of writing) so I had to use the REQ/REP
pattern, which has some restrictions on the protocol, and which end writes first - which I also had to
work around. For a while I kept 

### Discovery - with mDNS and beacons
To discover "executors" (processes with threads, able to execute flow jobs) on the network I wrote my own 
small discovery crate (as I couldn't get libp2p mDNS or other mDNS crates to work). Not very happy with it
as it frequently ties up ports and other issues I have had to work around.

### Portability - with WebAssembly (WASM)
Library functions are compiled both to native and optionally linked statically to a flow runner with a 
feature, AND compiled to wasm (and their size optimized to around 110KB) and described in a library manifest.
Libraries are referenced from a flow's compiled manifest and if the library is already statically linked then
the native implementations can be used, or the WASM supplied files can be used, under control of an option.
I have used this to have a flow program running on my mac, and with the flowrex job executor running on a
connected RaspberryPi running native or WASM. 

When a user writes a new flow and includes a "provided implementation" (a custom function used in the flow), 
they write it in rust and it is compiled to WASM and loaded at run time.

### Client - Server
I knew I wanted to be able to distriubute flow execution between processes, and I know I wanted to have the
ability to have a background process coordinate execution and execute jobs, and have different UIs (CLI, GUI)
and be able to use standard input/output from CLI. So, the "context functions" (impure functions that interact
with the environment where a flow runs) are implemented in the "runner" and can be CLI or GUI implementations.
That lead to some messy client/server message passing, that is now pretty stable and works on both CLI and GUI
with the same backend (in ºflowrlibº) coordinating a flow and executing jobs - but with some complexity.

### Testing
Testing coverage is about 85%-90% and I try to keep it high. There are simple unit tests for functions, a
lot of tests around the compiler semantics, integration tests of flow compile/run errors, and of flows
that compile & run correctly, and the examples all have supplied test inputs files and expected output files
and they are all tested to work correctly on every build. Additionally there are some integration 
tests of the debugger and of executing a flow in client-server mode (separate processes for each) and
the distribution of job execution using `flowrex`.

## GUI (`flowrgui`)
Data-flow programming, declaratively defining a graph of `processes` (nodes) and `connections` (edges), fits
naturally with visualization of a graph. 
The ability to define a flow, execute it and view its execution and debug it with a visual tool would be great! 
This tool would avoid the "hard work" of writing flow definition text files, just producing the flow definition 
files formats to be compiled by the `flowc` compiler. 

I have started work on a native rust GUI using the `Iced` 
toolkit. Initially, it is focussed only on running flows and replaces the Terminal based stdio, file output 
and image operations with visual equivalents.

I hope to add flow design and programming to it, using `flowrclib`, either in a single
binary, or in a second compile-time-only tool.

## Docs
Apart from this README, I have written pretty extensive documentation in a "guide" or book 
([Table of Contents](SUMMARY.md)), using Markdown
and mdbook. That describes writing flows, using flow, general ideas, the standard library functions,
the examples etc. It's hard to keep up to date but I try and am always generating GH issues for myself to
improve it! It doesn't go into two many details on the implementation, to reduce the burden of keeping it up to date.

The docs combine Markdown files from the code repos, Markdown files in the docs folder, and code docs 
generated with rustdoc and image files I generate from "dot" files and graphviz into one "book". That is rebuilt
and published to the ["flow guide"](https://mackenzie-serres.net/flow/docs/book_intro.html) on every merge to
master.

## What's next?
I generate ideas for ways to improve the project faster than I can implement things in my spare time,
so over time I accumulated many issues in GitHub, and had to organize them into a
[GitHub project](https://github.com/users/andrewdavidmackenzie/projects/2/views/1) with columns and 
to attack them kanban-style, to stop me going mad. 
I still have plenty left and continue to generate new ones all the time.

Probably the main areas of work short-term will be on the GUI, enabling me to learn `Iced`
in the process.

Other main themes of items in the [GitHub project](https://github.com/users/andrewdavidmackenzie/projects/2/views/1) 
are related to adding web functionality, better example programs, packaging/distribution/install,
and true distributed flow execution with flow partitioning based on resources and constraints.

## Contributing
Refer to the [contributing](docs/developing/contributing.md) section of the book.

## Feedback and/or Encouragement
You can open an issue or email me to let me know what you think.

If you want to encourage me, even with a "token gesture", you can
["patreonize me"](https://www.patreon.com/andrewmackenzie)

Thanks for Reading this far!

Andrew
