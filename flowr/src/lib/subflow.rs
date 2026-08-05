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

/// An `Implementation` that executes a sub-flow by running a nested coordinator.
///
/// When `run()` is called, it:
/// 1. Creates a Dispatcher with ZMQ sockets on random ports
/// 2. Starts executor threads to process jobs
/// 3. Creates a Coordinator and calls `execute_flow()`
/// 4. Returns `(None, RUN_AGAIN)` on success (sub-flow outputs flow
///    through the coordinator's normal connection routing)
pub struct SubFlowImplementation {
    manifest: FlowManifest,
    provider: Arc<dyn Provider>,
}

impl SubFlowImplementation {
    /// Create a new sub-flow implementation from an extracted manifest.
    #[must_use]
    pub fn new(manifest: FlowManifest, provider: Arc<dyn Provider>) -> Self {
        SubFlowImplementation { manifest, provider }
    }
}

impl Implementation for SubFlowImplementation {
    fn run(&self, inputs: &[Value]) -> flowcore::errors::Result<(Option<Value>, RunAgain)> {
        info!(
            "SubFlowImplementation: running sub-flow with {} inputs",
            inputs.len()
        );

        // TODO: inject inputs at the sub-flow's boundary functions
        // For now, just create the submission and run it

        let submission = Submission::new(
            self.manifest.clone(),
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

        coordinator.execute_flow(submission)?;

        // Signal executors to stop — don't wait, let threads exit naturally
        let _ = coordinator.send_done();
        drop(executor);

        Ok((None, RUN_AGAIN))
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
