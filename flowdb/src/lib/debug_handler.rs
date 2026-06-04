//! [`DebugHandler`] — implements the [`DebuggerHandler`] trait, bridging the debug server
//! in the coordinator to a debug client over a [`CoordinatorConnection`].
//!
//! This is used by both `flowrcli` and `flowrgui` to handle debugger communication.

use serde_json::Value;

use flowcore::model::block::Block;
use flowcore::model::debug_command::DebugCommand;
use flowcore::model::input::Input;
use flowcore::model::job::Job;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::debugger_handler::DebuggerHandler;
use flowrlib::run_state::{RunState, State};

use crate::coordinator_connection::{CoordinatorConnection, WAIT};
use crate::debug_server_message::DebugServerMessage::{
    BlockBreakpoint, BlockState, DataBreakpoint, EnteringDebugger, Error, ExecutionEnded,
    ExecutionStarted, ExitingDebugger, FlowUnblockBreakpoint, FunctionStates, Functions,
    InputState, JobCompleted, JobError, Message, OutputState, OverallState, Panic,
    PriorToSendingJob, Resetting, WaitingForCommand,
};

/// A debug handler that bridges the coordinator's debugger to a debug client over ZMQ
pub struct DebugHandler {
    /// The ZMQ connection to the debug client
    pub debug_server_connection: CoordinatorConnection,
}

impl DebuggerHandler for DebugHandler {
    fn start(&mut self) {
        let _ = self.debug_server_connection.receive::<DebugCommand>(WAIT);
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(EnteringDebugger);
    }

    fn job_breakpoint(&mut self, job: &Job, function: &RuntimeFunction, states: Vec<State>) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(PriorToSendingJob(job.clone()));

        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(FunctionStates((function.clone(), states)));
    }

    fn block_breakpoint(&mut self, block: &Block) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(BlockBreakpoint(block.clone()));
    }

    fn flow_unblock_breakpoint(&mut self, flow_id: usize) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(FlowUnblockBreakpoint(flow_id));
    }

    fn send_breakpoint(
        &mut self,
        source_function_name: &str,
        source_function_id: usize,
        output_route: &str,
        value: &Value,
        destination_id: usize,
        destination_name: &str,
        io_name: &str,
        input_number: usize,
    ) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DataBreakpoint(
                source_function_name.to_string(),
                source_function_id,
                output_route.to_string(),
                value.clone(),
                destination_id,
                destination_name.to_string(),
                io_name.to_string(),
                input_number,
            ));
    }

    fn job_error(&mut self, job: &Job) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(JobError(job.clone()));
    }

    fn job_completed(&mut self, job: &Job) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(JobCompleted(job.clone()));
    }

    fn blocks(&mut self, blocks: Vec<Block>) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(BlockState(blocks));
    }

    fn outputs(&mut self, output_connections: Vec<OutputConnection>) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(OutputState(output_connections));
    }

    fn input(&mut self, input: Input) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(InputState(input));
    }

    fn function_list(&mut self, functions: &[RuntimeFunction]) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Functions(functions.to_vec()));
    }

    fn function_states(&mut self, function: RuntimeFunction, function_states: Vec<State>) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(FunctionStates((function, function_states)));
    }

    fn run_state(&mut self, run_state: &RunState) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(OverallState(run_state.clone()));
    }

    fn message(&mut self, message: String) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Message(message));
    }

    fn panic(&mut self, state: &RunState, error_message: String) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Panic(error_message, state.get_number_of_jobs_created()));
    }

    fn debugger_exiting(&mut self) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(ExitingDebugger);
    }

    fn debugger_resetting(&mut self) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Resetting);
    }

    fn debugger_error(&mut self, error_message: String) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Error(error_message));
    }

    fn execution_starting(&mut self) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(ExecutionStarted);
    }

    fn execution_ended(&mut self) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(ExecutionEnded);
    }

    fn get_command(&mut self, state: &RunState) -> flowcore::errors::Result<DebugCommand> {
        self.debug_server_connection
            .send_and_receive_response(WaitingForCommand(state.get_number_of_jobs_created()))
    }
}
