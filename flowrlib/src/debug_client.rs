use serde_json::Value;

use crate::run_state::Output;

/// Types of `Params` used in communications between the debugger and the debug_client
pub enum Param {
    /// A "*" style parameter - meaning will depend on the `Command` it's use with
    Wildcard,
    /// A positive integer was specified
    Numeric(usize),
    /// A descriptor for the `Output` of a `Function` was specified
    Output((usize, String)),
    /// A descriptor for the `Inout` of a `Function` was specified
    Input((usize, usize)),
    /// A description of a "block" (when one function is blocked from running by another) was specified
    Block((usize, usize)),
}

/// A debugger command entered by the user in the client and sent to the debugger runtime
pub enum Command {
    /// Set a `breakpoint` - with an optional parameter
    Breakpoint(Option<Param>),
    /// `continue` execution of the flow
    Continue,
    /// `delete` an existing breakpoint - with an optional parameter
    Delete(Option<Param>),
    /// `exit` the debugger and runtime
    ExitDebugger,
    /// `inspect` the current state
    Inspect,
    /// `list` existing breakpoints
    List,
    /// `print` a function or functions state
    Print(Option<Param>),
    /// `reset` flow execution back to the initial state
    RunReset,
    /// `step` forward in flow execution by executing one `Job`
    Step(Option<Param>),
    /// Get the state of the `Flow`
    GetState,
    /// Get the state of a specific `Function`
    GetFunctionState(usize)
}

/// A run-time event that the debugger communicates to the debug_client for it to decide
/// what to do, or what to request of the user
pub enum Event {
    /// A `Job` ran to completion by a function
    /// includes:  job_id, function_id
    JobCompleted(usize, usize, Option<Value>),
    /// A `Flow` execution was started - start the debug_client
    Start,
    /// The run-time is about to send a `Job` for execution - an opportunity to break
    /// includes: job_id, function_id
    PriorToSendingJob(usize, usize),
    /// A breakpoint on a `Block` between two functions was encountered
    /// includes: blocked_id, blocking_id, blocking_io_number
    BlockBreakpoint(usize, usize, usize),
    /// A breakpoint on a `Value` being sent between two functions was encountered
    /// includes: source_process_id, output_route, value, destination_id, input_number));
    DataBreakpoint(usize, String, Value, usize, usize),
    /// A panic occured executing a `Flows` `Job` -  includes the output of the job that panicked
    Panic(Output),
    /// There was an error reported by the run-time
    RuntimeError(String), // message resulting from the error
    /// End of debug session - debug_client should disconnect
    End,
    /// A check has detected that there is a deadlock between functions impeding more execution
    Deadlock(String),
    /// A value is being sent from the output of one function to the input of another
    /// incluides: source_process_id, value, destination_id, input_number
    SendingValue(usize, Value, usize, usize),
}

/// A `Response` from the debugger and run-time to a command from the debug_client
pub enum Response {
    /// Simple acknowledgement
    Ack,
    /// An error was detected
    /// includes: A string describing the error
    Error(String),
    /// A message for display to the user of the debug_client
    /// includes: A string to be displayed
    Message(String),
    /// The run-time is resetting the status back to the initial state
    Resetting,
    /// The debugger/run-time is running
    Running,
    /// The debugger/run-time is exiting
    Exiting
}

/// debug_clients must implement this trait
pub trait DebugClient {
    /// Called at init to initalize the client
    fn init(&self);
    /// Called to fetch the next command from the debug_client
    fn get_command(&self, job_number: Option<usize>) -> Command;
    /// Called to send an event to the debug_client
    fn send_event(&self, event: Event);
    /// Called to send a response from the debug/run-time to the debug_client
    fn send_response(&self, response: Response);
}