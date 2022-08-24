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

### Events
The following events trigger evaluation of a function's state using the state variables and may cause it to transition 
to a new state.
- `JobCompleted` - a job that was using a function has just completed
- `InputReceived` - a function receives a value on one of its inputs, caused either by:
  - A job for another function that is connected to this input has completed, and a value was sent to one of its inputs
  - A job using this function has just completed and a loopback connection sent a value to one of its inputs
  - A job using this function has just completed and an "Input Initializer" caused one of its inputs to receive a value
- `DestinationBlocked`

### State Variables 
State variables are derived from the functions inputs states, and other runtime state and are used in determining
the next state that a function should be transitioned to:
- `needs_input` - the function has at least one input that has no data on it, and so the function cannot run
- `output_blocked` - the function has at least one destination input that is full and so it cannot send a result 
  value to that destination, hence the function cannot run
- `run_again` - a job using the function has just completed and returned either TRUE or FALSE for "RunAgain"

### State Transition Diagram