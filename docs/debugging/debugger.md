### The Flow Debugger

NOTE: To be able to use the flow debugger that is part of `flowr`, `flowr` must be compiled with
the `"debugger"` feature enabled. If not, the debugger code is not included in `flowr`.

#### Compiling with Debug Symbols
The debugger can be used to debug any flow, but flows compiled by `flowc` using the `-g` or `--symbols`
option will have extra human readable content included in the compiled manifest (names of processes
etc) and be more convenient to debug.

#### Running the flow with the debugger
To start debugging a flow, run it using `flowr` as normal, but using the `-d` or `--debugger`
options.

The compiled flow manifest will be loaded by `flowr` as usual, functions initialized and a command prompt
for the debugger will be shown.

You can use the `'h'` or `'help'` command at the prompt to to get help on debugger commands.

If you want to inspect the state of the flow at a particular point to debug a problem or understand 
its execution then you will probably want to either set some breakpoints initially before running the 
flow, or to step through the flow's execution one function invocation at a time.

Those can be done using the `Break` command to set breakpoints, the `List` command to list breakpoints set,
the `Run` command to start flow execution, the `Continue` command to continue execution after a breakpoint triggers,
and the `Step` command to step forward one function invocation.

#### Debugger Commands
* Break: Set a breakpoint on a function (by id), an output or an input using spec:
** function_id
** source_id/output_route ('source_id/' for default output route)
** destination_id:input_number
** blocked_process_id->blocking_process_id
  
* Continue: Continue execution until next breakpoint or end of execution

* Delete a breakpoint: Delete the breakpoint matching {spec} or all breakpoints with '*'

* Exit: Stop flow execution and exit debugger

* Help: Display this help message

* List breakpoints: List all breakpoints

* Print: Print the overall state, or state of process number 'n'

* Quit: Stop flow execution and exit debugger (same as Exit)

* Run: Run the flow or if running already then reset the state to initial state

* Step: Step over the next 'n' jobs (default = 1) then break

* Validate: Run a series of defined checks to validate the status of flow
