//! ZMQ-based [`DebuggerHandler`] implementation that forwards debug events
//! between the coordinator and an external debug client over a [`CoordinatorConnection`].
//!
//! Used by `flowrcli` and `flowrgui` when debugging with an external `flowrdb` client.
//! In the future, `flowrgui` may use a different handler that routes to its own UI.

use serde_json::Value;

use flowcore::model::input::Input;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;

use crate::connections::{CoordinatorConnection, WAIT};
use crate::debug_command::{BreakpointSpec, DebugCommand};
use crate::debug_server_message::DebugServerMessage;
use crate::debug_server_message::DebugServerMessage::{
    DataBreakpoint, EnteringDebugger, Error, ExecutionEnded, ExecutionStarted, ExitingDebugger,
    FlowUnblockBreakpoint, FunctionStates, Functions, InputState, JobCompleted, JobError, Message,
    OutputState, OverallState, Panic, PriorToSendingJob, Resetting, WaitingForCommand,
};
use crate::debugger_handler::DebuggerHandler;
use crate::job::Job;
use crate::run_state::{RunState, State};

/// A debug handler that bridges the coordinator's debugger to a debug client over ZMQ
pub struct DebugZmqHandler {
    /// The ZMQ connection to the debug client
    pub debug_server_connection: CoordinatorConnection,
}

impl DebuggerHandler for DebugZmqHandler {
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

    fn flow_list(&mut self, flow_ids: &[usize], state: &RunState) {
        let functions = state.get_functions();
        let flows: Vec<(usize, String, String)> = flow_ids
            .iter()
            .map(|id| {
                if let Some(f) = functions.get(id) {
                    (*id, f.name().to_string(), f.route().to_string())
                } else {
                    (*id, String::new(), String::new())
                }
            })
            .collect();
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DebugServerMessage::FlowList(flows));
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

    fn breakpoint_list(&mut self, breakpoints: Vec<BreakpointSpec>) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DebugServerMessage::BreakpointList(breakpoints));
    }

    fn process_tree(&mut self, state: &RunState) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DebugServerMessage::ProcessTree(state.clone()));
    }

    fn inspect_by_state(&mut self, state_name: &str, state: &RunState) {
        let _: flowcore::errors::Result<DebugCommand> =
            self.debug_server_connection.send_and_receive_response(
                DebugServerMessage::InspectByState(state_name.to_string(), state.clone()),
            );
    }

    fn inspect_flow(&mut self, flow_id: usize, state: &RunState) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DebugServerMessage::InspectFlow(flow_id, state.clone()));
    }

    fn job_inspect(&mut self, job: Job) {
        let _: flowcore::errors::Result<DebugCommand> = self
            .debug_server_connection
            .send_and_receive_response(DebugServerMessage::JobInspect(job));
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
