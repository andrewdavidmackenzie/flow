## Debugging your first flow

### Command line options to `flowc`
When running `flowc` using `cargo run -p flowc` you should add `--` to mark the end of the options passed to cargo, 
and the start of the options passed to `flowc`

You can see what they are using `--help` producing output similar to this:

```bash
cargo run -p flowc -- --help                         
    Finished dev [unoptimized + debuginfo] target(s) in 0.12s
     Running 'target/debug/flowc --help'
flowc 0.8.8

USAGE:
    flowc [FLAGS] [OPTIONS] [--] [ARGS]

FLAGS:
    -d, --dump        Dump the flow to .dump files after loading it
    -z, --graphs      Create .dot files for graph generation
    -h, --help        Prints help information
    -l, --lib         Compile a flow library
    -p, --provided    Provided function implementations should NOT be compiled from source
    -s, --skip        Skip execution of flow
    -g, --symbols     Generate debug symbols (like process names and full routes)
    -V, --version     Prints version information

OPTIONS:
    -L, --libdir <LIB_DIR|BASE_URL>...    Add a directory or base Url to the Library Search path
    -o, --output <OUTPUT_DIR>             Specify the output directory for generated manifest
    -i, --stdin <STDIN_FILENAME>          Read STDIN from the named file
    -v, --verbosity <VERBOSITY_LEVEL>     Set verbosity level for output (trace, debug, info, warn, error (default))

ARGS:
    <FLOW>            the name of the 'flow' definition file to compile
    <flow_args>...    Arguments that will get passed onto the flow if it is executed

```

### Command line options to `flowr`
By default `flowc` uses `flowr` to run the flow once it has compiled it. Also it defaults to passing the `-n/--native`
flag to `flowr` so that flows are executed using the native implementations of library functions.

In order to pass command line options on to `flowr` you separate them from the options to `flowc` after another `--` separator.

`flowr` accepts the same `-v/--verbosity` verbosity options as `flowc`.

### Getting debug output
If you want to follow what the run-time is doing in more detail, you can increase the verbosity level (default level is ERROR)
using the `-v/--verbosity` option.

So, if you want to walk through each and every step of the flow's execution, similar to the previous [step by step section](step-by-step.md) 
then you can do so by using `-v debug` and piping the output to `more` (as there is a lot of output!):
 
 `cargo run -p flowc -- samples/fibonacci -- -v debug| more`

which should produce output similar to this:

```bash
INFO    - 'flowr' version 0.8.8
INFO    - 'flowrlib' version 0.8.8
DEBUG   - Loading library 'flowruntime' from 'native'
INFO    - Library 'flowruntime' loaded.
DEBUG   - Loading library 'flowstdlib' from 'native'
INFO    - Library 'flowstdlib' loaded.
INFO    - Starting 4 executor threads
DEBUG   - Loading flow manifest from 'file:///Users/andrew/workspace/flow/samples/fibonacci/manifest.json'
DEBUG   - Loading libraries used by the flow
DEBUG   - Resolving implementations
DEBUG   - Setup 'FLOW_ARGS' with values = '["my-first-flow"]'
INFO    - Maximum jobs dispatched in parallel limited to 8
DEBUG   - Resetting stats and initializing all functions
DEBUG   - Init: Initializing Function #0 '' in Flow #0
DEBUG   -               Input initialized with 'Number(0)'
DEBUG   -               Input initialized with 'Number(1)'
DEBUG   - Init: Initializing Function #1 '' in Flow #0
DEBUG   - Init: Creating any initial block entries that are needed
DEBUG   - Init: Readying initial functions: inputs full and not blocked on output
DEBUG   -               Function #0 not blocked on output, so added to 'Ready' list
DEBUG   - ===========================    Starting flow execution =============================
DEBUG   - Job #0:-------Creating for Function #0 '' ---------------------------
DEBUG   - Job #0:       Inputs: [[Number(0)], [Number(1)]]
DEBUG   - Job #0:       Sent for execution
DEBUG   - Job #0:       Outputs '{"i1":0,"i2":1,"sum":1}'
DEBUG   -               Function #0 sending '1' via output route '/sum' to Self:1
DEBUG   -               Function #0 sending '1' via output route '/sum' to Function #1:0
DEBUG   -               Function #1 not blocked on output, so added to 'Ready' list
DEBUG   -               Function #0 sending '1' via output route '/i2' to Self:0
DEBUG   -               Function #0, inputs full, but blocked on output. Added to blocked list
DEBUG   - Job #1:-------Creating for Function #1 '' ---------------------------
DEBUG   - Job #1:       Inputs: [[Number(1)]]
DEBUG   -                               Function #0 removed from 'blocked' list
DEBUG   -                               Function #0 has inputs ready, so added to 'ready' list
DEBUG   - Job #1:       Sent for execution
DEBUG   - Job #2:-------Creating for Function #0 '' ---------------------------
DEBUG   - Job #2:       Inputs: [[Number(1)], [Number(1)]]
1
DEBUG   - Job #2:       Sent for execution
DEBUG   - Job #2:       Outputs '{"i1":1,"i2":1,"sum":2}'
DEBUG   -               Function #0 sending '2' via output route '/sum' to Self:1
DEBUG   -               Function #0 sending '2' via output route '/sum' to Function #1:0
DEBUG   -               Function #1 not blocked on output, so added to 'Ready' list
DEBUG   -               Function #0 sending '1' via output route '/i2' to Self:0
DEBUG   -               Function #0, inputs full, but blocked on output. Added to blocked list
```