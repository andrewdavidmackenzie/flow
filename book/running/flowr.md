## Running flows with `flowrcli`

In order to run a flow, it must first be compiled. Then a "flow runner" such as `flowrcli` can be used to run the 
compiled flow manifest).

Flow runners in general and `flowrcli` run the compiled flow manifest (by default named `manifest.json`).

In order to compile a flow definition down to a flow manifest that can be run, you use `flowc` as usual, with the
addition of the `-c, --compile` option. This compiles the flow but does not invoke `flowrcli` to run it.

Then `flowrcli` as described below can be used to run the compiled flow.

This section describes command line arguments that can be supplied to `flowrcli` and what they are useful for.

### Getting help for `flowrcli`
Use `-h, --help` (e.g. `flowc -h` or `cargo run -p flowc -- -h`) to print out help for the usage of `flowc`. 

This will print something like this:
```shell script 
Usage: flowr [OPTIONS] [flow-manifest] [flow_args]...

Arguments:
  [flow-manifest]  the file path of the 'flow' manifest file
  [flow_args]...   A list of arguments to pass to the flow.

Options:
  -d, --debugger                     Enable the debugger when running a flow
  -m, --metrics                      Calculate metrics during flow execution and print them out when done
  -n, --native                       Link with native (not WASM) version of flowstdlib
  -s, --server                       Launch flowr with a coordinator only, no client
  -c, --client <port>                Launch flowr with a client only, no coordinator, to connect to a flowr coordinator
  -C, --context                      Execute only 'context' (not general) jobs in the coordinator
  -j, --jobs <MAX_JOBS>              Set maximum number of jobs that can be running in parallel)
  -L, --libdir <LIB_DIR|BASE_URL>    Add a directory or base Url to the Library Search path
  -t, --threads <THREADS>            Set number of threads to use to execute jobs (min: 1, default: cores available)
  -v, --verbosity <VERBOSITY_LEVEL>  Set verbosity level for output (trace, debug, info, warn, default: error)
  -h, --help                         Print help information
  -V, --version                      Print version information
```

Similarly to [flowc](flowc.md), in order to locate libraries used in flow execution, `flowrcli` needs to know where to 
locate them. As for flowc, you can rely on the default (`$HOME/.flow/lib`), modified using the `$FLOW_LIB_PATH` 
environment variable, or using one or more instance of the `-L, --libdir <LIB_DIR|BASE_URL>` option.

### `flow-manifest`
After the Options you can supply an optional field for where to load the root flow from. This can be a relative or 
absolute path when no Url scheme is used, an absolute path if the `file://` scheme is used or a web resources if
either the `http` or `https` scheme is used.
* If no argument is supplied, it assumes the current directory as the argument, and continues as below
* If it's a directory then it attempts to load "root.toml" from within the directory
* If it's a file then it attempts to load the root flow from that file

### `flow_args`
Any arguments after `flow-manifest` are assumed to be arguments for the flow itself. When it starts executing it can
retrieve the value of these parameters using `context functions`.

### Example Invocations
For each of these examples, there is first a `flowc` line showing how the flow can be compiled. This will leave
a compiled `manifest.json` flow manifest alongside the flow's root definition file. That manifest is then run using
`flowrcli`

- `flowc -C flowr/src/bin/flowrcli -c flowr/examples/fibonacci` - compile the fibonacci example only
- `flowrcli flowr/examples/fibonacci` - run the pre-compiled fibonacci example flow manifest
    - You should get a fibonacci series output to the terminal,
- `flowc -C flowr/src/bin/flowrcli -c flowr/examples/sequence` - compile the flow only, do not run it
- `flowrcli flowr/examples/sequence 10` - run the compiled flow, a short sequence of numbers (2, 5, 8) and a string 
will be printed. The "10" represents the maximum of the sequence.
- `flowrcli flowr/examples/sequence/manifest.json 10` - run the compiled flow, specifying the full path to the 
manifest.json file

### `flowrgui`
Similar to `flowrcli` that interacts with the terminal and the file system for IO, `flowrgui` is another runner
for flows, but with a Graphical User Interface (GUI). It displays STDIO and STDERR on the UI, shows images written
to visually and tracks writes to files during execution.

Most (but not all) of the same command line options as `flowrcli` are supported, and help can be see using:

`flowrgui --help`
