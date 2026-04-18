//! Minimal no-op debug handler for flowedit.
//!
//! The coordinator requires a DebuggerHandler, but flowedit does not support
//! debugging. This stub satisfies the trait.

use flowcore::model::input::Input;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::block::Block;
use flowrlib::debug_command::DebugCommand;
use flowrlib::debugger_handler::DebuggerHandler;
use flowrlib::job::Job;
use flowrlib::run_state::{RunState, State};
use serde_json::Value;

use crate::coordinator::coordinator_connection::CoordinatorConnection;

/// A no-op debug handler — flowedit does not support debugging yet.
pub(crate) struct NoOpDebugHandler {
    pub(crate) connection: CoordinatorConnection,
}

impl DebuggerHandler for NoOpDebugHandler {
    fn start(&mut self) {}
    fn job_breakpoint(&mut self, _job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {}
    fn block_breakpoint(&mut self, _block: &Block) {}
    fn flow_unblock_breakpoint(&mut self, _flow_id: usize) {}
    fn send_breakpoint(
        &mut self,
        _source_function_name: &str,
        _source_function_id: usize,
        _output_route: &str,
        _value: &Value,
        _destination_id: usize,
        _destination_name: &str,
        _io_name: &str,
        _input_number: usize,
    ) {
    }
    fn job_error(&mut self, _job: &Job) {}
    fn job_completed(&mut self, _job: &Job) {}
    fn blocks(&mut self, _blocks: Vec<Block>) {}
    fn outputs(&mut self, _output: Vec<OutputConnection>) {}
    fn input(&mut self, _input: Input) {}
    fn function_list(&mut self, _functions: &[RuntimeFunction]) {}
    fn function_states(&mut self, _function: RuntimeFunction, _states: Vec<State>) {}
    fn run_state(&mut self, _run_state: &RunState) {}
    fn message(&mut self, _message: String) {}
    fn panic(&mut self, _state: &RunState, _error_message: String) {}
    fn debugger_exiting(&mut self) {}
    fn debugger_resetting(&mut self) {}
    fn debugger_error(&mut self, _error: String) {}
    fn execution_starting(&mut self) {}
    fn execution_ended(&mut self) {}
    fn get_command(&mut self, _state: &RunState) -> flowcore::errors::Result<DebugCommand> {
        Ok(DebugCommand::Continue)
    }
}
