# Flow Runtime TLA+ Specification

Formal specification of the flow runtime execution semantics, used to
verify correctness of `flowr/src/lib/run_state.rs`.

## Process Semantics

A **process** is the fundamental unit of computation. It has inputs that
receive values, an execution that consumes those values and may produce
output, and connections that route output to other processes' inputs.

The semantics of a process — how it receives values, when it executes,
and how it sends output — are the same regardless of whether the process
is implemented as a function or a flow. From the perspective of a parent
flow containing a process, the guarantees are identical:

- A process executes when all its inputs have values
- Execution consumes one value from each input
- Execution may produce an output value
- Output is sent to connected processes' inputs
- A process may indicate it should not run again (completion)

These semantics hold regardless of whether the runtime executes jobs
in parallel or sequentially. Running with `--jobs 1` produces the same
results as parallel execution — parallelism is an implementation
optimization, not a semantic change.

## Functions vs Flows — Implementation Detail

The runtime implements two kinds of processes:

- **Function**: executes a code implementation. The runtime may create
  multiple jobs for the same function in parallel when multiple input
  sets are available. This is an optimization — the function semantics
  are the same whether jobs run in parallel or sequentially.

- **Flow**: contains child processes (functions or other flows). A flow's
  execution is the collective execution of its children. When a flow
  goes idle (all children complete their current work), internal
  feedback values are cleared and flow initializers re-fire to prepare
  for the next iteration.

This distinction is an implementation detail of how process execution
is realized. The formal specification defines process semantics at the
abstract level, then shows how the runtime implements those semantics
for each process type.

## How TLA+ and TLC Work

### What is TLA+?

TLA+ is a formal specification language for describing systems as state
machines. You define:

- **Variables**: the system state (input queues, busy counts, etc.)
- **Init**: the initial state
- **Actions**: how the state can change (create job, retire job, etc.)
- **Invariants**: properties that must hold in EVERY reachable state

### What does TLC do?

TLC is the model checker. Given a spec, it:

1. Computes the initial state
2. Tries every possible action from that state
3. For each resulting state, tries every possible action again
4. Continues until all reachable states have been explored
5. At every state, checks all invariants

If any invariant is violated, TLC stops and shows you the exact sequence
of steps that leads to the violation — a concrete counterexample you can
use to reproduce the bug.

For example, the sub-flow gating bug we fixed in PR #2865 would show up
as a violation of the `AncestorConsistency` invariant: TLC would find a
state where a function has a job but its parent flow is not marked busy.

### The spec vs the topology

The `.tla` file has two parts:

1. **Generic logic** — the actions (CreateJob, Dispatch, Retire, etc.)
   and invariants. These define *how* the runtime works, independent of
   any specific flow graph. This is what we're verifying.

2. **Topology definition** — which processes exist, their inputs,
   connections, parent flows, and initializers. This defines a specific
   flow graph to check.

The generic logic lives in `FlowRuntimeBase.tla`. Scenario files
(like `TwoFuncsOneFlow.tla`) INSTANCE it with a specific topology:

```tla
FR == INSTANCE FlowRuntimeBase WITH
    Procs <- {1, 2},
    Flows <- {10},
    InputsOf <- 1 :> {0, 1} @@ 2 :> {0},
    Parent <- 1 :> 10 @@ 2 :> 10 @@ 10 :> NoParent,
    ...
```

To check a different topology, create a new scenario file or use
`flowc --tla` to auto-generate one from a flow definition.

### The .cfg file

The `.cfg` file tells TLC what to check:

```
SPECIFICATION Spec
INVARIANTS
    TypeOK
    CompletedNeverRuns
    InternalCountBound
    AncestorConsistency
```

It lists the specification and which invariants to verify.

### Adding more scenarios

Create new scenario files for specific topologies:
- `NestedFlows.tla` — testing sub-flow semantics
- `FeedbackLoop.tla` — topology with loopback connections

Or use `flowc --tla` to generate scenarios from any compiled flow.

### What TLC output looks like

**Success** (all invariants hold):
```
Model checking completed. No error has been found.
10 states generated, 10 distinct states found, 0 states left on queue.
```

**Failure** (invariant violated):
```
Error: Invariant AncestorConsistency is violated.
The behavior up to this point is:
State 1: <Initial predicate>
  /\ busyCount = << >>
  /\ inputQ = ...
State 2: <CreateJob>
  /\ busyCount = (1 :> 1)    \* BUG: parent flow 10 not marked busy!
  ...
```

The trace shows you exactly which sequence of actions leads to the bug.

## Files

- `FlowRuntimeBase.tla` — generic runtime semantics (CONSTANTS for topology)
- `TwoFuncsOneFlow.tla` / `.cfg` — scenario: 2 processes, 1 flow (internal send)
- `InternalExternal.tla` / `.cfg` — scenario: mixed internal and external sends to same input
- `MixedQueue.tla` / `.cfg` — scenario: internal self-feedback and external send with bounded execution
- `ExternalGating.tla` / `.cfg` — scenario: external send gating across flow boundaries
- `README.md` — this file

## Installing TLA+ Tools

### macOS (Homebrew)
```bash
brew install --cask tla+-toolbox
```

### Running the Model Checker
```bash
java -XX:+UseParallelGC \
  -cp "/Applications/TLA+ Toolbox.app/Contents/Eclipse/tla2tools.jar" \
  tlc2.TLC -config specs/FlowRuntime.cfg specs/FlowRuntime.tla \
  -workers auto -metadir /tmp/tlc -deadlock
```

The `-deadlock` flag tells TLC not to report deadlock as an error (flows
naturally reach a terminal state where no actions are enabled).

TLC uses all available CPU cores (`-workers auto`) and prints progress
as it explores states.

## Phases

1. **Core state machine** — process states, input queues, job lifecycle ✅
2. **Input queue ordering** — internal vs external values ✅
3. **Flow hierarchy** — busy/idle detection, ancestor tracking ✅
4. **External send gating** — cross-flow sends deferred when busy ✅
5. **Initializer semantics** — Once/Always, function vs flow level
