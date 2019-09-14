## Understanding a simple flow
The flow shown in the image in the previous section is a fibonacci series generator.

Here is a simple explanation of what's involved and how it runs. 

You can find a complete description of the 'flow' semantics used, and others, in the next 
section [Describing Flows](../describing/definition_overview.md)

### Top Level - Context
The top-level, or root, of a flow is called the Context. This term is stolen from 
"Structured Analysis" (see [Inspirations section](../introduction/inspirations.md)) and it's
purpose is to define what the interaction with the surrounding execution environment is.

Other flows can be included under this level, via references to separate flow description files,
to enable encapsulation and sub-flow reuse.

In this example there is no sub-flow included (for the sake of simplicity).

### Interaction with the execution environment
This top-level defines what the interaction with the surrounding execution environment is,
such as STDIO, or other inputs/outputs provided by the flow runtime being used.

The only interaction with the execution environment in this example is the use of STDOUT.

STDOUT (Standard Output) is an output (input to a function) defined in the flow standard library (flowstdlib), 
to which output can be sent for display.

When executing a flow using `flowc`, STDOUT is printed sent to the standard output of 
the process running `flowc`, hence is displayed in the terminal when running from the command line.

### Values
The boxes labelled "HEAD" and "HEAD-1" in this example are examples of values. They store a simple
value. In this example they are both initialized with the integer value 1. Initialization of all
initialized values happens before flow execution starts.

### Functions

### Connections

