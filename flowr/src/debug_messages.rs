use std::fmt;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use flowcore::model::input::Input;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::block::Block;
//use flowrlib::debug_command::DebugCommand;
use flowrlib::job::Job;
use flowrlib::run_state::{RunState, State};

/// A Message sent from the debugger to a debug_client
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize)]
pub enum DebugServerMessage {
    /// A `Job` ran to completion by a function
    JobCompleted(Job),
    /// Entering the debugger
    EnteringDebugger,
    /// The debugger/run-time is exiting
    ExitingDebugger,
    /// The run-time is about to send a `Job` for execution
    PriorToSendingJob(Job),
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
    FunctionState((RuntimeFunction, State)),
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
                DebugServerMessage::JobCompleted(_) => "JobCompleted",
                DebugServerMessage::EnteringDebugger => "EnteringDebugger",
                DebugServerMessage::ExitingDebugger => "ExitingDebugger",
                DebugServerMessage::PriorToSendingJob(_) => "PriorToSendingJob",
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

impl From<DebugServerMessage> for String {
    fn from(msg: DebugServerMessage) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => message_string,
            _ => String::new(),
        }
    }
}

impl From<String> for DebugServerMessage {
    fn from(msg: String) -> Self {
        match serde_json::from_str(&msg) {
            Ok(message) => message,
            _ => DebugServerMessage::Invalid,
        }
    }
}