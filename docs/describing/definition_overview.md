## Guide to Defining Flows
In this guide to defining flows, we will describe the syntax of definitions files, but also the run-time semantics
of flows, functions, jobs, inputs etc in order to understand how a flow will run when defined.

A flow is a static hierarchical grouping of functions that produce and consume data, connected via 
connections  into a graph.

### Root Flow
All flows have a `root` flow definition file.

The root flow can reference functions provided by the "flow runner" application that will execute the flow, 
for the purpose of interacting with the surrounding environment (such as file IO, standard IO, etc). 
These are the `context functions`.

The root flow (as any sub-flow can) may include references to sub-flows and functions, joined by connections between 
their inputs and outputs, and so on down in a hierarchy.

The root flow cannot have any input or output. As such, all data flows start or end in the root flow. What you
might consider "outputs", such as printing to standard output, is done by describing a connection to a `context 
functions` that interacts with the environment.

### Flows in General
Any flow can contain references to functions it uses, plus zero or more references to nested flows via [Process 
References](process_references.md), and so on down.

Data flows internally between sub-flows and functions (collectively known as "processes"), as defined by the 
connections.

All computation is done by functions. A flow is just a hierarchical organization method that allows to group
and abstract groups of functions (and sub-flows) into higher level concepts. All data that flows originates in
a function and terminates in a function. 

flow and sub-flow nesting is just an organizational technique to facilitate encapsulation and re-use of functionality,
and does not affect program semantics.

Whether a certain process in a flow is implemented by one more complex function - or by a sub-flow combining multiple,
simpler, functions - should not affect the program semantics.

### Valid Elements of a flow definition
Valid entries in a flow definition include:
- `flow` - A String naming this flow (obligatory)
- `docs` - An optional name of an associated markdown file that documents the flow
- `version` - A SemVer compatible version number for this flow (Optional)
- `authors` - Array of Strings of names and emails of authors of the flow (Optional)
- `input`|`output` - 0 or more input/outputs of this flow made available to any parent including it (Note: 
  that the root flow may not contain any inputs or outputs). See [IOs](ios.md) for more details.
- `process` - 0 or more references to sub-processes to include under the current flow. A sub-process
can be another `flow` or a `function`. See [Process References](process_references.md) for more details.
- `connection` - 0 or more connections between io of sub-processes and/or `io` of this flow. See [Connections](connections.md)
for more details.

### Complete Feature List
The complete list of features that can be used in the description of flows is:

* Flow definitions
  * Named inputs and outputs (except root flow which has no parent)
  * References to sub-processes to use them in the flow via connections
    * Functions
       * Provided functions
       * Library functions
       * Context functions
    * Sub-flows
      * Arbitrarily from the file system or the web
      * From a library
    * Initializers for sub-process inputs and the flow outputs
      * `Once` initializers that initialize the input/output with a value just once at the start of flow execution
      * `Always` initializers that initialize the input/output every time it is emptied by the creation of a job that 
      takes the value.
    * Use of aliases to refer to sub-process with different names inside a flow, facilitating the use of the same
    function or flow multiple times for different purposes within the sub-flow
  * Connections between outputs and inputs within a flow
    * Connections can be formed between inputs to flow or outputs of one process (function or flow) and outputs
      of the flow or inputs of a process
    * Multiple connections from a source
    * Multiple connections to a destination
    * Connection to/from a default input/output by just referencing the process in the connection
    * Destructuring of output struct in a connection to just connect a sub-part o fit
    * Optional naming of a connection to facilitate debugging
* Function definitions
  * With just inputs
  * With just outputs
  * With inputs and outputs
  * default single input/output, named single input/output, named multiple inputs/outputs
  * author and versioning meta-data and references to the implementation
* Libraries of processes (functions and flows) can be built and described, and referenced in flows