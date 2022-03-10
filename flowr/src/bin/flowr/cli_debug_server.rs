use serde_json::Value;

use flowcore::model::input::Input;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::block::Block;
use flowrlib::debug_command::DebugCommand;
use flowrlib::job::Job;
use flowrlib::run_state::{RunState, State};
#[cfg(feature = "debugger")]
use flowrlib::server::DebugServer;

use crate::{BlockBreakpoint, DataBreakpoint, DebugServerMessage, ExecutionEnded, ExecutionStarted, ExitingDebugger, JobCompleted, JobError, Panic, PriorToSendingJob, Resetting, SendingValue, ServerConnection, WAIT, WaitingForCommand};

pub(crate) struct  CliDebugServer {
    pub(crate) debug_server_connection: ServerConnection,
}

/// Implement a CLI debug server that implements the trait required by the runtime
impl DebugServer for CliDebugServer {
    // Start the debugger - which swallows the first message to initialize the connection
    fn start(&mut self) {
        let _ = self.debug_server_connection.receive::<DebugCommand>(WAIT);
    }

    // a breakpoint has been hit on a Job being created
    fn job_breakpoint(&mut self, next_job_id: usize, function: &RuntimeFunction, state: State) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(PriorToSendingJob(next_job_id, function.id()));

        // display the status of the function we stopped prior to creating a job for
        let event = DebugServerMessage::FunctionState((function.clone(), state));
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(event);
    }

    // A breakpoint set on creation of a `Block` matching `block` has been hit
    fn block_breakpoint(&mut self, block: &Block) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(BlockBreakpoint(block.clone()));
    }

    // A breakpoint on sending a value from a specific function or to a specific function was hit
    fn send_breakpoint(&mut self, source_process_id: usize, output_route: &str, value: &Value,
                       destination_id: usize, input_number: usize) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(SendingValue(
                source_process_id,
                value.clone(),
                destination_id,
                input_number,
            ));
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DataBreakpoint(
                source_process_id,
                output_route.to_string(),
                value.clone(),
                destination_id,
                input_number,
            ));
    }

    // A job error occurred during execution of the flow
    fn job_error(&mut self, job: &Job) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(JobError(job.clone()));
    }

    // A specific job completed
    fn job_completed(&mut self, job: &Job) {
        let _: flowcore::errors::Result<DebugCommand> =
            self.debug_server_connection
                .send_and_receive_response(JobCompleted(job.clone()));
    }

    // returns a set of blocks
    fn blocks(&mut self, blocks: Vec<Block>) {
        let _: flowcore::errors::Result<DebugCommand> =
            self.debug_server_connection
                .send_and_receive_response(DebugServerMessage::BlockState(blocks));
    }

    // returns an output's connections
    fn outputs(&mut self, output_connections: Vec<OutputConnection>) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DebugServerMessage::OutputState(output_connections));
    }

    // returns an inputs state
    fn input(&mut self, input: Input) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DebugServerMessage::InputState(input));
    }

    // returns the state of a function
    fn function_state(&mut self,  function: RuntimeFunction, function_state: State) {
        let message = DebugServerMessage::FunctionState((function, function_state));
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(message);
    }

    // returns the global run state
    fn run_state(&mut self, run_state: RunState) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DebugServerMessage::OverallState(run_state));
    }

    // a string message from the Debugger
    fn message(&mut self, message: String) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(message);
    }

    // a panic occurred during execution
    fn panic(&mut self, state: &RunState, error_message: String) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Panic(error_message, state.jobs_created()));
    }

    // the debugger is exiting
    fn debugger_exiting(&mut self) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(ExitingDebugger);
    }

    // The debugger is resetting the runtime state
    fn debugger_resetting(&mut self) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Resetting);
    }

    // An error occurred in the debugger
    fn debugger_error(&mut self, error_message: String) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection.send_and_receive_response(
            DebugServerMessage::Error(error_message),
        );
    }

    // execution of the flow is starting
    fn execution_starting(&mut self) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(ExecutionStarted);
    }

    // Execution of the flow fn execution_ended(&mut self, state: &RunState) {
    fn execution_ended(&mut self) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(ExecutionEnded);
    }

    // Get a command for the debugger to perform
    fn get_command(&mut self, state: &RunState) -> flowcore::errors::Result<DebugCommand> {
        self
            .debug_server_connection
            .send_and_receive_response(WaitingForCommand(state.jobs_created()))
    }
}