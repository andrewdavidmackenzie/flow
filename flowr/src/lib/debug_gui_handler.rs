//! Channel-based [`DebuggerHandler`] implementation for GUI debugger integration.
//!
//! Routes debug events through `std::sync::mpsc` channels to the iced GUI,
//! and receives commands back from UI buttons. Unlike the ZMQ-based handler,
//! this runs in-process without network communication.

use std::sync::mpsc;

use serde_json::Value;

use flowcore::model::input::Input;
use flowcore::model::output_connection::OutputConnection;
use flowcore::model::runtime_function::RuntimeFunction;

use crate::debug_command::DebugCommand;
use crate::debug_server_message::DebugServerMessage;
use crate::debugger_handler::DebuggerHandler;
use crate::job::Job;
use crate::run_state::{RunState, State};

/// A debug handler that routes events to the GUI via channels
pub struct DebugGuiHandler {
    event_sender: mpsc::Sender<DebugServerMessage>,
    command_receiver: mpsc::Receiver<DebugCommand>,
}

impl DebugGuiHandler {
    /// Create a new GUI debug handler with the given channels
    #[must_use]
    pub fn new(
        event_sender: mpsc::Sender<DebugServerMessage>,
        command_receiver: mpsc::Receiver<DebugCommand>,
    ) -> Self {
        DebugGuiHandler {
            event_sender,
            command_receiver,
        }
    }

    fn send_event(&self, event: DebugServerMessage) {
        let _ = self.event_sender.send(event);
    }
}

impl DebuggerHandler for DebugGuiHandler {
    fn start(&mut self) {
        self.send_event(DebugServerMessage::EnteringDebugger);
    }

    fn job_breakpoint(&mut self, job: &Job, _function: &RuntimeFunction, _states: Vec<State>) {
        self.send_event(DebugServerMessage::PriorToSendingJob(job.clone()));
    }

    fn flow_unblock_breakpoint(&mut self, flow_id: usize) {
        self.send_event(DebugServerMessage::FlowUnblockBreakpoint(flow_id));
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
        self.send_event(DebugServerMessage::DataBreakpoint(
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
        self.send_event(DebugServerMessage::JobError(job.clone()));
    }

    fn job_completed(&mut self, job: &Job) {
        self.send_event(DebugServerMessage::JobCompleted(job.clone()));
    }

    fn outputs(&mut self, output_connections: Vec<OutputConnection>) {
        self.send_event(DebugServerMessage::OutputState(output_connections));
    }

    fn input(&mut self, input: Input) {
        self.send_event(DebugServerMessage::InputState(input));
    }

    fn function_list(&mut self, functions: &[RuntimeFunction]) {
        self.send_event(DebugServerMessage::Functions(functions.to_vec()));
    }

    fn function_states(
        &mut self,
        function: RuntimeFunction,
        function_states: Vec<State>,
        input_blockers: Vec<usize>,
    ) {
        self.send_event(DebugServerMessage::FunctionStates((
            function,
            function_states,
            input_blockers,
        )));
    }

    fn run_state(&mut self, run_state: &RunState) {
        self.send_event(DebugServerMessage::OverallState(run_state.clone()));
    }

    fn message(&mut self, message: String) {
        self.send_event(DebugServerMessage::Message(message));
    }

    fn breakpoint_list(&mut self, breakpoints: Vec<crate::debug_command::BreakpointSpec>) {
        self.send_event(DebugServerMessage::BreakpointList(breakpoints));
    }

    fn process_tree(&mut self, state: &RunState) {
        self.send_event(DebugServerMessage::ProcessTree(state.clone()));
    }

    fn inspect_by_state(&mut self, state_name: &str, state: &RunState) {
        self.send_event(DebugServerMessage::InspectByState(
            state_name.to_string(),
            state.clone(),
        ));
    }

    fn inspect_function(&mut self, function_id: usize, state: &RunState) {
        self.send_event(DebugServerMessage::InspectFunction(
            function_id,
            state.clone(),
        ));
    }

    fn inspect_flow(&mut self, flow_id: usize, state: &RunState) {
        self.send_event(DebugServerMessage::InspectFlow(flow_id, state.clone()));
    }

    fn job_inspect(&mut self, job: Job) {
        self.send_event(DebugServerMessage::JobInspect(job));
    }

    #[cfg(feature = "metrics")]
    fn execution_metrics(&mut self, metrics: flowcore::model::metrics::Metrics) {
        self.send_event(DebugServerMessage::ExecutionMetrics(metrics));
    }

    fn flow_list(&mut self, _flow_ids: &[usize], state: &RunState) {
        let manifest = &state.get_submission().manifest;
        let flows: Vec<(usize, String, String, Option<usize>)> = manifest
            .flows()
            .values()
            .map(|fi| {
                (
                    fi.process_id,
                    fi.name.clone(),
                    fi.route.clone(),
                    fi.parent_id,
                )
            })
            .collect();
        self.send_event(DebugServerMessage::FlowList(flows));
    }

    fn panic(&mut self, state: &RunState, error_message: String) {
        self.send_event(DebugServerMessage::Panic(
            error_message,
            state.get_number_of_jobs_created(),
        ));
    }

    fn debugger_exiting(&mut self) {
        self.send_event(DebugServerMessage::ExitingDebugger);
    }

    fn debugger_resetting(&mut self) {
        self.send_event(DebugServerMessage::Resetting);
    }

    fn debugger_error(&mut self, error_message: String) {
        self.send_event(DebugServerMessage::Error(error_message));
    }

    fn execution_starting(&mut self) {
        self.send_event(DebugServerMessage::ExecutionStarted);
    }

    fn execution_ended(&mut self) {
        self.send_event(DebugServerMessage::ExecutionEnded);
    }

    fn get_command(&mut self, state: &RunState) -> flowcore::errors::Result<DebugCommand> {
        self.send_event(DebugServerMessage::WaitingForCommand(
            state.get_number_of_jobs_created(),
        ));
        self.command_receiver
            .recv()
            .map_err(|e| format!("Debug GUI command channel closed: {e}").into())
    }
}
