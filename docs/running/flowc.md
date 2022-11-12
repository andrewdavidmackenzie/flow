## `flowc` Command Line Arguments
`flowc` is the flow "compiler", although compiling a flow is very different to a procedural language compile.

What it and other components do is described in more detail later in the [Internals](../internals/overview.md) section.

This section describes command line arguments that can be supplied to `flowc` and what they are useful for.

### Getting help
Use `-h, --help` (e.g. `flowc -h` or `cargo run -p flowc -- -h`) to print out help for the usage of `flowc`. 

This will print something like this:
```shell script 
flowc 0.4.0

USAGE:
    flowc [FLAGS] [OPTIONS] [ARGS]

FLAGS:
    -d, --dump       Dump the flow to standard output after loading it
    -h, --help       Prints help information
    -s, --skip       Skip code generation and running
    -V, --version    Prints version information

OPTIONS:
    -v, --verbosity <VERBOSITY_LEVEL> Set verbosity level for output (trace, debug, info, warn, error (default))
    -o, --output <OUTPUT_DIR>         Output directory for generated code

ARGS:
    <FLOW>            the name of the 'flow' file
    <flow_args>...
```

Where the first line prints the binary name and the version number.

### Flags Described
* `-d, --dump` - Dumps a text representation of the the flow hierarchy to standard output after loading it
* `-s, --skip` - Skip the code generation and running of the generated flow
* `-V, --version`- Prints version information

### Options Described
* `-v, --verbosity <VERBOSITY_LEVEL>`- Set verbosity level for output (VERBOSITY_LEVEL_LEVEL can be `trace`, `debug`, `info`, `warn` or `error` (the default))
* `-o, --output <OUTPUT_DIR>`- Specify the output directory for generated code. By default this is in a "rust" 
subdirectory of the directory where the flow's root was loaded from

### Flow Directory or Filename
After Flags and Options you can supply an optional field for where to load the root flow from.
* By default this is the current directory.
* If it's a directory then it attempts to load "root.toml" from there
* If it's a file then it attempts to load "root.toml" from that file

It can also be a URL to a flow root specification somewhere on the web. Currently supports http and http.

### Arguments for the flow
If a flow directory or filename is supplied, then any files after that are assumed to be command line arguments
for the flow itself. When it starts executing it can retrieve the value of these parameters using functions
in the run-time.
