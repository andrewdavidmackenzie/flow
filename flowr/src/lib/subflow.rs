//! Sub-flow execution support.
//!
//! Provides the ability to execute an extracted sub-flow independently
//! by creating a nested coordinator with its own dispatcher and executor
//! threads.

use std::sync::Arc;

use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use log::info;
use serde_json::Value;
use url::Url;

use crate::coordinator::Coordinator;
use crate::dispatcher::Dispatcher;
use crate::executor::Executor;

#[cfg(feature = "debugger")]
use crate::debugger_handler::DebuggerHandler;
#[cfg(feature = "submission")]
use crate::submission_handler::SubmissionHandler;

/// Describes one input to a sub-flow's external interface.
#[derive(Clone, Debug)]
pub struct InterfaceInput {
    /// The function inside the sub-flow that receives this input
    pub destination_id: usize,
    /// Which input number on the destination function
    pub destination_io_number: usize,
}

/// An `Implementation` that executes a sub-flow by running a nested coordinator.
///
/// When `run()` is called, it:
/// 1. Clones the manifest and injects input values as initializers
/// 2. Creates a Dispatcher with ZMQ sockets on random ports
/// 3. Starts executor threads to process jobs
/// 4. Creates a Coordinator and calls `execute_flow()`
/// 5. Returns `(None, RUN_AGAIN)` on success
pub struct SubFlowImplementation {
    manifest: FlowManifest,
    provider: Arc<dyn Provider>,
    /// Mapping from input index (position in `input_set`) to the boundary
    /// function input where the value should be injected.
    interface_inputs: Vec<InterfaceInput>,
}

impl SubFlowImplementation {
    /// Create a new sub-flow implementation from an extracted manifest
    /// and its interface input mapping.
    #[must_use]
    pub fn new(
        manifest: FlowManifest,
        provider: Arc<dyn Provider>,
        interface_inputs: Vec<InterfaceInput>,
    ) -> Self {
        SubFlowImplementation {
            manifest,
            provider,
            interface_inputs,
        }
    }
}

impl Implementation for SubFlowImplementation {
    fn run(&self, inputs: &[Value]) -> flowcore::errors::Result<(Option<Value>, RunAgain)> {
        info!(
            "SubFlowImplementation: running sub-flow with {} inputs",
            inputs.len()
        );

        // Clone the manifest and inject input values as Once initializers
        // on the boundary function inputs
        let mut manifest = self.manifest.clone();
        for (i, iface_input) in self.interface_inputs.iter().enumerate() {
            if let Some(value) = inputs.get(i) {
                if let Some(func) = manifest
                    .get_functions()
                    .get_mut(&iface_input.destination_id)
                {
                    func.set_flow_initializer(
                        iface_input.destination_io_number,
                        flowcore::model::input::InputInitializer::Once(value.clone()),
                    );
                }
            }
        }

        let submission = Submission::new(
            manifest,
            None,
            None,
            #[cfg(feature = "debugger")]
            false,
            #[cfg(feature = "trace")]
            None,
        );

        // Set up the dispatcher and executor for this sub-flow
        let ports = get_four_ports()?;
        let bind_addrs = get_bind_addresses(ports);
        let dispatcher = Dispatcher::new(&bind_addrs)?;
        let connect_addrs = get_connect_addresses(ports);

        let mut executor = Executor::new();
        #[cfg(feature = "flowstdlib")]
        executor.add_lib(
            flowstdlib::manifest::get()
                .map_err(|e| format!("Could not get flowstdlib manifest: {e}"))?,
            Url::parse("memory://").map_err(|e| format!("Could not parse memory URL: {e}"))?,
        )?;

        executor.start(
            &self.provider,
            1, // single executor thread for sub-flows
            &connect_addrs.0,
            &connect_addrs.2,
            &connect_addrs.3,
        );

        // Give executor threads time to connect to ZMQ sockets
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Create no-op handlers for the sub-flow coordinator
        #[cfg(feature = "submission")]
        let mut sub_handler = NoOpSubmissionHandler;
        #[cfg(feature = "debugger")]
        let mut sub_debugger = NoOpDebugHandler;

        let mut coordinator = Coordinator::new(
            dispatcher,
            #[cfg(feature = "submission")]
            &mut sub_handler,
            #[cfg(feature = "debugger")]
            &mut sub_debugger,
        );

        let mut state = coordinator.execute_subflow(submission)?;

        // Signal executors to stop
        let _ = coordinator.send_done();
        drop(executor);

        // Collect boundary outputs — values destined for parent flow functions
        let boundary_outputs = state.drain_boundary_outputs();
        if boundary_outputs.is_empty() {
            Ok((None, RUN_AGAIN))
        } else {
            // Package boundary outputs as a JSON array for the parent
            let outputs: Vec<Value> = boundary_outputs
                .into_iter()
                .map(|bo| {
                    serde_json::json!({
                        "destination_id": bo.connection.destination_id,
                        "destination_io_number": bo.connection.destination_io_number,
                        "value": bo.value,
                    })
                })
                .collect();
            Ok((Some(Value::Array(outputs)), RUN_AGAIN))
        }
    }
}

