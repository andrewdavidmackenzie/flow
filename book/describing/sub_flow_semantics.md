## Sub-flow Execution Semantics

Both functions and sub-flows are referenced as processes in a flow definition. At runtime,
a sub-flow behaves like a function: it receives input values, processes them, produces
output values, and completes — leaving no internal state from that execution.

### Run to Completion

When a sub-flow receives values on its inputs, it executes its internal functions until
no more work can be done. At that point:

1. All internally generated values are cleared
2. Flow initializers (if any) are re-applied
3. The sub-flow is ready to accept new input values for the next invocation

This means a sub-flow executes as a self-contained unit. Each invocation starts with
a clean internal state, processes the input values, and produces outputs. Between
invocations, no internal state carries over — the sub-flow behaves like a stateless
function from the perspective of the parent flow.

### Internal vs External Values

Values within a sub-flow are classified as either **internal** or **external**:

- **Internal values** are produced by connections between functions *within* the same
  flow definition. These include loopback connections (a function sending to itself)
  and pipeline connections between sibling functions.

- **External values** are produced by connections that cross flow boundaries — values
  entering the sub-flow from the parent flow, or values arriving from a sibling
  sub-flow via the parent.

When a sub-flow completes execution, all internal values are cleared. External values
that arrived while the sub-flow was running are preserved and will be consumed in the
next invocation.

### Implications for Flow Design

When designing flows with sub-flows, keep these rules in mind:

#### Intermediate values in pipelines

If a sub-flow contains a multi-stage pipeline where intermediate results are produced
by one function and consumed by another, those connections are internal. If the pipeline
cannot complete in a single pass (because some inputs arrive later), intermediate values
may be cleared prematurely.

**Solution**: Route intermediate values through the parent flow. Instead of connecting
function A directly to function B inside the sub-flow, have A's output leave the
sub-flow, and have the parent flow connect it back to B's input. This makes the
connection external, so the value persists across idle transitions.

The router example demonstrates this pattern — the `path_tracker` sub-flow routes
`forward_sum` and `cross_distance` outputs through the parent flow before they reach
`compare` and `cross_sum`.

#### Loopback values and sequences

The `sequence` library flow uses internal loopback connections to iterate. When the
sequence completes, the stale loopback values (step, limit) are automatically cleared.
This ensures a clean start for the next invocation with fresh external values.

If you build a flow that loops internally (e.g., using `compare_switch` with feedback),
make sure the loop terminates cleanly — all internal loopback values should be consumed
or become irrelevant when the loop ends. Avoid producing internal values that feed
downstream functions after the loop's termination condition is met.

The runtime ensures that when a function with a loopback has a running job, external
values on that input are not consumed until the loopback value arrives. This prevents
values from different invocations being paired incorrectly when a sub-flow receives
multiple sets of inputs concurrently.

#### Flow initializers are external

Input initializers set at the parent flow level (e.g., `input.start = {always = 0}`)
are treated as external values. They are re-applied when the sub-flow goes idle,
providing fresh starting values for the next invocation. Function-level initializers
defined within the sub-flow itself (e.g., `input.i2 = {once = 1}`) are also external.
