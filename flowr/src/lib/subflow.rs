//! Sub-flow execution support.
//!
//! Provides the ability to execute an extracted sub-flow independently
//! by creating a nested coordinator with its own dispatcher and executor
//! threads.

use std::sync::{Arc, Mutex};

use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowcore::{Implementation, RunAgain, RUN_AGAIN};
use log::{error, info};
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
/// 1. Clones the manifest and injects input values as flow initializers
/// 2. Creates a Dispatcher with ZMQ sockets on random ports
/// 3. Starts executor threads to process jobs
/// 4. Creates a Coordinator and calls `execute_subflow()`
/// 5. Returns boundary outputs (values destined for the parent flow)
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

        let result = coordinator.execute_subflow(submission);

        // Always signal executors to stop, even on error
        if let Err(e) = coordinator.send_done() {
            error!("Failed to send DONE to sub-flow executors: {e}");
        }
        drop(executor);

        let mut state = result?;

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

/// An `Implementation` that executes a sub-flow on a remote coordinator.
///
/// When `run()` is called, it connects to the coordinator via `ClientConnection`,
/// sends the sub-flow as a `ClientSubmission` (with `is_subflow = true`), and
/// receives boundary outputs in the `FlowEnd` response.
pub(crate) struct RemoteSubFlowImplementation {
    manifest: FlowManifest,
    interface_inputs: Vec<InterfaceInput>,
    peer_address: String,
    /// Optional `ContextIO` for relaying context function requests from the
    /// peer back to the origin coordinator's client.
    context_io: Option<crate::context_io::ContextIO>,
    /// Cached connection to the peer coordinator, reused across invocations.
    /// The peer's bridge loops back after `FlowEnd`, ready for the next submission.
    cached_connection: Mutex<Option<crate::connections::ClientConnection>>,
}

impl RemoteSubFlowImplementation {
    /// Create a new remote sub-flow implementation.
    #[must_use]
    pub(crate) fn new(
        manifest: FlowManifest,
        interface_inputs: Vec<InterfaceInput>,
        peer_address: String,
        context_io: Option<crate::context_io::ContextIO>,
    ) -> Self {
        RemoteSubFlowImplementation {
            manifest,
            interface_inputs,
            peer_address,
            context_io,
            cached_connection: Mutex::new(None),
        }
    }

    /// Build a `Submission` from the manifest and input values, with inputs
    /// injected as `Once` initializers on the boundary functions.
    fn build_submission(&self, inputs: &[Value]) -> Submission {
        let mut manifest = self.manifest.clone();
        for (i, iface) in self.interface_inputs.iter().enumerate() {
            if let Some(value) = inputs.get(i) {
                if let Some(func) = manifest.get_functions().get_mut(&iface.destination_id) {
                    func.set_flow_initializer(
                        iface.destination_io_number,
                        flowcore::model::input::InputInitializer::Once(value.clone()),
                    );
                }
            }
        }
        let mut submission = Submission::new(
            manifest,
            None,
            None,
            #[cfg(feature = "debugger")]
            false,
            #[cfg(feature = "trace")]
            None,
        );
        submission.is_subflow = true;
        submission
    }

    /// Submit to a remote coordinator and return boundary outputs.
    ///
    /// If the sub-flow contains context functions, the peer sends context
    /// requests as intermediate messages before `FlowEnd`. This method
    /// relays them through the origin's `ContextIO` and sends the responses
    /// back to the peer.
    fn submit_to_peer(
        &self,
        submission: Submission,
    ) -> flowcore::errors::Result<Vec<crate::client_protocol::BoundaryOutputEntry>> {
        use crate::client_protocol::{ClientMessage, CoordinatorMessage};
        use crate::connections::ClientConnection;

        // Reuse the cached connection if available, otherwise create a new one.
        // The peer's bridge loops back after FlowEnd, ready for the next submission.
        let connection = {
            let mut cached = self
                .cached_connection
                .lock()
                .map_err(|_| "Could not lock cached connection")?;
            if let Some(conn) = cached.take() {
                conn
            } else {
                let conn = ClientConnection::new(&self.peer_address).map_err(|e| {
                    format!("Could not connect to peer at {}: {e}", self.peer_address)
                })?;
                conn.set_receive_timeout(300_000)
                    .map_err(|e| format!("Could not set receive timeout: {e}"))?;
                conn
            }
        };

        connection
            .send(ClientMessage::ClientSubmission(Box::new(submission)))
            .map_err(|e| format!("Could not send submission to peer: {e}"))?;

        // Loop receiving messages from the peer. Context function requests
        // are relayed through ContextIO; FlowEnd terminates the loop.
        loop {
            let response: CoordinatorMessage = connection
                .receive()
                .map_err(|e| format!("Could not receive from peer: {e}"))?;

            match response {
                #[cfg(feature = "metrics")]
                CoordinatorMessage::FlowEnd(boundary_outputs, _) => {
                    // Cache the connection for the next invocation
                    if let Ok(mut cached) = self.cached_connection.lock() {
                        *cached = Some(connection);
                    }
                    return Ok(boundary_outputs);
                }
                #[cfg(not(feature = "metrics"))]
                CoordinatorMessage::FlowEnd(boundary_outputs) => {
                    if let Ok(mut cached) = self.cached_connection.lock() {
                        *cached = Some(connection);
                    }
                    return Ok(boundary_outputs);
                }
                CoordinatorMessage::CoordinatorExiting(result) => {
                    // Don't cache — the peer exited
                    return Err(format!("Peer coordinator exited: {result:?}").into());
                }
                context_msg => {
                    // Relay the context request through our ContextIO.
                    // GetStdin/GetLine use the blocking channel since they
                    // may block waiting for user input.
                    let client_response = if let Some(ref cio) = self.context_io {
                        let result = match &context_msg {
                            CoordinatorMessage::GetStdin | CoordinatorMessage::GetLine(_) => {
                                cio.send_and_receive_blocking(context_msg)
                            }
                            _ => cio.send_and_receive(context_msg),
                        };
                        result.unwrap_or(ClientMessage::Error("ContextIO relay failed".into()))
                    } else {
                        return Err("Context function in delegated sub-flow but no ContextIO \
                             configured — cannot proxy"
                            .into());
                    };
                    // Send the client's response back to the peer
                    connection
                        .send(client_response)
                        .map_err(|e| format!("Could not send context response to peer: {e}"))?;
                }
            }
        }
    }
}

