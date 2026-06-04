//! Defines [`DebugServerMessage`] — messages sent from the debug server (in the coordinator)
//! to a debug client (such as `flowdb` or a GUI debugger).

use std::fmt;

use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

use flowcore::model::block::Block;
use flowcore::model::input::Input;
use flowcore::model::job::Job;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::run_state::{RunState, State};

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
    /// A breakpoint on a `Block` between two functions was encountered
    /// includes: `blocked_id`, `blocking_id`, `blocking_io_number`
    BlockBreakpoint(Block),
    /// A breakpoint on a `Flow` that was busy going idle (and unblocking senders to it) was hit
    FlowUnblockBreakpoint(usize),
    /// A breakpoint on a `Value` being sent between two functions was encountered
    /// includes: `source_process_id`, `output_route`, `value`, `destination_id`, `function_name`,
    /// `io_name`, `input_number`));
    DataBreakpoint(String, usize, String, Value, usize, String, String, usize),
    /// A panic occurred executing a `Flows` `Job` -  includes the output of the job that panicked
    Panic(String, usize),
    /// There was an error executing the Job
    JobError(Job),
    /// A check has detected that there is a deadlock between functions impeding more execution
    Deadlock(String),
    /// Execution of the flow has started
    ExecutionStarted,
    /// Execution of the flow has ended
    ExecutionEnded,
    /// An error was detected - includes: A string describing the error
    Error(String),
    /// A list of all functions
    Functions(Vec<RuntimeFunction>),
    /// The state of a function
    FunctionStates((RuntimeFunction, Vec<State>)),
    /// A value is being sent from the output of one function to the input of another
    /// includes: `source_process_id`, value, `destination_id`, `input_number`
    SendingValue(usize, Value, usize, usize),
    /// The overall state
    OverallState(RunState),
    /// The state of an Input - optional values on it
    InputState(Input),
    /// The state of an Output - list of connections
    OutputState(Vec<OutputConnection>),
    /// One or more Blocks
    BlockState(Vec<Block>),
    /// A message for display to the user of the `debug_client`
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
                DebugServerMessage::BlockState(_) => "BlockState",
                DebugServerMessage::Message(_) => "Message",
                DebugServerMessage::Resetting => "Resetting",
                DebugServerMessage::WaitingForCommand(_) => "WaitingForCommand",
                DebugServerMessage::Invalid => "Invalid",
                DebugServerMessage::FlowUnblockBreakpoint(_) => "FlowUnblockBreakpoint",
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod test {
    use super::DebugServerMessage;

    #[test]
    fn roundtrip_simple_variants() {
        let variants: Vec<DebugServerMessage> = vec![
            DebugServerMessage::EnteringDebugger,
            DebugServerMessage::ExitingDebugger,
            DebugServerMessage::ExecutionStarted,
            DebugServerMessage::ExecutionEnded,
            DebugServerMessage::Resetting,
            DebugServerMessage::WaitingForCommand(42),
            DebugServerMessage::Error("test error".into()),
            DebugServerMessage::Message("hello".into()),
            DebugServerMessage::Deadlock("deadlock info".into()),
            DebugServerMessage::Panic("panic msg".into(), 10),
            DebugServerMessage::FlowUnblockBreakpoint(5),
        ];

        for variant in variants {
            let serialized: String = variant.into();
            assert!(!serialized.is_empty());
            let deserialized: DebugServerMessage = serialized.into();
            let reserialized: String = deserialized.into();
            assert!(!reserialized.is_empty());
        }
    }

    #[test]
    fn invalid_string_yields_invalid() {
        let msg: DebugServerMessage = "not valid json".to_string().into();
        assert_eq!(format!("{msg}"), "DebugServerMessage Invalid");
    }

    #[test]
    fn display_variants() {
        assert_eq!(
            format!("{}", DebugServerMessage::EnteringDebugger),
            "DebugServerMessage EnteringDebugger"
        );
        assert_eq!(
            format!("{}", DebugServerMessage::WaitingForCommand(0)),
            "DebugServerMessage WaitingForCommand"
        );
        assert_eq!(
            format!("{}", DebugServerMessage::Invalid),
            "DebugServerMessage Invalid"
        );
    }
}
