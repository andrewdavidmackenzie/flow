## Distributed execution of jobs with `flowrcli` and `flowrex`

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
Normally, the `flowrcli` process runs the coordinator in one thread and a number of executors in additional
threads.

However, due to the "pure" nature of the job execution, it can be done anywhere, including in additional 
processes, or on processes in additional machines.

### `flowrex` executor binary
`florex` is an additional small binary that is built. 
It cannot coordinate the execution of a flow but it can execute (just library for now) jobs.

Additional instances of `flowrex` can be started in other processes on the same machine and have it 
execute some of the jobs, increasing compute resources and concurrency/parallelism of flow execution.

It is possible to start `flowrcli` with 0 executor threads and force `flowrex` to execute all the 
(library) jobs.

It can also be ran on another node, even one with a different architecture such as ARM, on the network and have job 
execution done entirely by it or shared with flowr.

How many jobs are done in one process/machine or another depends on the number of executors and network and cpu speed.

The `flowrcli` flow runner and the `flowrex` job executor discover each other using mDNS
and then jobs are distributed out over the network and results are sent back
to the coordinator running in `flowrcli` also over the network.

### Dynamic Executor Addition

Executors can join and leave during flow execution. The ZMQ PUSH/PULL architecture
distributes jobs automatically to all connected executors via round-robin, so
adding executors mid-run immediately increases parallelism.

#### Flexible startup order

`flowrex` retries service discovery indefinitely, so it can be started **before**
the coordinator. It will wait until `flowrcli` advertises its services, then connect
and start processing jobs. This means startup order does not matter.

#### Mid-run scaling

Starting additional `flowrex` instances while a flow is running works immediately:
1. The new instance discovers the coordinator's services via mDNS
2. It connects to the job and results ZMQ sockets
3. ZMQ round-robins new jobs across all connected executors (including the new one)
4. No coordinator restart or reconfiguration needed

#### Graceful shutdown

Executor threads use a poll timeout to detect when the coordinator has disappeared.
If no jobs or control messages are received for 60 seconds, executor threads exit
gracefully and `flowrex` loops back to wait for a new coordinator.

### TODO
It is pending to allow `flowrex` to also execute provided functions, by distributing
the architecture-neutral WASM function implementations to other nodes and hence allow
them to load and run those functions also.

### Example of distributed execution
This can be done in two terminals on the same machine, or across two machines of the same or different CPU architecture.

#### Starting flowrex first (new: flexible startup order)

Terminal 1 — start `flowrex` (it will wait for the coordinator):

`> flowrex -v info`

The output will show:

`INFO    - Waiting for coordinator to advertise 'jobs' service...`

Terminal 2 — compile and run a flow:

`>  flowc -c -C flowr/src/bin/flowrcli flowr/examples/fibonacci`

`> flowrcli -t 0 flowr/examples/fibonacci`

Terminal 1 will show `flowrex` discovering the services and executing jobs.

#### Starting coordinator first (classic order)

Terminal 1 — compile and run a flow with zero local executors:

`> flowrcli -t 0 flowr/examples/fibonacci`

Terminal 2 — start `flowrex` to execute the jobs:

`> flowrex -v debug`

`flowrex` discovers the coordinator, connects, and begins executing jobs.

#### Adding a second executor mid-run

While a flow is already running with one `flowrex` instance, start another in a
third terminal:

`> flowrex -v info`

It will discover the same coordinator services and join the job pool immediately.
ZMQ distributes subsequent jobs across both executor instances.


