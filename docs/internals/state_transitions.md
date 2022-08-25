## Flow Execution State Machine

### States
Prior to initialization, all functions will be in the `Initial` state.

The Initialization step described below is run, after which all functions will be in one or more of the
following states (see `State` struct in `run_state.rs`):
- `Ready` - Inputs are satisfied, the Output destinations are free and it can be run
- `Blocked`- One or more destination inputs this functions sends to is full, blocking execution
- `Waiting` - One or more of the inputs lack data, so the function cannot run
- `Running` - There is at least one job running that is using this function
- `Completed` - The function has returned FALSE for "RunAgain" and is not available for execution

### Events that cause state changes
The following top level events trigger evaluation of a function's state using the state variables and may cause 
a function to transition to a new state:
- `NewJob`- A new job is created by the runner for a specific function, taking values from its inputs. This will 
  cause the flow containing the function in the job to also be marked as busy, preventing functions from outside the 
  flow sending to it while it is busy. Functions inside the same flow that were previously blocked sending to 
  this function are now unblocked as the inputs are available to send to (unblock_internal_flow_senders). Functions 
  from outside the flow attempting to send to it are blocked by "flow_blocks" that are
  removed when the flow goes idle later (as all the functions within it go idle).
- `JobDone` - a job that was using a function completes, returning a result to the runner that includes the 
  `run_again`value. 
  This may cause a change in state in the function that was running in the job, and via the `ValueReceived` event 
  below it may also affect other functions it sends values to.
- `ValueReceived` - a function receives a value on one of its inputs, caused either by:
  - An "Input Initializer" on the function being run
  - A value was sent to it from another function on `JobDone`
  - A value was sent to it from itself upon `JobDone` (loopback)
- `UnBlock` - previously a function was blocked from running as a destination it sends to had it's inputs full. That 
  function ran and it's inputs were freed, and it is not blocked on any other destination so the sender can now be 
  unblocked. Functions that were blocked sending to the function being used in the job _may_ become unblocked and so 
  produces multiple UnBocks

### State Variables
State variables for a function can be calculated at any time based on its inputs states, and other functions states.
They are used in determining the next state that a function should be transitioned to when an event occurs:
- `needs_input` - the function has at least one input that has no data on it, and so the function cannot run
- `output_blocked` - the function has at least one destination input that is full and so it cannot send a result
  value to that destination, hence the function cannot run

### State Transitions
An event may cause the affected functions to transition to a new state, based on its state variables:
- `NewJob`
  `Ready` --> `Running`     - The function used in the job transitions to `Running`
- `JobDone` (job_done)
  `Running` --> `Completed` - !`run_again`                                        (job_done)
  `Running` --> `Waiting`   -  `run_again` &&  `needs_input`
  `Running` --> `Blocked`   -  `run_again` && !`needs_input` && `output_blocked`  (make_ready_or_blocked)
  `Running` --> `Ready`     -  `run_again` && !`needs_input` && !`output_blocked` (make_ready_or_blocked)
- `ValueReceived` - a function receives a value on one of its inputs.             (send_a_value)
  `Waiting` --> `Waiting`   -  `needs_input`
  `Waiting` --> `Blocked`   - !`needs_input` && `output_blocked`  (make_ready_or_blocked)
  `Blocked` --> `Blocked`   - !`needs_input` && `output_blocked`  (make_ready_or_blocked)
  `Waiting` --> `Ready`     - !`needs_input` && !`output_blocked` (make_ready_or_blocked)
- `UnBlock` - (remove_blocks) <-- (unblock_flows, unblock_internal_flow_senders) <-- (job_done)
  `Blocked` --> `Ready`

### State Transition Diagram

                             +---------+
                             | Initial |
                             +---------+
                                  |
                                  |ValueReceived (via InputInitializer)
                                  v
                UnBlock      +---------+    ValueReceived
           +---------------> |  Ready  |<--------------------+
           |                 +---------+                     |
           |                  ^       |                      |
           |           JobDone|       |NewJob                |
      +---------+             |       |                   +---------+
      | Blocked |<------------|-------|-------------------| Waiting |
      +---------+             |       |                   +---------+
           ^                  |       |                        ^
           |                  |       v                        |
           |   JobDone       +---------+      JobDone          |
           +-----------------| Running |-----------------------+
                             +---------+        
                                  |
                                  |JobDone
                                  v
                             +---------+
                             |Completed|
                             +---------+
