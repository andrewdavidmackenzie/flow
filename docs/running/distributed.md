## Distributed execution of jobs with `flowr` and `flowrex`

### Job Dispatch and Job Execution
The `flowrlib` that is used by flow runner applications to execute a flow has two important functions:
- job dispatch - that managers the state of the flow, the dispatch of jobs for execution, and distribution
of results received back, passing those results onto other functions in the flow etc.
- job execution - this is the execution of "pure" functions, receiving a set of input data, a reference
to the function's implementation. It executes it with the provided input, and returns the job including
the results.

Job dispatch is done by the server thread running the coordinator, responsible for maintaining a consistent 
state for the flow and it's functions and coordinating the distribution of results and enabling of
new functions to be run.

Additional threads are started for job execution, allowing many jobs to be executed concurrently, and
in parallel on a multi-core machine. Job execution on "pure" functions can run in isolation, just needing
the input data and the function implementation.

### Normal Execution
Normally, the `flowr` process runs the coordinator in one thread and a number of executors in additional
threads.

However, due to the "pure" nature of the job execution, it can be done anywhere, including in additional 
processes, or on processes in additional machines.

### `flowrex` executor binary
`florex` is an additional small binary that is built. 
It cannot coordinate the execution of a flow but it can execute (just library for now) jobs.

Additional instances of `flowrex` can be started in other processes on the same machine and have it 
execute some of the jobs, increasing compute resources and concurrency/parallelism of flow execution.

It is possible to start `flowr` with 0 executor threads and force `flowrex` to execute all the 
(library) jobs.

It can also be ran on another node, even one with a different architecture such as ARM, on the network and have job 
execution done entirely by it or shared with flowr.

How many jobs are done in one process/machine or another depends on the number of executors and network and cpu speed.

The `flowr` flow runner and the `flowrex` job executor discover each other using mDNS
and then jobs are distributed out over the network and results are sent back
to the coordinator running in `flowr` also over the network.

### TODO
It is pending to allow `flowrec` to also execute provided functions, by distributing the architecture-neutral WASM 
function implementations to other nodes and hence allow them to load and run those functions also.

### Example of distributed execution
This can be done in two terminals on the same machine, or across two machines of the same or different CPU architecture.

Terminal 1

Start an instance of `flowrex` that will wait for jobs to execute.
(we start with debug logging level to see what's happening)

`> flowrex -v debug`

The log output should end with

`INFO    - Waiting for beacon matching 'jobs._flowr._tcp.local'`

indicating that it is waiting to discover the `flowr` process on the network.

Terminal 2

First let's compile the fibonacci sample (but not run it) by using `flowc` with the `-c, --compile` option:

`>  flowc -c -C flowr/src/cli flowsamples/fibonacci`

Let's check that worked:

```
> ls flowsamples/fibonacci/manifest.json
flowsamples/fibonacci/manifest.json
```

Then let's run the sample fibonacci flow, forcing zero executors threads so that we 
see `flowrex` executing all (non context) jobs

`> flowr -t 0 flowsamples/fibonacci`

That will produce the usual fibonacci series on the STDOUT of Terminal 2, then `flowr` exiting

Logs of what is happening in order to execute the flow jobs will be produced in Terminal 1, ending with the same line
as before:

`INFO    - Waiting for beacon matching 'jobs._flowr._tcp.local'`

Indicating that it has returned to the initial state and is ready to discover a new flowr dispatcher of jobs to it.


