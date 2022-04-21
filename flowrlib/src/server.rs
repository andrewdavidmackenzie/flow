#[cfg(feature = "debugger")]
use serde_json::Value;

use flowcore::errors::*;
#[cfg(feature = "debugger")]
use flowcore::model::input::Input;
#[cfg(feature = "debugger")]
use flowcore::model::output_connection::OutputConnection;
#[cfg(feature = "debugger")]
use flowcore::model::runtime_function::RuntimeFunction;
use flowcore::model::submission::Submission;

#[cfg(feature = "debugger")]
use crate::block::Block;
#[cfg(feature = "debugger")]
use crate::debug_command::DebugCommand;
#[cfg(feature = "debugger")]
use crate::job::Job;
#[cfg(feature = "debugger")]
use crate::run_state::{RunState, State};

/// A `Server` implements a number of "callbacks" to communicate between a CLI/UI and background
/// flow coordinator executing the flow
pub trait Server {
    /// The flow is starting
    fn flow_starting(&mut self) -> Result<()>;

    /// See if the runtime client has sent a message to request us to enter the debugger,
    /// if so, return Ok(true).
    /// A different message or Absence of a message returns Ok(false)
    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool>;

    /// The flow has ended
    #[cfg(feature = "metrics")]
    fn flow_ended(&mut self, state: &mut RunState) -> Result<()>;
    #[cfg(not(feature = "metrics"))]
    fn flow_ended(&mut self) -> Result<()>;

    /// Wait for a `Submission` to be sent for execution
    fn wait_for_submission(&mut self) -> Result<Option<Submission>>;

    /// The flow server is about to exit
    fn server_exiting(&mut self, result: Result<()>) -> Result<()>;
}

/// a `DebugServer` implements these "callbacks" in order to communicate between a CLI/UI
/// implementation of one and the background flow coordinator executing the flow and debugger
#[cfg(feature = "debugger")]
pub trait DebugServer {
    /// Start the debugger - which swallows the first message to initialize the connection
    fn start(&mut self);
    /// a breakpoint has been hit on a Job being created
    fn job_breakpoint(&mut self, job: &Job, function: &RuntimeFunction, states: Vec<State>);
    /// A breakpoint set on creation of a `Block` matching `block` has been hit
    fn block_breakpoint(&mut self, block: &Block);
    /// A breakpoint on sending a value from a specific function or to a specific function was hit
    #[allow(clippy::too_many_arguments)]
    fn send_breakpoint(&mut self, source_function_name: &str, source_function_id: usize,
                       output_route: &str, value: &Value, destination_id: usize,
                       destination_name: &str, io_name: &str, input_number: usize);
    /// A job error occurred during execution of the flow
    fn job_error(&mut self, job: &Job);
    /// A specific job completed
    fn job_completed(&mut self, job: &Job);
    /// returns a set of blocks
    fn blocks(&mut self, blocks: Vec<Block>);
    /// returns an output's connections
    fn outputs(&mut self, output: Vec<OutputConnection>);
    /// returns an inputs state
    fn input(&mut self, input: Input);
    /// lists all functions
    fn function_list(&mut self, functions: &[RuntimeFunction]);
    /// returns the state of a function
    fn function_states(&mut self, function: RuntimeFunction, function_states: Vec<State>);
    /// returns the global run state
    fn run_state(&mut self, run_state: &RunState);
    /// a string message from the Debugger
    fn message(&mut self, message: String);
    /// a panic occurred during execution
    fn panic(&mut self, state: &RunState, error_message: String);
    /// the debugger is exiting
    fn debugger_exiting(&mut self);
    /// The debugger is resetting the runtime state
    fn debugger_resetting(&mut self);
    /// An error occurred in the debugger
    fn debugger_error(&mut self, error: String);
    /// execution of the flow is starting
    fn execution_starting(&mut self);
    /// Execution of the flow fn execution_ended(&mut self, state: &RunState) {
    fn execution_ended(&mut self);
    /// Get a command for the debugger to perform
    fn get_command(&mut self, state: &RunState) -> Result<DebugCommand>;
}