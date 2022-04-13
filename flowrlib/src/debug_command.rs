use std::fmt;

use serde_derive::{Deserialize, Serialize};

use crate::param::Param;

/// A Command sent by the debug_client to the debugger
#[derive(Serialize, Deserialize)]
pub enum DebugCommand {
    /// Acknowledge event processed correctly
    Ack,
    /// Set a `breakpoint` - with an optional parameter
    Breakpoint(Option<Param>),
    /// `continue` execution of the flow
    Continue,
    /// Debug client is starting
    DebugClientStarting,
    /// `delete` an existing breakpoint - with an optional parameter
    Delete(Option<Param>),
    /// An error on the client side
    Error(String),
    /// `exit` the debugger and runtime
    ExitDebugger,
    /// List of all functions
    FunctionList,
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

impl fmt::Display for DebugCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DebugCommand {}",
            match self {
                DebugCommand::Ack => "Ack",
                DebugCommand::Breakpoint(_) => "Breakpoint",
                DebugCommand::Continue => "Continue",
                DebugCommand::Delete(_) => "Delete",
                DebugCommand::Error(_) => "Error",
                DebugCommand::ExitDebugger => "ExitDebugger",
                DebugCommand::FunctionList => "FunctionList",
                DebugCommand::InspectFunction(_) => "InspectFunction",
                DebugCommand::Inspect => "Inspect",
                DebugCommand::InspectInput(_, _) => "InspectInput",
                DebugCommand::InspectOutput(_, _) => "InspectOutput",
                DebugCommand::InspectBlock(_, _) => "InspectBlock",
                DebugCommand::Invalid => "Invalid",
                DebugCommand::List => "List",
                DebugCommand::RunReset => "RunReset",
                DebugCommand::Step(_) => "Step",
                DebugCommand::Validate => "Validate",
                DebugCommand::DebugClientStarting => "DebugClientStarting",
            }
        )
    }
}

impl From<DebugCommand> for String {
    fn from(msg: DebugCommand) -> Self {
        match serde_json::to_string(&msg) {
            Ok(message_string) => message_string,
            _ => String::new(),
        }
    }
}

impl From<String> for DebugCommand {
    fn from(msg: String) -> Self {
        match serde_json::from_str(&msg) {
            Ok(message) => message,
            _ => DebugCommand::Invalid,
        }
    }
}
