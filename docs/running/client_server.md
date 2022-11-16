## Running a flow in client/server mode of `flowr`

### `flowrlib` architecture
The `flowrlib` library is designed to be used, not just in `flowr` CLI-style flow runners, but in other incarnations
such as a GUI application, or web application, etc.

In order to have `flowrlib` work well in such applications, it avoids running any `context function` function that
interacts with the environment (Read/Write to a FIle, Read/Write to STDIO, etc) and that may block, on the main
thread running the "coordinator" that managers flow execution.

Different applications, like a GUI App, may need to provide totally different implementations for some of 
those functions, provided by the application and not the library.

For this reason, it implements a "client/server" architecture, where a "server" thread runs the coordinator
and sends and receives messages to a client thread (in the flow runner app) that runs the `context functions` whose 
implementations are provided by the flow runner application that links the `flowrlib` library.

### `flowr` - an example of a flow runner app
`flowr` is one example of a flow runner app that uses `flowrlib` to build an application to run flows.

It implements a set of `client function`, that interact with STDIO etc, on a client thread.

The `flowr` process running that client thread must be able to interact with STDIO.

In normal use, `flowr` runs the client and server threads in the same process and the user is unaware of this
separation.

### Separating the client from the server
However, `flowr` can be run as two separate processes, one "client" process that executes the `context functions`
and interacts with STDIO, and another "server" process with a thread that that runs the coordinator plus a number 
of threads running executors for job execution.

These two "client" and "server" processes exchange messages over the network.
The two processes can be on the same node/machine or on separate machines. The one running the "client"
should be able to interact with the FileSystem and STDIO and interact with the user. The "server" does not run
any such function and does not need to interact with the user.

They use mDNS and service discovery to discover the network address and port of the other process, running within
the same network.

### Example of running a flow with "client" separate from "server"
First let's compile the fibonacci sample (but not run it) by using `flowc` with the `-c, --compile` option:

`>  flowc -c -C flowr/src/cli flowsamples/fibonacci`

Let's check that worked:

```
> ls flowsamples/fibonacci/manifest.json
flowsamples/fibonacci/manifest.json
```

In Terminal 1, lets start the server that will wait for a flow to be submitted for execution,
using `flowr` with debug logging verbosity level to be able to see what it's doing.

`> flowr -n -s -v debug`

which will log some lines, ending with:

`INFO    - Server is waiting to receive a 'Submission'`

In Terminal 2, let's start a client using `flowr` with the `-c, --client` option. 
This will submit the flow to the server for execution over the network, reading the flow manifest from the File
System. It will then execute the `client functions`, in response to messages from the server, providing STDIO (just 
standard out in this example)

`> flowr -c flowsamples/fibonacci`

That will produce the usual fibonacci series on the STDOUT of Terminal 2.

Logs of what is happening in order to execute the flow will be produced by the server in Terminal 1, ending with 

`INFO    - Server is waiting to receive a 'Submission'`

which indicates the server has returned to the initial state, ready to receive another flow for execution.

You can execute the flow again by repeating the same command in Terminal 2.

In order to exit the server, in Terminal 1 just hit Control-C.
