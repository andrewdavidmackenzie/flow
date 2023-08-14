### Standard Input
`context` provides functions to read from STDIN. You can pipe input to the flow by piping
it to the `cargo run -p flowc` or `flowc` command line used to execute the flow.

If not piped in, then the `stdin` function will attempt to read STDIN, blocking that function until input 
(or EOF) is provided. If input is read then it will be passed on by that function at its output.
The function will indicate to the run-time that it should be run again (to read more lines of STDIN)
and it will be re-added to the ready list and executed again later.

When EOF is detected, that function will indicate to the run-time that it does not want to be run again
and will not be added back to the ready list for re-execution.

### Standard Output & Standard Error
`context` provides functions to send output to STDOUT/STDERR. This output is printed on 
stdout or stderr of the process that executed the `cargo run -p flowc` or `flowc` command to execute the flow.

### Writing to Files
`context` supplies the `file_write`function (`context://file/file_write`) that allows flows to write
to files hosted by the file system where the flow runner is running.

Here is an example of a flow that writes the ASCII string "hello" to a file called "pipe":
```
flow = "test"

[[process]]
source = "context://file/file_write"
input.bytes = { once = [104, 101, 108, 108, 111] }
input.filename = { once = "pipe" }
```

You can run that flow from the command line using:`flowc -C flowr/src/bin/flowrcli root.toml`

and see that it has worked using: `cat pipe`

which will show the text `hello`

Now clean-up: `rm pipe`

### Named Pipes
On most *nix systems (including macos) there exists what are called "named pipes",
which allow interprocess communication via something that looks to them like files.

An example of how to use that, using the above flow is:
  * Terminal Window 1
    * `mkfifo pipe`
    * `cat pipe`
    * (process should block reading from that file and not display anything)
  * Terminal Window 2
    * Run the flow as before using `flowc -C flowr/src/bin/flowrcli root.toml`
    * The process blocked above in Terminal Window 1 will unblock and display `hello`
    * The flow will run to completion in Terminal Window 2

You can also run the flow first, it will block writing to the pipe, and then read from the pipe 
using `cat pipe`. Both processes will run to completion and `hello` will be displayed.