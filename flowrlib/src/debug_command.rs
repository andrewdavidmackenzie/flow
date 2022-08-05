use std::fmt;

use serde_derive::{Deserialize, Serialize};

/// Types of `Params` used in communications between the debugger and the debug_client
#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum BreakpointSpec {
    /// All existing breakpoints
    All,
    /// A positive integer was specified - could be a function or a job number
    Numeric(usize),
    /// A descriptor for the `Output` of a `Function` was specified
    Output((usize, String)),
    /// A descriptor for the `Inout` of a `Function` was specified
    Input((usize, usize)),
    /// A description of a "block" (when one function is blocked from running by another) was specified
    Block((Option<usize>, Option<usize>)),
}

/// A Command sent by the debug_client to the debugger
#[derive(Serialize, Deserialize)]
pub enum DebugCommand {
    /// Acknowledge event processed correctly
    Ack,
    /// Set a `breakpoint` - with an optional parameter
    Breakpoint(Option<BreakpointSpec>),
    /// `continue` execution of the flow
    Continue,
    /// Debug client is starting
    DebugClientStarting,
    /// `delete` an existing breakpoint - with an optional parameter
    Delete(Option<BreakpointSpec>),
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
    /// `modify` a debugger or runtime state value e.g. jobs=1 to set parallel jobs to 1
    Modify(Option<Vec<String>>),
    /// `reset` flow execution back to the initial state, or run the flow from the start
    RunReset,
    /// `step` forward in flow execution by executing one (default) or more `Jobs`
    Step(Option<usize>),
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
                DebugCommand::Modify(_) => "Modify"
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
