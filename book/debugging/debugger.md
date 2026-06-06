### The Flow Debugger

The flow debugger allows you to interactively debug flow programs by setting breakpoints,
stepping through execution, and inspecting runtime state.

The debugger uses a two-process architecture: the flow runner (`flowrcli`) hosts the debug
server, and `flowrdb` is a standalone debug client that connects from a separate terminal.
This keeps the debugger's I/O separate from the flow's stdin/stdout.

#### Compiling with Debug Symbols

Flows compiled by `flowc` using the `-d` or `--debug` option will have extra human-readable
content included in the compiled manifest (names of processes, source locations, etc.)
and be more convenient to debug.

Note: `flowrcli` must be compiled with the `"debugger"` feature enabled (it is by default).

#### Starting a Debug Session from the Command Line

**Terminal 1** — Start the flow with debugging enabled:
```bash
flowrcli --debugger --native my-flow/manifest.json
# Debug server listening on port 12345. Connect with: flowrdb --address localhost:12345
```

**Terminal 2** — Connect the debugger:
```bash
flowrdb --address localhost:12345
```

Or let mDNS discover the debug server automatically:
```bash
flowrdb
```

The debugger will display a prompt where you can enter commands before execution begins.

#### Starting a Debug Session from flowrgui

You can also debug flows launched from the `flowrgui` GUI runner:

1. Open `flowrgui` and load a flow
2. Click the **Debug** button (instead of Run)
3. The status bar shows "Waiting for debugger to connect..." along with the
   `flowrdb` command to use
4. In a separate terminal, run the `flowrdb` command shown in the status bar
5. The flow will start executing once `flowrdb` connects
6. Flow output appears in flowrgui's tabs while debug commands are entered in flowrdb

#### Debugging Workflow

1. The debugger starts paused before flow execution
2. Use `inspect` to examine the initial state
3. Set breakpoints with `breakpoint` on specific functions, inputs, or outputs
4. Use `continue` to run until a breakpoint, or `step` to advance one job at a time
5. When a breakpoint triggers, examine state with `inspect` and `functions`
6. Use `continue` or `step` to resume
7. After the flow completes, you can `reset` to re-run or `exit` to quit

#### Debugger Commands

| Command | Short | Description |
|---------|-------|-------------|
| `help` | `h`, `?` | Display help on available commands |
| `step [n]` | `s` | Step over the next `n` jobs (default 1) then break |
| `continue` | `c` | Continue execution until next breakpoint or end |
| `breakpoint {spec}` | `b` | Set a breakpoint (see specs below) |
| `delete {spec}` | `d` | Delete a breakpoint matching the spec, or `*` for all |
| `list` | `l` | List all breakpoints currently set |
| `functions` | `f` | Show all functions in the flow |
| `inspect [spec]` | `i` | Inspect overall state, or a specific function/input/output |
| `validate` | `v` | Run checks to validate the current flow state |
| `modify name=value` | `m` | Modify a runtime variable (e.g. `max_parallel_jobs=2`) |
| `run` / `reset` | `r` | Reset the flow state and re-run from the beginning |
| `exit` / `quit` | `e`, `q` | Stop execution and exit the debugger |

#### Breakpoint Specs

Breakpoints can be set on different aspects of flow execution:

| Spec | Example | Description |
|------|---------|-------------|
| `function_id` | `b 3` | Break when function #3 is about to execute |
| `source_id/route` | `b 3/result` | Break when function #3 sends on output `/result` |
| `dest_id:input` | `b 5:0` | Break when input #0 of function #5 receives a value |
| `src->dest` | `b 1->2` | Break when a block is created between functions #1 and #2 |
| `*` | `d *` | Delete all breakpoints |

#### Inspect Specs

The `inspect` command accepts the following spec formats to examine specific parts of the flow:

| Command | Description |
|---------|-------------|
| `i` | Show overall flow state with all functions |
| `i 3` | Show state of function #3 |
| `i 5:0` | Show state of input #0 on function #5 |
| `i 3/result` | Show output connections from function #3's `/result` route |
| `i 1->2` | Show blocks between functions #1 and #2 |
