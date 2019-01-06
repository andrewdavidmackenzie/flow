## What is 'flow'?

`flow` is a system for defining and running parallel, data-dependency driven 'programs'.
 
A 'flow' is a program created by a description of interconnected and communicating "processes" or functions,
where functions have no side-effects except accepting (possibly) accespting inputs and (possible) producing outputs.

The interconnections between functions are explicit declarations of data dependencies between them.
i.e. what data is required for a function to be able to run, and what output it produces.

Thus a 'flow' is inherently parallem, without any further need to express the parallelism of the described algorithm.

As part of describing the interconnections, I would like 'flow' to be also visual, making the data dependencies visible
and directly "authorable". Functions and flows are nestable, so that higher
level programs can be constructed by combining primitive functions and nested 'flows' defined elsewhere, making flows
also reusable.

At the time of writing, I haven't been able to do the GUI for the visual creation, viewing, running and
debugging of flows - and flows are described in a textual intermediate format based on TOML. This format could
be used as the description format for a visual GUI though.

I don't consider it a "programming language" as the functionality of the program is created from the combination of many
low-level functions, that could be very fine grained and implemented in many programming languages (or even assembly, 
WebAssembly, LLVM-IR or something else). Most logic (control flow, loops) and comes from how the basic functions are 
combined in 'flows'. I have chosen to implement my basic functions (in 'flowstdlib' in rust, but they could 
be in other languages). I have made provisions for the code generation of "compiled" flows to be in multiple languages, with rust just
the first one I have implemented.

I don't consider it (or the flow description format) a DSL. The file format chosen for describing a flow in text files is
not important providing it can capture the flow description. I chose TOML as there was good library support for parsing
and generating it in rust. I have made provisions for the flow description to be able top be in other formats (and possibly 
even) to mix and combine descriptions in multiple formats. I started a JSON one but haven't had the need to implement it as 
TOML is working fine so far.

Q. Is it high-level or low-level? 

A. "Well...yes". 

The level of granularity chosen for the implementation of the primitive functions that flows are built from 
(both the ones I have is arbitrary and could be as simple as adding two numbers, or a function can implement a 
complex algorithm - but the 
implemented in 'flowstdlib' and custom functions provided as part of a flow description)