// --- No-op handlers for sub-flow coordinator ---

#[cfg(feature = "submission")]
struct NoOpSubmissionHandler;

#[cfg(feature = "submission")]
impl SubmissionHandler for NoOpSubmissionHandler {
    fn flow_execution_starting(&mut self) -> flowcore::errors::Result<()> {
        Ok(())
    }

    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> flowcore::errors::Result<bool> {
        Ok(false)
    }

    fn flow_execution_ended(
        &mut self,
        _state: &crate::run_state::RunState,
        #[cfg(feature = "metrics")] _metrics: flowcore::model::metrics::Metrics,
    ) -> flowcore::errors::Result<()> {
        Ok(())
    }

    fn wait_for_submission(
        &mut self,
    ) -> flowcore::errors::Result<Option<flowcore::model::submission::Submission>> {
        Ok(None)
    }

    fn coordinator_is_exiting(
        &mut self,
        _result: flowcore::errors::Result<()>,
    ) -> flowcore::errors::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "debugger")]
struct NoOpDebugHandler;

#[cfg(feature = "debugger")]
impl DebuggerHandler for NoOpDebugHandler {
    fn start(&mut self) {}
    fn job_breakpoint(
        &mut self,
        _job: &crate::job::Job,
        _function: &flowcore::model::runtime_function::RuntimeFunction,
        _states: Vec<crate::run_state::State>,
    ) {
    }
    fn flow_unblock_breakpoint(&mut self, _flow_id: usize) {}
    fn send_breakpoint(
        &mut self,
        _: &str,
        _: usize,
        _: &str,
        _: &Value,
        _: usize,
        _: &str,
        _: &str,
        _: usize,
    ) {
    }
    fn job_error(&mut self, _: &crate::job::Job) {}
    fn job_completed(&mut self, _: &crate::job::Job) {}
    fn outputs(&mut self, _: Vec<flowcore::model::output_connection::OutputConnection>) {}
    fn input(&mut self, _: flowcore::model::input::Input) {}
    fn function_list(&mut self, _: &[flowcore::model::runtime_function::RuntimeFunction]) {}
    fn function_states(
        &mut self,
        _: flowcore::model::runtime_function::RuntimeFunction,
        _: Vec<crate::run_state::State>,
        _: Vec<usize>,
    ) {
    }
    fn inspect_function(&mut self, _: usize, _: &crate::run_state::RunState) {}
    fn run_state(&mut self, _: &crate::run_state::RunState) {}
    fn message(&mut self, _: String) {}
    fn breakpoint_list(&mut self, _: Vec<crate::debug_command::BreakpointSpec>) {}
    fn panic(&mut self, _: &crate::run_state::RunState, _: String) {}
    fn debugger_exiting(&mut self) {}
    fn debugger_resetting(&mut self) {}
    fn debugger_error(&mut self, _: String) {}
    fn execution_starting(&mut self) {}
    fn execution_ended(&mut self) {}
    fn process_tree(&mut self, _: &crate::run_state::RunState) {}
    fn inspect_by_state(&mut self, _: &str, _: &crate::run_state::RunState) {}
    fn inspect_flow(&mut self, _: usize, _: &crate::run_state::RunState) {}
    fn job_inspect(&mut self, _: crate::job::Job) {}
    #[cfg(feature = "metrics")]
    fn execution_metrics(&mut self, _: flowcore::model::metrics::Metrics) {}
    fn flow_list(&mut self, _: &[usize], _: &crate::run_state::RunState) {}
    fn get_command(
        &mut self,
        _: &crate::run_state::RunState,
    ) -> flowcore::errors::Result<flowcore::model::debug_command::DebugCommand> {
        Ok(flowcore::model::debug_command::DebugCommand::Continue)
    }
}

// --- Port helpers (duplicated from flowrcli, should be shared later) ---

fn get_four_ports() -> flowcore::errors::Result<(u16, u16, u16, u16)> {
    Ok((
        portpicker::pick_unused_port().ok_or("No ports free")?,
        portpicker::pick_unused_port().ok_or("No ports free")?,
        portpicker::pick_unused_port().ok_or("No ports free")?,
        portpicker::pick_unused_port().ok_or("No ports free")?,
    ))
}

fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://*:{}", ports.0),
        format!("tcp://*:{}", ports.1),
        format!("tcp://*:{}", ports.2),
        format!("tcp://*:{}", ports.3),
    )
}

fn get_connect_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://127.0.0.1:{}", ports.0),
        format!("tcp://127.0.0.1:{}", ports.1),
        format!("tcp://127.0.0.1:{}", ports.2),
        format!("tcp://127.0.0.1:{}", ports.3),
    )
}
