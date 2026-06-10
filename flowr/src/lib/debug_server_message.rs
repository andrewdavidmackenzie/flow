//! Defines [`DebugServerMessage`] — messages sent from the debug server (in the coordinator)
//! to a debug client (such as `flowrdb` or a GUI debugger).

use std::fmt;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use flowcore::model::debug_command::BreakpointSpec;
use flowcore::model::input::Input;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;

use crate::job::Job;
use crate::run_state::{RunState, State};

/// A `Message` sent from the debugger to a `debug_client`
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
    /// A breakpoint on a `Flow` going idle (unblocking senders) was hit
    FlowUnblockBreakpoint(usize),
    /// A breakpoint on a `Value` being sent between two functions was encountered
    DataBreakpoint(String, usize, String, Value, usize, String, String, usize),
    /// A panic occurred executing a `Job`
    Panic(String, usize),
    /// There was an error executing the Job
    JobError(Job),
    /// A deadlock between functions was detected
    Deadlock(String),
    /// Execution of the flow has started
    ExecutionStarted,
    /// Execution of the flow has ended
    ExecutionEnded,
    /// An error was detected
    Error(String),
    /// A list of all functions
    Functions(Vec<RuntimeFunction>),
    /// The state of a function
    FunctionStates((RuntimeFunction, Vec<State>, Vec<usize>)),
    /// Inspect a function — carries function ID and `RunState` for detailed view
    InspectFunction(usize, RunState),
    /// A value is being sent from one function to another
    SendingValue(usize, Value, usize, usize),
    /// The overall state
    OverallState(RunState),
    /// The state of an Input
    InputState(Input),
    /// The state of an Output - list of connections
    OutputState(Vec<OutputConnection>),
    /// A message for display to the user
    Message(String),
    /// Process tree — carries `RunState` for structured rendering
    ProcessTree(RunState),
    /// Inspect by state — carries state name and `RunState`
    InspectByState(String, RunState),
    /// Inspect a flow — carries flow ID and `RunState`
    InspectFlow(usize, RunState),
    /// Inspect a job — carries the Job
    JobInspect(Job),
    /// Execution metrics snapshot
    #[cfg(feature = "metrics")]
    ExecutionMetrics(flowcore::model::metrics::Metrics),
    /// List of flows with names and routes
    FlowList(Vec<(usize, String, String)>),
    /// The list of active breakpoints
    BreakpointList(Vec<BreakpointSpec>),
    /// The run-time is resetting the status back to the initial state
    Resetting,
    /// Debugger is blocked waiting for a command
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
                DebugServerMessage::DataBreakpoint(_, _, _, _, _, _, _, _) => "DataBreakpoint",
                DebugServerMessage::Deadlock(_) => "Deadlock",
                DebugServerMessage::Error(_) => "Error",
                DebugServerMessage::ExecutionStarted => "ExecutionStarted",
                DebugServerMessage::ExecutionEnded => "ExecutionEnded",
                DebugServerMessage::Functions(_) => "Functions",
                DebugServerMessage::FunctionStates(_) => "FunctionState",
                DebugServerMessage::JobError(_) => "JobError",
                DebugServerMessage::SendingValue(_, _, _, _) => "SendingValue",
                DebugServerMessage::OverallState(_) => "OverallState",
                DebugServerMessage::Panic(_, _) => "Panic",
                DebugServerMessage::InputState(_) => "InputState",
                DebugServerMessage::OutputState(_) => "OutputState",
                DebugServerMessage::Message(_) => "Message",
                DebugServerMessage::Resetting => "Resetting",
                DebugServerMessage::WaitingForCommand(_) => "WaitingForCommand",
                DebugServerMessage::Invalid => "Invalid",
                DebugServerMessage::FlowUnblockBreakpoint(_) => "FlowUnblockBreakpoint",
                DebugServerMessage::BreakpointList(_) => "BreakpointList",
                DebugServerMessage::ProcessTree(_) => "ProcessTree",
                DebugServerMessage::InspectByState(_, _) => "InspectByState",
                DebugServerMessage::InspectFlow(_, _) => "InspectFlow",
                DebugServerMessage::InspectFunction(_, _) => "InspectFunction",
                DebugServerMessage::JobInspect(_) => "JobInspect",
                #[cfg(feature = "metrics")]
                DebugServerMessage::ExecutionMetrics(_) => "ExecutionMetrics",
                DebugServerMessage::FlowList(_) => "FlowList",
            }
        )
    }
}

impl From<DebugServerMessage> for String {
    fn from(msg: DebugServerMessage) -> Self {
        serde_json::to_string(&msg).unwrap_or_else(|e| {
            log::error!("Failed to serialize DebugServerMessage: {e}");
            String::new()
        })
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
