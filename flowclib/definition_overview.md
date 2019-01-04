## Flow Definition

A flow can define the entities external to the program with which it interacts and should be provided by the run-time, 
or bundled via a library. i.e. they are not included in the flow, but the flow interacts with them as part of it's
run-time "context".

By definition, nothing enters or leaves the top-level flow ("context").
At most things flow between the contained flow and entities referenced, as defined by the connections.

A flow contain 0 or more sub-flows which are described in separate "flow" files.
The entities referenced in the top-level flow definition are connected to inputs and outputs of any sub-flow by 
connections.

So, valid entries in a flow definition include:
- name       - String naming this flow (obligatory)
- io         - 0 or more input/outputs of this flow made available to any parent using it
- flow       - 0 or more references to flows to include under the current flow
- connection - 0 or more connections between entities, sub-flows and ios (e.g. to parent)
- function   - 0 or more functions referenced in this flow.
- value      - 0 or more values contained in this flow

### Context
All flows start with a root flow call the `context`. This is the flow that defines the interactions
of the overall flow hierarchy with the environment or "context" around the contained flows.

Any flow can contain any number of nexted flows via [Flow References](flow_reference.md).