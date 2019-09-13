## Fundamental tenets of 'flow'?

The 'tenets', or fundamental design principles, of `flow` include:

* No Global shared memory
* Processes have no side-effects (exception is IO processes which I'll describe later)
* Encapsulation - complexity of a Process is hidden inside it's definition and you don't need to know it's implementation
to know how to use it. In fact a Process can be a primitive process implemented as a single function in code or an 
entire sub-flow containing many sub-layers
* Re-usability - enabled by encapsulation. A well defined process can be used in many other flows via references to it.
* Portability - the intention is that the run-time can run on many platforms (and the libraries have been written to
be able to compile to WASM and run in a browser also) and that implementations of primitive Processses supplied by
the user can be compiled to WASM once, then distributed with the flow and run by any of the run-times
* Polyglot - although the compiler is written in one language, others versions could be written in other languages. 
There is only one runtime written at the moment, but others could be written in other languages to run flows compiled
by any compiler. Process implementations supplied with a flow can be written in any language that can compile to WASM, 
so it can then be distributed with the flow and then loaded and run by any runtime implementation.
* Functional Composition/Decomposition - a problem can be decomposed into a number of communicating processes, and those in 
turn can be decomposed and so on down in a hierarchy of processes until primitive processes are used. Thus the 
implementation is composed of a number of processes, some of which maybe reused from elsewhere and some specific to
the problem being solved.
* Data Composition/Decomposition - data that flows between processes can be defined at a high-level, but consist of a 
complex structure or multiple levels or arrays of data, and processes and sub-processes can select sub-elements as
input for their processing.
* Inherently parallel - this is a design goal that still needs a lot of work, but the idea is that the flow describes
the algorithm to execute via processes and connections between them and order of execution (obeying data dependencies
and dat aflow rules) falls out from the definition and can be executed in series (one process at a time) or parallel
(multiple processes executing at a time)