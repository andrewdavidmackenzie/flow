## What is 'flow'?

`flow` is a system for defining and running parallel, data-dependency-driven 'programs'.
 
A `flow` is a program created by a description of interconnected and communicating `processes`.

A process can have zero or more inputs and produces zero or one output. They have no side-effects.
There is no shared-memory.

The interconnections between processes are explicit declarations of data dependencies between them.
i.e. what data is required for a process to be able to run, and what output it produces.

Thus a `flow` is inherently parallel, without any further need to express the parallelism of the described algorithm.

As part of describing the interconnections, I would like `flow` to be also visual, making the data dependencies visible
and directly "authorable". Processes and sub-flows are interchangeable and nestable, so that higher
level programs can be constructed by combining primitive processes and nested 'flows', making flows reusable.

At the time of writing, I haven't been able to do the GUI for the visual creation, viewing, running and
debugging of flows - and flows are described in a textual intermediate format based on TOML. This format could
be used as the description format for a visual GUI though.

I don't consider it a "programming language" as the functionality of the program is created from the combination of many
low-level functions, that can be very fine grained and implemented in many programming languages (or even assembly, 
WebAssembly, LLVM-IR or something else). Most logic (control flow, loops) comes from how the processess are "wired together"
in 'flows'. I have chosen to implement my basic processes in 'flowstdlib' and the 'flowr' run-time in rust (but they could 
be in other languages).

I don't consider it (or the flow description format) a DSL. The file format is chosen for describing a flow in text.
The file format is not important, providing it can describe the flow.
I chose TOML as there was good library support for parsing it in rust and it's a bit easier on the eyes than writing
JSON. I have made provisions for the flow description to be able to be in other formats and even to be able to mix and 
combine descriptions in multiple formats. I started a JSON one but haven't had the need to implement it as 
TOML is working fine so far.

Q. Is it high-level or low-level? 

A. "Well...yes". 

The level of granularity chosen for the implementation of the primitive processes that flows are built from is arbitrary. 
A process could be as simple as adding two numbers, or it could implement a complex algorithm.

A number of simple primitive processes can be combined together into a flow (which is also a process) and hence appear
as a complex process to it's users. Or it could be a complex primitive process, implementing the entire algorithm in
code. The users of the process (complex primitive process, or a flow process built from multiple others) should not
have to know how it is implemented. They see the definition of inputs and outputs and use it indistinctly.