# flowdb - Flow Debugger

`flowdb` is a standalone debugger client for [flow](https://github.com/andrewdavidmackenzie/flow)
programs. It connects to a flow runner's debug server and provides an interactive REPL for
setting breakpoints, stepping through execution, and inspecting runtime state.

## Usage

Start a flow with debugging enabled in one terminal:

```bash
flowrcli --debugger --native my-flow/manifest.json
# Debug server listening on port 12345. Connect with: flowdb --address localhost:12345
```

Connect the debugger from another terminal:

```bash
flowdb --address localhost:12345
```

Or let mDNS discover the debug server automatically:

```bash
flowdb
```

## Debugger Commands

| Command | Description |
|---------|-------------|
| `h` / `help` | Show help |
| `s` / `step` [n] | Step over the next n jobs (default 1) |
| `c` / `continue` | Continue execution |
| `b` / `breakpoint` {spec} | Set a breakpoint |
| `d` / `delete` {spec} | Delete a breakpoint |
| `l` / `list` | List all breakpoints |
| `f` / `functions` | List all functions |
| `i` / `inspect` [spec] | Inspect state |
| `v` / `validate` | Validate flow state |
| `m` / `modify` name=value | Modify a runtime variable |
| `r` / `reset` | Reset and re-run the flow |
| `e` / `exit` | Exit the debugger |

## Breakpoint Specs

- `42` - break on function #42
- `3/result` - break on output route `/result` of function #3
- `5:0` - break on input #0 of function #5
- `1->2` - break on block between functions #1 and #2
- `*` - all breakpoints (for delete)

## Library

The `flowdblib` library provides shared types for building debugger frontends:

- `DebugClient` - REPL debug client
- `DebugHandler` - implements `DebuggerHandler` trait over ZMQ
- `DebugServerMessage` - protocol messages from server to client
- `ClientConnection` / `CoordinatorConnection` - ZMQ connection types
