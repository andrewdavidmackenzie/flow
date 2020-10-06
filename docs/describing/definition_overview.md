## Defining Flows

A flow can define the entities external to the program with which it interacts and should be provided by the run-time, 
or bundled via a library. i.e. they are not included in the flow, but the flow interacts with them as part of it's
run-time "context".

By definition, nothing enters or leaves the top-level flow ("context").
At most things flow between the contained flow and entities referenced, as defined by the connections.

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

### Context
All flows start with a flow called the `context`. This is the flow that defines the interactions
of the overall flow hierarchy with the environment or "context" around the contained flows.

Any flow can contain any number of nested flows via [Process References](process_references.md).