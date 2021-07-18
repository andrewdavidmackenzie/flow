use std::fmt;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(feature = "distributed")]
use zmq::Message;

use flowcore::function::Function;
use flowcore::input::Input;
use flowcore::output_connection::OutputConnection;

use crate::run_state::{Block, Job, RunState, State};

/// Types of `Params` used in communications between the debugger and the debug_client
#[derive(Serialize, Deserialize, PartialEq)]
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
    Block((Option<usize>, Option<usize>)),
}

/// A Message sent from the debug server to the debug_client
#[derive(Serialize, Deserialize)]
pub enum DebugServerMessage {
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
    /// The state of an Input - optional values on it
    InputState(Input),
    /// The state of an Output - list of connections
    OutputState(Vec<OutputConnection>),
    /// One or more Blocks
    BlockState(Vec<Block>),
    /// A message for display to the user of the debug_client
    Message(String),
    /// The run-time is resetting the status back to the initial state
    Resetting,
    /// Debugger is blocked waiting for a command before proceeding
    WaitingForCommand(usize),
    /// Invalid - used when deserialization goes wrong
    Invalid,
}

impl fmt::Display for DebugServerMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DebugServerMessage {}",
            match self {
                DebugServerMessage::JobCompleted(_, _, _) => "JobCompleted",
                DebugServerMessage::EnteringDebugger => "EnteringDebugger",
                DebugServerMessage::ExitingDebugger => "ExitingDebugger",
                DebugServerMessage::PriorToSendingJob(_, _) => "PriorToSendingJob",
                DebugServerMessage::BlockBreakpoint(_) => "BlockBreakpoint",
                DebugServerMessage::DataBreakpoint(_, _, _, _, _) => "DataBreakpoint",
                DebugServerMessage::Panic(_, _) => "Panic",
                DebugServerMessage::JobError(_) => "JobError",
                DebugServerMessage::ExecutionStarted => "ExecutionStarted",
                DebugServerMessage::ExecutionEnded => "ExecutionEnded",
                DebugServerMessage::Deadlock(_) => "Deadlock",
                DebugServerMessage::SendingValue(_, _, _, _) => "SendingValue",
                DebugServerMessage::Error(_) => "Error",
                DebugServerMessage::FunctionState(_) => "FunctionState",
                DebugServerMessage::OverallState(_) => "OverallState",
                DebugServerMessage::InputState(_) => "InputState",
                DebugServerMessage::OutputState(_) => "OutputState",
                DebugServerMessage::BlockState(_) => "BlockState",
                DebugServerMessage::Message(_) => "Message",
                DebugServerMessage::Resetting => "Resetting",
                DebugServerMessage::WaitingForCommand(_) => "WaitingForCommand",
                DebugServerMessage::Invalid => "Invalid",
            }
        )
    }
}

/// A Message sent by the debug_client to the debug server
#[derive(Serialize, Deserialize)]
pub enum DebugClientMessage {
    /// Acknowledge event processed correctly
    Ack,
    /// Set a `breakpoint` - with an optional parameter
    Breakpoint(Option<Param>),
    /// `continue` execution of the flow
    Continue,
    /// `delete` an existing breakpoint - with an optional parameter
    Delete(Option<Param>),
    /// An error on the client side
    Error(String),
    /// `exit` the debugger and runtime
    ExitDebugger,
    /// `inspect` a function
    InspectFunction(usize),
    /// Inspect overall state
    Inspect,
    /// Inspect an Input (function_id, input_number)
    InspectInput(usize, usize),
    /// Inspect an Output (function_id, sub-path)
    InspectOutput(usize, String),
    /// Inspect a Block (optional source function_id, optional destination function_id)
    InspectBlock(Option<usize>, Option<usize>),
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

impl fmt::Display for DebugClientMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DebugClientMessage {}",
            match self {
                DebugClientMessage::Ack => "Ack",
                DebugClientMessage::Breakpoint(_) => "Breakpoint",
                DebugClientMessage::Continue => "Continue",
                DebugClientMessage::Delete(_) => "Delete",
                DebugClientMessage::Error(_) => "Error",
                DebugClientMessage::ExitDebugger => "ExitDebugger",
                DebugClientMessage::InspectFunction(_) => "InspectFunction",
                DebugClientMessage::Inspect => "Inspect",
                DebugClientMessage::InspectInput(_, _) => "InspectInput",
                DebugClientMessage::InspectOutput(_, _) => "InspectOutput",
                DebugClientMessage::InspectBlock(_, _) => "InspectBlock",
                DebugClientMessage::Invalid => "Invalid",
                DebugClientMessage::List => "List",
                DebugClientMessage::RunReset => "RunReset",
                DebugClientMessage::Step(_) => "Step",
                DebugClientMessage::Validate => "Validate",
            }
        )
    }
}

#[cfg(feature = "distributed")]
impl From<DebugServerMessage> for Message {
    fn from(debug_event: DebugServerMessage) -> Self {
        match serde_json::to_string(&debug_event) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new(),
        }
    }
}

#[cfg(feature = "distributed")]
impl From<Message> for DebugServerMessage {
    fn from(msg: Message) -> Self {
        match msg.as_str() {
            Some(message_string) => match serde_json::from_str(message_string) {
                Ok(message) => message,
                _ => DebugServerMessage::Invalid,
            },
            _ => DebugServerMessage::Invalid,
        }
    }
}

#[cfg(feature = "distributed")]
impl From<DebugClientMessage> for Message {
    fn from(msg: DebugClientMessage) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => Message::from(&message_string),
            _ => Message::new(),
        }
    }
}

#[cfg(feature = "distributed")]
impl From<Message> for DebugClientMessage {
    fn from(msg: Message) -> Self {
        match msg.as_str() {
            Some(message_string) => match serde_json::from_str(message_string) {
                Ok(message) => message,
                _ => DebugClientMessage::Invalid,
            },
            _ => DebugClientMessage::Invalid,
        }
    }
}
