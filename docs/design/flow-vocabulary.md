# Flow Runtime Vocabulary

## Functions and Flows

- **Function**: A unit of computation with inputs and outputs. Identified by `function_id`. Lives in exactly
  one flow.
- **Flow**: A group of functions that form a logical unit. Identified by `flow_id`. Flow 0 is the **root flow**.
  Flows with id > 0 are **sub-flows**.
- **Sub-flow**: A flow nested inside another. Its functions may receive values from, or output values to, the
  **parent flow** via parent flow connections.
- **Parent flow**: The flow that contains a reference to a sub-flow and sends values into it, and may receive
  values from it.

## Inputs and Values

- **Input**: A numbered port on a function that receives values. It may have a name. Has a queue of pending
  values.
- **Input queue**: The ordered list of values waiting to be consumed on an input. Values are consumed FIFO.
- **Input set**: A complete set of values (one per input) needed for a function to run. Taken from the head
  of each input queue.
- **Full input**: An input with at least one value in its queue.
- **Empty input**: An input with no values in its queue.
- **Ready**: A function is ready when all its inputs have at least one value AND it is not blocked.

## Initializers

- **Function initializer**: An initializer defined directly on the function's input. Applied at startup (Once)
  or after each run (Always). The compiler rejects having both an Always initializer and a connection on the
  same input, so the Always value is always appended to an empty queue.
- **Flow initializer**: An initializer propagated from a parent flow's connection during compilation. Applied
  at startup (Once or Always) and when the flow goes idle (Always only). Unlike function initializers, flow
  initializers CAN coexist with connections on the same input.
- **Once initializer**: Fills the input once at startup. Not re-applied.
- **Always initializer**: For function initializers: fills the input after each function run. For flow
  initializers: fills the input at startup and each time the flow goes idle.

## Jobs

- **Job**: A unit of work consisting of a function's input set and its implementation. Created when a function
  becomes ready.
- **Dispatch**: Sending a job to an executor thread for processing.
- **Retire**: Processing a completed job's results — sending output values to destinations, creating new jobs.
- **Ready queue**: Jobs waiting to be dispatched.
- **Running job**: A job that has been dispatched and is executing.

## Connections and Sending

- **Connection**: A link from a function's output to another function's input.
- **Same-flow connection**: Source and destination are in the same flow.
- **Cross-flow connection**: Source and destination are in different flows.
- **Loopback**: A connection from a function's output back to its own input (source_id == destination_id).
- **Feedback connection**: A same-flow connection that carries values back in a loop (not necessarily to the
  same function).
- **Send**: Delivering a value from a completed job's output to a destination input queue.

## Blocking

### Function blocking

A function becomes **blocked** when it has all inputs ready (could run) but at least one of its cross-flow
output destinations already has a value queued. Running the function would produce output that can't be
delivered, so it is held back.

- **Blocked function**: A function in the `blocked` set. It has a complete input set but cannot be scheduled.
  It transitions from blocked to ready when all its output destinations have consumed their values (blocks
  removed).
- **Output block**: The condition that causes blocking — a cross-flow destination input already has a value.
  Same-flow sends and loopbacks never create blocks.
- **Block record**: An entry in the `blocks` set recording the blocked function (sender), the blocking
  function (destination whose input is full), and which input is full.

### Flow blocking

When a function inside a sub-flow starts a job, external senders (functions in other flows) that send to
that sub-flow are prevented from sending until the sub-flow goes idle. This prevents values from the parent
flow arriving while the sub-flow is busy processing.

- **flow_blocks**: A map of `flow_id → set of function_ids`. When a function starts a job, it registers in
  flow_blocks for its flow. When the flow goes idle, `remove_blocks` is called for each registered function,
  resolving the output blocks that were holding back external senders.

## Function State Transitions

A function can be in one of five states: **Waiting**, **Ready**, **Blocked**, **Running**, **Completed**.

```
From      To         Trigger and conditions
--------  ---------  --------------------------------------------------------
Init      Ready      All inputs initialized AND no destination input full
                     OR: no inputs AND no destination input full
Init      Blocked    All inputs initialized BUT a destination input is full
Init      Waiting    At least one input is not full

Waiting   Ready      A send fills the last empty input, no destination full
Waiting   Blocked    A send fills the last empty input, but a destination full

Ready     Running    Job dispatched for execution

Running   Ready      Job done, all inputs still full, no destination full
Running   Waiting    Job done, at least one input empty
Running   Blocked    Job done, all inputs full but a destination full

Blocked   Ready      Output block removed (destination consumed its value)

Any       Completed  Function indicates it will not run again (run_again=false)
```

Notes:
- Loopback sends (function to itself) never create blocks
- Same-flow sends never create blocks
- Only cross-flow sends can cause a function to become blocked

## Flow State Transitions

A flow can be in one of two states: **Busy** or **Idle**.

```
From   To     Trigger and conditions
-----  -----  --------------------------------------------------------
Init   Idle   No functions are ready after initialization
Init   Busy   At least one function is ready after initialization

Idle   Busy   A job is created for a function in this flow
              (e.g., after external sender delivers values)

Busy   Idle   Last busy function in the flow completes and no new jobs
              are created for functions in this flow
              On transition to idle:
                1. External senders to this flow are unblocked
                2. Always flow initializers are re-applied
```

Notes:
- A flow with blocked functions but no busy functions is considered idle
- The root flow (flow_id=0) going idle is normal between job batches
- A sub-flow going idle signals the end of an iteration, triggering
  parent flow unblocking and flow initializer re-application

## Stale Values

- **Stale value**: A value in an input queue produced by a previous iteration of a loop that will never be
  consumed in that iteration because the loop terminated. It remains in the queue and gets consumed by the
  NEXT iteration, ahead of fresh values.
- **Dead value**: A value sent to a function that cannot run because another input will never be filled (the
  loop terminated without producing a value for that input).

## Execution Cycle (Coordinator Loop)

Each iteration of the coordinator's main loop:
1. **Dispatch**: Send ready jobs to executors
2. **Retire**: Receive one completed result, process it (send outputs, create new jobs)
3. **Check idle**: Determine if any flows transitioned to idle
4. **Terminate check**: If no running jobs and no ready jobs, execution is done
