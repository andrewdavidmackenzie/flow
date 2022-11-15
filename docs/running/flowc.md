## `flowc` Command Line Arguments
`flowc` is the flow "compiler", although compiling a flow is very different to a procedural language compile.

What it and other components do is described in more detail later in the [Internals](../internals/overview.md) section.

This section describes command line arguments that can be supplied to `flowc` and what they are useful for.

### Getting help
Use `-h, --help` (e.g. `flowc -h` or `cargo run -p flowc -- -h`) to print out help for the usage of `flowc`. 

This will print something like this:
```shell script 
Usage: flowc [OPTIONS] [source_url] [flow_args]...

Arguments:
  [source_url]    path or url for the flow or library to compile
  [flow_args]...  List of arguments get passed to the flow when executed

Options:
  -d, --debug
          Generate symbols for debugging. If executing the flow, do so with the debugger
  -c, --compile
          Compile the flow and implementations, but do not execute
  -C, --context_root <CONTEXT_DIRECTORY>
          Set the directory to use as the root dir for context function definitions
  -l, --lib
          Compile a flow library
  -n, --native
          Compile only native (not wasm) implementations when compiling a library
  -L, --libdir <LIB_DIR|BASE_URL>
          Add a directory or base Url to the Library Search path
  -t, --tables
          Write flow and compiler tables to .dump and .dot files
  -g, --graphs
          Create .dot files for graphs then generate SVGs with 'dot' command (if available)
  -m, --metrics
          Show flow execution metrics when execution ends
  -w, --wasm
          Use wasm library implementations when executing flow
  -O, --optimize
          Optimize generated output (flows and wasm)
  -p, --provided
          Provided function implementations should NOT be compiled from source
  -o, --output <OUTPUT_DIR>
          Specify the output directory for generated manifest
  -v, --verbosity <VERBOSITY_LEVEL>
          Set verbosity level for output (trace, debug, info, warn, error (default))
  -i, --stdin <STDIN_FILENAME>
          Read STDIN from the named file
  -h, --help
          Print help information
  -V, --version
          Print version information
```

### Options
*  `-d, --debug` Generate symbols for debugging. If executing the flow, do so with the debugger
*  `-c, --compile` Compile the flow and implementations, but do not execute
*  `-C, --context_root <CONTEXT_DIRECTORY>` Set the directory to use as the root dir for context function definitions
*  `-l, --lib` Compile a flow library. The `source_url` supplied should be the root of the library to compile.
*  `-n, --native` Compile only native (not wasm) implementations when compiling a library
*  `-L, --libdir <LIB_DIR|BASE_URL>` Add a directory or base Url to the Library Search path
*  `-t, --tables` Write flow and compiler tables to .dump and .dot files
*  `-g, --graphs` Create .dot files for graphs then generate SVGs with 'dot' command (if available)
*  `-m, --metrics` Show flow execution metrics when execution ends
*  `-w, --wasm` Use wasm library implementations (not any statically linked native implementations) when executing flow
*  `-O, --optimize` Optimize generated output (flows and wasm)
*  `-p, --provided` Provided function implementations should NOT be compiled
*  `-o, --output <OUTPUT_DIR>` Specify the output directory for generated manifest
*  `-v, --verbosity <VERBOSITY_LEVEL>` Set verbosity level for output (trace, debug, info, warn, error (default))
*  `-i, --stdin <STDIN_FILENAME>` Read STDIN from the named file
*  `-h, --help` Print help information
*  `-V, --version` Print version information


### `source_url`
After the Options you can supply an optional field for where to load the root flow from. This can be a relative or 
absolute path when no Url scheme is used, an absolute path if the `file://` scheme is used or a web resources if
either the `http` or `https` scheme is used.
* If no argument is supplied, it assumes the current directory as the argument, and continues as below
* If it's a directory then it attempts to load "root.toml" from within the directory
* If it's a file then it attempts to load the root flow from that file

### `flow_args`
If a flow directory or filename is supplied, then any files after that are assumed to be command line arguments
for the flow itself. When it starts executing it can retrieve the value of these parameters using functions
in the run-time.
