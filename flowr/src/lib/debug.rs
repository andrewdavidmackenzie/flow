use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use flowcore::function::Function;

use crate::run_state::{Block, Job, RunState, State};

/// Types of `Params` used in communications between the debugger and the debug_client
#[derive(Serialize, Deserialize)]
pub enum Param {
    /// A "*" style parameter - meaning will depend on the `Command` it's use with
    Wildcard,
    /// A positive integer was specified - could be a function or a job number
    Numeric(usize),
    /// A descriptor for the `Output` of a `Function` was specified
    Output((usize, String)),
    /// A descriptor for the `Inout` of a `Function` was specified
    Input((usize, usize)),
    /// A description of a "block" (when one function is blocked from running by another) was specified
    Block((usize, usize)),
}

/// A debugger Response sent by the debug_client to the debug server
#[derive(Serialize, Deserialize)]
pub enum Response {
    /// Acknowledge event processed correctly
    Ack,
    /// Set a `breakpoint` - with an optional parameter
    Breakpoint(Option<Param>),
    /// `continue` execution of the flow
    Continue,
    /// `delete` an existing breakpoint - with an optional parameter
    Delete(Option<Param>),
    /// `enter` the debugger at the next opportunity the runtime has
    EnterDebugger,
    /// An error on the client side
    Error(String),
    /// `exit` the debugger and runtime
    ExitDebugger,
    /// `inspect` a function
    InspectFunction(usize),
    // Inspect overall state
    Inspect,
    /// Invalid - used when deserialization goes wrong
    Invalid,
    /// `list` existing breakpoints
    List,
    /// `reset` flow execution back to the initial state
    RunReset,
    /// `step` forward in flow execution by executing one (default) or more `Jobs`
    Step(Option<Param>),
    /// `validate` the current state
    Validate,
}

/// A run-time event that the debugger communicates to the debug_client for it to decide
/// what to do, or what to request of the user
#[derive(Serialize, Deserialize)]
pub enum Event {
    /// A `Job` ran to completion by a function - includes:  job_id, function_id
    JobCompleted(usize, usize, Option<Value>),
    /// Entering the debugger
    EnteringDebugger,
    /// The debugger/run-time is exiting
    ExitingDebugger,
    /// The run-time is about to send a `Job` for execution - an opportunity to break
    /// includes: job_id, function_id
    PriorToSendingJob(usize, usize),
    /// A breakpoint on a `Block` between two functions was encountered
    /// includes: blocked_id, blocking_id, blocking_io_number
    BlockBreakpoint(Block),
    /// A breakpoint on a `Value` being sent between two functions was encountered
    /// includes: source_process_id, output_route, value, destination_id, input_number));
    DataBreakpoint(usize, String, Value, usize, usize),
    /// A panic occurred executing a `Flows` `Job` -  includes the output of the job that panicked
    Panic(String, usize),
    /// There was an error executing the Job
    JobError(Job),
    /// Execution of the flow has started
    ExecutionStarted,
    /// Execution of the flow has ended
    ExecutionEnded,
    /// A check has detected that there is a deadlock between functions impeding more execution
    Deadlock(String),
    /// A value is being sent from the output of one function to the input of another
    /// includes: source_process_id, value, destination_id, input_number
    SendingValue(usize, Value, usize, usize),
    /// An error was detected - includes: A string describing the error
    Error(String),
    /// The state of a function
    FunctionState((Function, State)),
    /// The overall state
    OverallState(RunState),
    /// A message for display to the user of the debug_client
    Message(String),
    /// The run-time is resetting the status back to the initial state
    Resetting,
    /// Debugger is blocked waiting for a command before proceeding
    WaitingForCommand(usize),
    /// Invalid - used when deserialization goes wrong
    Invalid,
}
