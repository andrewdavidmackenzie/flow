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
  that the root flow may not contain any inputs or outputs)
- `process` - 0 or more references to sub-processes to include under the current flow. A sub-process
can be another `flow` or a `function`, but here they are referenced in the same way
- `connection` - 0 or more connections between io of sub-processes and/or `io` of this flow