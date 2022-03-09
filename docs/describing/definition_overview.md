## Defining Flows

A flow can define the entities external to the program with which it interacts. These are the "context functions"
provided by the run-time.

By definition, nothing enters or leaves the top-level root flow (named "root.toml" by default).
At most, things flow internally between contained sub-flows and functions (collectively known as "processes"), 
as defined by the connections.

A flow contain 0 or more sub-flows which are described in separate "flow" files.
The entities referenced in the top-level flow definition are connected to inputs and outputs of any sub-flow by 
connections.

So, valid entries in a flow definition include:
- `flow`        - a String naming this flow (obligatory)
- `version`     - a SemVer compatible version number for this flow (Optional)
- `authors`     - Array of Strings of names and emails of authors of the flow (Optional)
- `io`          - 0 or more input/outputs of this flow made available to any parent including it
- `value`       - 0 or more values contained in this flow
- `process`     - 0 or more references to sub-processes to include under the current flow. A sub-process
can be another `flow` or a `function`
- `connection`  - 0 or more connections between outputs and inputs of values or sub-processes and `io` of this flow 
(hence permitting connections to/from parent flows including this one)

### Root of Flow
All flows start with a flow called the `root`. This is the flow that defines the interactions
of the overall flow hierarchy with the environment or "context" around it.

Any flow can contain any number of nested flows via [Process References](process_references.md).