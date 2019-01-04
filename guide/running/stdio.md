### Standard Input
The flow runtime provides functions to read from STDIN. You can pipe input to the flow by piping
it to the `cargo run` or `flowc` command line used to execute the flow.

If not piped in, then the runtime will attempt to read STDIN, blocking that function until input 
(or EOF) is provided. If input is read then it will be passed on by that function at it's output.
The function will indicate to the runtime that it should be run again (to read more lines of STDIN)
and it will be re-added to the ready list and executed again later.

When EOF is detected, that function will indicate to the runtime that it does not want to be run again
and will not be added back to the ready list for re-execution.

### Standard Output & Standard Error
The flow runtime provides functions to send output to STDOUT/STDERR. This output is printed on 
stdout or stderr of the process that executed the `cargo run` or `flowc` command to execute the flow.