## Understanding a simple flow
The flow shown in the image in the previous section is a fibonacci series generator.

Here is a simple explanation of what's involved and how it runs. 

You can find a complete description of the 'flow' semantics used, and others, in the next 
section [Describing Flows](../describing/definition_overview.md)

### Top Level - Root
The top-level, or root, defines the interaction with the surrounding execution environment and the flow contents.

Other flows can be included under this level, via references to separate flow description files,
to enable encapsulation and flow reuse.

In this example there is no sub-flow included (for the sake of simplicity).

### Interaction with the execution environment
The top-level defines what the interaction with the surrounding execution environment is,
such as `stdout`, or other inputs/outputs provided by the flow runtime being used.

The only interaction with the execution environment in this example is the use of `stdout`.

`stdout` (Standard Output) is a function defined in the `context`, to which output can be sent for display.

When executing a flow using `flowc`, `stdout` is sent to the standard output of 
the process running `flowc`, hence it is displayed in the terminal if running from the command line.

Other runtime environments (e.g. The experimental `flowide-gtk` graphical IDE that you can find in
a separate project in GitHuib) may display the output in some other way.

### Functions
This flow uses two functions: `stdout` from `context` as described above, plus `add` from the flow library
called `flowstdlib` to add two integers together.

### Connections
Connections take the output of a function and send it to the input of another. When all the inputs of a function
have a value the function can run, and produce outputs for others (or not produce outputs, as in the case of the 
impure `stdout` function).
