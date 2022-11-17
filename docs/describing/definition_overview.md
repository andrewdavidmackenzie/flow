## Defining Flows

All flows start at the `root`, that defines the interactions of the flow with the environment or `context` around it,
plus including sub-flows and functions, and connections between them.

Any flow can contain zero or more nested flows or functions directly via [Process References](process_references.md).

A flow can use functions provided by the "flow runner" that is executing the flow, for the purpose of interacting
with the surrounding environment (such as file IO, standard IO, etc). These are the `context functions`.

No connections enter or leave the top-level root flow, unless via a `context function` interacting with the environment.

Data flow internally between sub-flows and functions (collectively known as "processes"), as defined by the connections.

So, valid entries in a flow definition include:
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
The complete list of features that can be used in the description of flows:

* Root Flow
  * Cannot have any outputs from this flow as there is no parent level
* All Flows (Root and Children)
  * Can contain elements directly inside the root flow description
  * Child flow inclusion from description in its own flow file in current project or different project
  * Named outputs from child flow, referenced by parent flow for connections
* Functions
  * With just inputs
  * With just outputs
  * With inputs and outputs
  * Use of aliases to refer to functions with different names inside a flow
* Use of Library Functions
* Providing a Custom function (in rust) with a flow
* Destructuring of output value into multiple named outputs
* Connections between outputs and inputs
  * Connections between inputs and outputs of functions, values, current flow and sub-flows
  * Multiple connections to a single inputs (first arrived wins)
  * Multiple connections from a single output (value is copied to all destinations)
  * Connections to values don't require input name as only have one input
  * Connections from values don't require output name as only have one output
  * Functions with single input can have a connection to it without naming the input
  * Functions with single output can have a connection from it without naming the output
* Libraries of functions can be built and described, like flowstdlib, and referenced in flows
* Run-time functions for
  * Retrieving arguments from the flow's invocation
  * STDIN/STDOUT/STDERR
  * Retrieving the value of Environment Variables
