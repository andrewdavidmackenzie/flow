use serde_json::Value;

use flowcore::model::input::Input;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;
use flowrlib::block::Block;
use flowrlib::debug_command::DebugCommand;
use flowrlib::debugger_handler::DebuggerHandler;
use flowrlib::job::Job;
use flowrlib::run_state::{RunState, State};

use crate::{BlockBreakpoint, CoordinatorConnection, DataBreakpoint, ExecutionEnded, ExecutionStarted,
            ExitingDebugger, JobCompleted, JobError, Panic, PriorToSendingJob, Resetting,
            WaitingForCommand};
use crate::gui::connections::WAIT;
use crate::DebugServerMessage::{BlockState, Error, FlowUnblockBreakpoint, Functions, FunctionStates, InputState, Message, OutputState, OverallState};

/// A debug handler for interacting between the CLI client and the Debugger in the Coordinator
pub(crate) struct CliDebugHandler {
    pub(crate) debug_server_connection: CoordinatorConnection,
}

/// Implement a CLI debug server that implements the trait required by the runtime
impl DebuggerHandler for CliDebugHandler {
    // Start the debugger - which swallows the first message to initialize the connection
    fn start(&mut self) {
        let _ = self.debug_server_connection.receive::<DebugCommand>(WAIT);
    }

    // a breakpoint has been hit on a Job being created
    fn job_breakpoint(&mut self, job: &Job, function: &RuntimeFunction, states: Vec<State>) {
        // Send a copy of the job about to be sent - for display
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(PriorToSendingJob(job.clone()));

        // display the status of the function we stopped prior to creating a job for
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(FunctionStates((function.clone(), states)));
    }

    // A breakpoint set on creation of a `Block` matching `block` has been hit
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

    // A breakpoint on sending a value from a specific function or to a specific function was hit
    fn send_breakpoint(&mut self, source_function_name: &str, source_function_id: usize,
                       output_route: &str, value: &Value, destination_id: usize,
                       destination_name: &str, io_name: &str, input_number: usize) {
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
                .send_and_receive_response(BlockState(blocks));
    }

    // returns an output's connections
    fn outputs(&mut self, output_connections: Vec<OutputConnection>) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(OutputState(output_connections));
    }

    // returns an inputs state
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

    // returns the state of a function
    fn function_states(&mut self,  function: RuntimeFunction, function_states: Vec<State>) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(FunctionStates((function, function_states)));
    }

    // returns the global run state
    fn run_state(&mut self, run_state: &RunState) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(OverallState(run_state.clone()));
    }

    // a string message from the Debugger
    fn message(&mut self, message: String) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Message(message));
    }

    // a panic occurred during execution
    fn panic(&mut self, state: &RunState, error_message: String) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(Panic(error_message, state.get_number_of_jobs_created()));
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
            .debug_server_connection.send_and_receive_response(Error(error_message));
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
            .send_and_receive_response(WaitingForCommand(state.get_number_of_jobs_created()))
    }
}