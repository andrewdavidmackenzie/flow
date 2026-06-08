# Interactive Run Process in Debugger (#2021)

## Overview

Extend the debugger's `r` (run/reset) command to allow running a specific
function or sub-flow (process) interactively, with user-supplied inputs and
sandboxed execution via reset.

## Command Syntax

- `r` — run the root flow from init (unchanged behavior)
- `r <id>` — run a specific process by numeric ID
- `r <route>` — run a specific process by route (e.g., `r /flow/add`)
- `r <name>` — run a specific process by name
- `r <target> <arg1> <arg2> ...` — run with inline input values

### Target Resolution

`ProcessTarget` enum: `Id(usize)`, `Route(String)`, or `Name(String)`.

Resolution order: try parsing as usize (ID), then check if it starts with `/`
(route), otherwise treat as name. Names are matched against function names.
If no match: error. If multiple matches: list the matching processes with their
IDs and routes so the user can re-run with a more specific target.

### Precondition

The flow must be in init/reset state (`get_number_of_jobs_created() == 0`).
If not, return: `"Cannot run a process mid-execution. Reset first with 'r'."`

## Input Capture

When `r <target>` is issued, the debugger resolves the target process and
retrieves its input list (name, type, initializer).

### CLI (flowrdb)

Prompt for every input, one at a time. Pre-fill with defaults where available:

- If an inline arg was provided for this input (positional), show it as default
- Else if the input has an initializer (`Once`/`Always`), show that as default
- Else no default (user must provide a value)

Prompt format:
```
input_name (Type) [default_value]: _
input_name: _                          # generic, no default
```

Pressing Enter accepts the pre-filled default. Typing a new value overrides it.

### GUI (flowrgui)

Always open a pre-fill panel (slideout) showing all inputs:

- Each input shown as a label (`name (Type)`) with a text field
- Pre-fill with: inline arg if provided, otherwise initializer value if present,
  otherwise empty
- User can edit any field before clicking "Run"
- Closing the panel without clicking Run cancels the operation

## Type Coercion

### When input has a declared type

Attempt to coerce the user's string to the declared type. On failure:
`"Cannot coerce '<value>' to <Type> for input '<name>'"`

### Generic inputs (heuristic rules)

- `true`/`false` → Bool
- Parses as number → Number
- Quoted string (`"hello"`) → String (even if contents look numeric)
- `null` or `Null` → Null
- `[...]` → Array, element type inferred recursively from first element;
  `[]` for empty array
- `{...}` → Object (parsed as JSON)
- Anything else unquoted → String

## Execution

### Approach: Execute-in-init-state with reset

Requires the flow to be in init/reset state. After execution completes, the
flow is automatically reset back to init state.

### Steps

1. Resolve target to a process ID
2. Validate the process exists (function or sub-flow)
3. Gather inputs via CLI prompting or GUI panel
4. Apply type coercion; error on failure
5. **Function:** create a Job with coerced inputs, push to head of ready queue,
   execute one job, capture result
6. **Sub-flow:** *(deferred — not implemented in the initial version)* fill
   entry-point inputs of the sub-flow's internal functions, run the coordinator
   loop over the sub-flow's internal graph to completion (auto-run). Breakpoints
   inside the sub-flow are honored if set.
7. Capture outputs
8. Display outputs
9. Reset back to init state

## Output Display

### CLI

Print each output prefixed with its route:
```
/flow/add/output: 42
/flow/add/remainder: 2
```

If no output: `(no output)`

### GUI

A slideout panel appears showing the outputs. Each line: `route: value`.
Panel has a close/dismiss button. Disappears on close or next command.

## Error Handling

- Target not found: `"No process found matching '<target>'"`
- Too many inline args: `"Process has N inputs but M values were provided"`
- Type coercion failure: `"Cannot coerce '<value>' to <Type> for input '<name>'"`
- Flow not in init state: `"Cannot run a process mid-execution. Reset first with 'r'."`
- Execution error: display the Job result error, then reset

## DebugCommand Changes

`RunReset` changes from a unit variant to:

```rust
RunReset {
    target: Option<ProcessTarget>,  // None = root flow
    args: Vec<String>,              // inline input values
}
```

## Scope

- Both functions and sub-flows supported
- Both CLI (flowrdb) and GUI (flowrgui)
- Sandboxed execution via init-state requirement + auto-reset
