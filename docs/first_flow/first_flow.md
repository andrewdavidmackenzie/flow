# First flow
Without knowing anything about `flow` and its detailed semantics you might be able to guess what this flow 
below does when executed and what the output to STDOUT will be. ![First flow](first.svg)
It is a fibonacci series generator.

## Understanding the flow
NOTE:You can find a complete description of flow semantics in the next
section [Defining Flows](../describing/definition_overview.md)

### Root flow
All flows start with a root "flow definition". Other sub-flows can be nested under the root, via references to 
separate flow description files, to enable encapsulation and flow reuse.

In this case it is the only one, and no hierarchy of flows descriptions is used or needed.
You can see the TOML root flow definition for this flow in the flowsample crate's fibonacci sample.
[root.toml](../../flowsamples/fibonacci/root.toml)

### Interaction with the execution environment
The root defines what the interaction with the surrounding execution environment is,
such as [Stdout](../../flowr/src/cli/stdio/stdout.md), or any other `context function` provided by the flow runtime 
being used (e.g. `flowr`).

The only interaction with the execution environment in this example is the use of `stdout` to print the numbers
in the series to the Terminal.

### Functions
Functions are stateless, and pure, and just take a set of inputs (one on each of its inputs) and produce an output.

When all the inputs of a function have a value, then the function can run and produce an output, or not
produce outputs, as in the case of the impure `stdout` function.

This flow uses two functions (shown as orange ovals):
- `stdout` from the `context functions` as described above
  - `stdout` only has one, unnamed, default input and no outputs. It will print the value on STDOUT of the process
  running the flow runner (`flowr`) that is executing the flow.
- the `add` function from the flow standard library `flowstdlib` to add two integers together.
  - `add` has two inputs "i1" and "i2" and produces the sum of them on the only, unnamed, "default" output.

### Connections
Connections (the solid lines) take the output of a function when it has ran, and send it to the input of connected 
functions. They can optionally have a name.

When a functions has ran, the input values used are made available again at the output. 

In this case the following three connections exist:
- "i2" input value is connected back to the "i1" input.
- the output of "add" (the sum of "i1" and "i2") is connected back to the "i2" inputs. This connection has optionally 
  been called "sum"
- the output of "add" (the sum of "i1" and "i2") is connected to the default input of "Stdout". This connection has 
  optionally been called "sum"
- 
### Initializations
Inputs of processes (flows or functions) can be initialized with a value "Once" (at startup) or "Always" (each time 
it ran) using input initializers (dotted lines)

In this example two input initializers are used to setup the series calculation
- "Once" initializer with value "1" in the "i2" input of "add"
- "Once" initializer with value "0" in the "i1" input of "add"