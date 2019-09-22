## Debugging your first flow
When the flow is compiled by flowc, the current implementation is to generate a rust project that is linked with the 
runtime and together it is built and run, with the runtime library executing the flow according to the generated 
tables of functions.

NOTE: in the future this implementation will change for being able to generate the functions table in a data file that is loaded and
run by a single runtime, removing the need to compile and build each time.

### Running the generated project
You can run this generated project for the flow directly from the `rust` subdirectory of the sample.
So, from the project root:
* `cd samples/first/rust`
* `cargo run`

### Command line options to generated project
The generated project uses `clap` to parse command line options, and you can see what they are using `--help`.

When running the project via `cargo run` you should add `--` to mark the end of the options passed to cargo, 
and the start of the options passed to the executable run by cargo.

```shell script
> cargo run -- --help
    Finished dev [unoptimized + debuginfo] target(s) in 0.13s
     Running `target/debug/root --help`
```
flowrlib

USAGE:
    root [OPTIONS] [flow_args]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -v, --verbosity <VERBOSITY_LEVEL> Set verbosity level for output (trace, debug, info, warn, error (default))

ARGS:
    <flow_args>...
`

### Getting debug output
If you want to follow what the runtime is doing in more detail, you can increase the verbosity level (default level is ERROR)
using the -v/--verbosity option.

So, if you want to walk through each and every step of the flow's execution, similar to the previous [step by step section](step-by-step.md) 
then you can do so by using `-v DEBUG` and piping the output to `more` (as there is a lot of output!) ```cargo run -- -v DEBUG | more```

which should produce output similar to this:

`
INFO    - 'flowrlib' version '0.5.0'
DEBUG   - Panic hook set to catch panics in functions
DEBUG   - Initializing all functions
DEBUG   -       Initializing function #0 'HEAD-1'
DEBUG   -               Function initialized by writing 'Number(1)' to input
DEBUG   -                       Function #0 inputs are ready
DEBUG   -                       Function #0 not blocked on output, so added to end of 'Ready' list
DEBUG   -       Initializing function #1 'HEAD'
DEBUG   -               Value initialized by writing 'Number(1)' to input
DEBUG   -                       Function #1 inputs are ready
DEBUG   -                       Function #1 not blocked on output, so added to end of 'Ready' list
DEBUG   -       Initializing function #2 'sum'
DEBUG   -       Initializing function #3 'print'
DEBUG   - Starting execution loop
DEBUG   - -----------------------------------------------------------------
DEBUG   - Dispatch count: 0
DEBUG   -        Can Run: {1, 0}
DEBUG   -       Blocking: []
DEBUG   -       Ready: [0, 1]
DEBUG   - -------------------------------------
DEBUG   - Function #0 'HEAD-1' dispatched
DEBUG   -       Function #0 consumed its inputs, removing from the 'Can Run' list
DEBUG   -       Function #0 'HEAD-1' running with inputs: [[Number(1)]]
DEBUG   -               Function #0 'HEAD-1' sending output '1' to Function #2 'sum' input #0
DEBUG   -                       Function #0 is now blocked on output by Function #2
DEBUG   -               Function #0 'HEAD-1' sending output '1' to Function #3 'print' input #0
DEBUG   -                       Function #0 is now blocked on output by Function #3
DEBUG   -                       Function #3 inputs are ready
DEBUG   -                       Function #3 not blocked on output, so added to end of 'Ready' list
DEBUG   -       Function #0 'HEAD-1' completed
DEBUG   - Dispatch count: 1
DEBUG   -        Can Run: {1, 3}
DEBUG   -       Blocking: [(2, 0), (3, 0)]
DEBUG   -       Ready: [1, 3]
DEBUG   - -------------------------------------
DEBUG   - Function #1 'HEAD' dispatched
DEBUG   -       Function #1 consumed its inputs, removing from the 'Can Run' list
DEBUG   -       Function #1 'HEAD' running with inputs: [[Number(1)]]
DEBUG   -               Function #1 'HEAD' sending output '1' to Function #0 'HEAD-1' input #0
DEBUG   -                       Function #1 is now blocked on output by Function #0
DEBUG   -                       Function #0 inputs are ready
DEBUG   -               Function #1 'HEAD' sending output '1' to Function #2 'sum' input #1
DEBUG   -                       Function #1 is now blocked on output by Function #2
DEBUG   -                       Function #2 inputs are ready
DEBUG   -                       Function #2 not blocked on output, so added to end of 'Ready' list
DEBUG   -       Function #1 'HEAD' completed
DEBUG   - Dispatch count: 2
DEBUG   -        Can Run: {3, 2, 0}
DEBUG   -       Blocking: [(2, 0), (3, 0), (0, 1), (2, 1)]
DEBUG   -       Ready: [3, 2]
`
