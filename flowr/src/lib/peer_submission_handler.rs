//! A [`SubmissionHandler`] for receiving sub-flow submissions from a parent
//! coordinator (peer-to-peer delegation).
//!
//! This handler receives a sub-flow manifest via a ZMQ REP socket, runs it
//! through the coordinator's normal `execute_flow`, and relays boundary
//! outputs back to the parent.

use flowcore::errors::Result;
#[cfg(feature = "metrics")]
use flowcore::model::metrics::Metrics;
use flowcore::model::submission::Submission;
use log::{debug, info};

use crate::peer_protocol::{PeerRequest, PeerResponse};
use crate::run_state::{BoundaryOutput, RunState};
use crate::submission_handler::SubmissionHandler;

/// A [`SubmissionHandler`] that receives sub-flow submissions from a parent
/// coordinator via a ZMQ REP socket.
pub struct PeerSubmissionHandler {
    /// ZMQ REP socket for receiving sub-flow submissions
    socket: zmq::Socket,
}

impl PeerSubmissionHandler {
    /// Create a new peer submission handler bound to the given address.
    ///
    /// # Errors
    ///
    /// Returns an error if the ZMQ socket cannot be created or bound.
    pub fn new(context: &zmq::Context, bind_address: &str) -> Result<Self> {
        let socket = context
            .socket(zmq::REP)
            .map_err(|e| format!("Could not create peer REP socket: {e}"))?;
        socket
            .bind(bind_address)
            .map_err(|e| format!("Could not bind peer socket to {bind_address}: {e}"))?;
        info!("Peer coordinator listening on {bind_address}");
        Ok(PeerSubmissionHandler { socket })
    }

    /// Send a response back to the parent coordinator.
    fn send_response(&self, response: &PeerResponse) -> Result<()> {
        let json = serde_json::to_string(response)
            .map_err(|e| format!("Could not serialize peer response: {e}"))?;
        self.socket
            .send(json.as_bytes(), 0)
            .map_err(|e| format!("Could not send peer response: {e}"))?;
        Ok(())
    }

    /// Send all boundary outputs to the parent coordinator in a single
    /// `Completed` message, avoiding per-output ZMQ round-trip overhead.
    /// This also signals that the sub-flow execution is complete (idle).
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be serialized or sent.
    fn send_completed(&self, outputs: &[BoundaryOutput]) -> Result<()> {
        let batch: Vec<crate::peer_protocol::BoundaryOutputEntry> = outputs
            .iter()
            .map(|o| crate::peer_protocol::BoundaryOutputEntry {
                connection: o.connection.clone(),
                value: o.value.clone(),
            })
            .collect();
        self.send_response(&PeerResponse::Completed(batch))
    }
}

impl SubmissionHandler for PeerSubmissionHandler {
    fn flow_execution_starting(&mut self) -> Result<()> {
        debug!("Peer coordinator: flow execution starting");
        Ok(())
    }

    #[cfg(feature = "debugger")]
    fn should_enter_debugger(&mut self) -> Result<bool> {
        Ok(false)
    }

    fn flow_execution_ended(
        &mut self,
        state: &RunState,
        #[cfg(feature = "metrics")] _metrics: Metrics,
    ) -> Result<()> {
        debug!("Peer coordinator: flow execution ended");

        // Send all boundary outputs + completion signal in a single message
        let outputs = state.boundary_outputs();
        info!(
            "Peer coordinator completed with {} boundary outputs",
            outputs.len()
        );
        self.send_completed(outputs)?;
        Ok(())
    }

    fn wait_for_submission(&mut self) -> Result<Option<Submission>> {
        info!("Peer coordinator waiting for sub-flow submission");

        let msg = self
            .socket
            .recv_msg(0)
            .map_err(|e| format!("Could not receive peer request: {e}"))?;
        let msg_str = msg
            .as_str()
            .ok_or("Could not convert peer message to string")?;

        let request: PeerRequest = serde_json::from_str(msg_str)
            .map_err(|e| format!("Could not deserialize peer request: {e}"))?;

        match request {
            PeerRequest::Submit(submission) => {
                let mut manifest = submission.manifest;
                let inputs = submission.inputs;
                if let Some(ref wasm_url) = submission.wasm_base_url {
                    info!("Parent WASM server at {wasm_url}");
                }
                info!(
                    "Peer coordinator received sub-flow with {} functions, {} inputs",
                    manifest.functions().len(),
                    inputs.len()
                );

                // Inject inputs as Once initializers on boundary functions
                for (dest_func_id, dest_io_number, value) in &inputs {
                    let func = manifest
                        .get_functions()
                        .get_mut(dest_func_id)
                        .ok_or_else(|| {
                            format!(
                                "Input destination function #{dest_func_id} not found in sub-flow"
                            )
                        })?;
                    func.set_flow_initializer(
                        *dest_io_number,
                        flowcore::model::input::InputInitializer::Once(value.clone()),
                    );
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

                Ok(Some(submission))
            }
            PeerRequest::Done => {
                info!("Peer coordinator received Done signal");
                self.send_response(&PeerResponse::DoneAck)?;
                Ok(None) // Exit submission loop
            }
        }
    }

    fn coordinator_is_exiting(&mut self, _result: Result<()>) -> Result<()> {
        debug!("Peer coordinator exiting");
        Ok(())
    }
}