impl RemoteSubFlowImplementation {
    /// Execute the sub-flow on the remote coordinator, sending each boundary
    /// output back to the parent coordinator's results socket individually.
    ///
    /// Each boundary output is sent as a separate result with `RUN_AGAIN`.
    /// A final result with `DONT_RUN_AGAIN` signals the proxy job is complete.
    ///
    /// # Errors
    ///
    /// Returns an error if the peer connection or sub-flow execution fails.
    pub(crate) fn run_streaming(
        &self,
        inputs: &[Value],
        job_id: usize,
        executor_id: &str,
        results_sink: &zmq::Socket,
    ) -> flowcore::errors::Result<()> {
        info!(
            "RemoteSubFlowImplementation: submitting sub-flow with {} inputs to peer at {}",
            inputs.len(),
            self.peer_address
        );

        let submission = self.build_submission(inputs);

        let submit_result = self.submit_to_peer(submission);

        match submit_result {
            Ok(boundary_outputs) => {
                let output_count = boundary_outputs.len();
                for bo in &boundary_outputs {
                    let bo_value = serde_json::json!({
                        "destination_id": bo.connection.destination_id,
                        "destination_io_number": bo.connection.destination_io_number,
                        "value": bo.value,
                    });
                    let result: flowcore::errors::Result<(Option<Value>, flowcore::RunAgain)> =
                        Ok((Some(Value::Array(vec![bo_value])), flowcore::RUN_AGAIN));
                    let msg = serde_json::to_string(&(job_id, executor_id, result))
                        .map_err(|e| format!("Could not serialize boundary output: {e}"))?;
                    results_sink
                        .send(msg.as_bytes(), 0)
                        .map_err(|e| format!("Could not send boundary output result: {e}"))?;
                }

                info!(
                    "RemoteSubFlowImplementation: sent {output_count} boundary outputs from peer"
                );

                // Send final "complete" result
                let final_result: flowcore::errors::Result<(Option<Value>, flowcore::RunAgain)> =
                    Ok((None, flowcore::DONT_RUN_AGAIN));
                let msg = serde_json::to_string(&(job_id, executor_id, final_result))
                    .map_err(|e| format!("Could not serialize final result: {e}"))?;
                results_sink
                    .send(msg.as_bytes(), 0)
                    .map_err(|e| format!("Could not send final result: {e}"))?;

                Ok(())
            }
            Err(e) => {
                error!("Peer sub-flow execution failed: {e}");
                let err_result: flowcore::errors::Result<(Option<Value>, flowcore::RunAgain)> =
                    Err(format!("Peer sub-flow failed: {e}").into());
                if let Ok(msg) = serde_json::to_string(&(job_id, executor_id, err_result)) {
                    let _ = results_sink.send(msg.as_bytes(), 0);
                }
                Err(e)
            }
        }
    }
}

impl Implementation for RemoteSubFlowImplementation {
    fn run(&self, inputs: &[Value]) -> flowcore::errors::Result<(Option<Value>, RunAgain)> {
        info!(
            "RemoteSubFlowImplementation: submitting sub-flow with {} inputs to peer at {}",
            inputs.len(),
            self.peer_address
        );

        let submission = self.build_submission(inputs);
        let boundary_outputs = self
            .submit_to_peer(submission)
            .map_err(|e| format!("Peer sub-flow execution failed: {e}"))?;

        info!(
            "RemoteSubFlowImplementation: received {} boundary outputs from peer",
            boundary_outputs.len()
        );

        if boundary_outputs.is_empty() {
            Ok((None, RUN_AGAIN))
        } else {
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
/// No-op submission handler for sub-flow execution (used by `SubFlowImplementation`).
pub struct NoOpSubmissionHandler;

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
/// No-op debugger handler for sub-flow and peer coordinator execution.
pub struct NoOpDebugHandler;

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
