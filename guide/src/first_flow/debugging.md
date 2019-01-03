## Debugging your first flow
When the flow is compiled by flowc, the current implementation is to generate a rust project that is linked with the 
runtime and together it is built and run, with the runtime library executing the flow according to the generated 
tables of runnables.

NOTE: in the future this implementation will change to generate the runnables table in a data file that is loaded and
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

```
> cargo run -- --help
    Finished dev [unoptimized + debuginfo] target(s) in 0.13s
     Running `target/debug/root --help`
flowrlib

USAGE:
    root [OPTIONS] [flow_args]...

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -l, --log <LOG_LEVEL>    Set log level for output (trace, debug, info, warn, error (default))

ARGS:
    <flow_args>...
```

### Getting debug output
If you want to follow what the runtime is doing in more detail, you can increase the log level (default level is ERROR)
using the -l/--log option.

So, if you want to walk through each and every step of the flow's execution, similar to the previous [step by step section](step-by-step.md) 
then you can do so by using `-l DEBUG` and piping the output to `more` (as there is a lot of output!) ```cargo run -- -l DEBUG | more```

which should produce output similar to this:

```
INFO    - 'flowrlib' version '0.5.0'
DEBUG   - Panic hook set to catch panics in runnables
DEBUG   - Initializing all runnables
DEBUG   -       Initializing runnable #0 'HEAD-1'
DEBUG   -               Value initialized by writing 'Number(1)' to input
DEBUG   -                       Runnable #0 inputs are ready
DEBUG   -                       Runnable #0 not blocked on output, so added to end of 'Will Run' list
DEBUG   -       Initializing runnable #1 'HEAD'
DEBUG   -               Value initialized by writing 'Number(1)' to input
DEBUG   -                       Runnable #1 inputs are ready
DEBUG   -                       Runnable #1 not blocked on output, so added to end of 'Will Run' list
DEBUG   -       Initializing runnable #2 'sum'
DEBUG   -       Initializing runnable #3 'print'
DEBUG   - Starting execution loop
DEBUG   - -----------------------------------------------------------------
DEBUG   - Dispatch count: 0
DEBUG   -        Can Run: {1, 0}
DEBUG   -       Blocking: []
DEBUG   -       Will Run: [0, 1]
DEBUG   - -------------------------------------
DEBUG   - Runnable #0 'HEAD-1' dispatched
DEBUG   -       Runnable #0 consumed its inputs, removing from the 'Can Run' list
DEBUG   -       Runnable #0 'HEAD-1' running with inputs: [[Number(1)]]
DEBUG   -               Runnable #0 'HEAD-1' sending output '1' to Runnable #2 'sum' input #0
DEBUG   -                       Runnable #0 is now blocked on output by Runnable #2
DEBUG   -               Runnable #0 'HEAD-1' sending output '1' to Runnable #3 'print' input #0
DEBUG   -                       Runnable #0 is now blocked on output by Runnable #3
DEBUG   -                       Runnable #3 inputs are ready
DEBUG   -                       Runnable #3 not blocked on output, so added to end of 'Will Run' list
DEBUG   -       Runnable #0 'HEAD-1' completed
DEBUG   - Dispatch count: 1
DEBUG   -        Can Run: {1, 3}
DEBUG   -       Blocking: [(2, 0), (3, 0)]
DEBUG   -       Will Run: [1, 3]
DEBUG   - -------------------------------------
DEBUG   - Runnable #1 'HEAD' dispatched
DEBUG   -       Runnable #1 consumed its inputs, removing from the 'Can Run' list
DEBUG   -       Runnable #1 'HEAD' running with inputs: [[Number(1)]]
DEBUG   -               Runnable #1 'HEAD' sending output '1' to Runnable #0 'HEAD-1' input #0
DEBUG   -                       Runnable #1 is now blocked on output by Runnable #0
DEBUG   -                       Runnable #0 inputs are ready
DEBUG   -               Runnable #1 'HEAD' sending output '1' to Runnable #2 'sum' input #1
DEBUG   -                       Runnable #1 is now blocked on output by Runnable #2
DEBUG   -                       Runnable #2 inputs are ready
DEBUG   -                       Runnable #2 not blocked on output, so added to end of 'Will Run' list
DEBUG   -       Runnable #1 'HEAD' completed
DEBUG   - Dispatch count: 2
DEBUG   -        Can Run: {3, 2, 0}
DEBUG   -       Blocking: [(2, 0), (3, 0), (0, 1), (2, 1)]
DEBUG   -       Will Run: [3, 2]
```