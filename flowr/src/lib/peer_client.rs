//! Client for communicating with a peer coordinator.
//!
//! Used by a parent coordinator to delegate sub-flows to discovered peer
//! coordinators on the network.

use flowcore::errors::Result;
use flowcore::model::flow_manifest::FlowManifest;
use log::info;
use serde_json::Value;

use crate::peer_protocol::{PeerRequest, PeerResponse, SubflowSubmission};
use crate::run_state::BoundaryOutput;

/// A client connection to a peer coordinator for sub-flow delegation.
pub struct PeerClient {
    socket: zmq::Socket,
    address: String,
}

impl PeerClient {
    /// Connect to a peer coordinator at the given address.
    ///
    /// # Errors
    ///
    /// Returns an error if the ZMQ socket cannot be created, connected,
    /// or if the send/receive timeouts cannot be configured.
    pub fn connect(context: &zmq::Context, address: &str) -> Result<Self> {
        let socket = context
            .socket(zmq::REQ)
            .map_err(|e| format!("Could not create peer REQ socket: {e}"))?;
        let tcp_address = format!("tcp://{address}");
        socket
            .connect(&tcp_address)
            .map_err(|e| format!("Could not connect to peer coordinator at {tcp_address}: {e}"))?;
        // Set 30-second send and receive timeouts so we don't block
        // forever if the peer stops responding.
        socket
            .set_rcvtimeo(30_000)
            .map_err(|e| format!("Could not set receive timeout: {e}"))?;
        socket
            .set_sndtimeo(30_000)
            .map_err(|e| format!("Could not set send timeout: {e}"))?;
        info!("Connected to peer coordinator at {address}");
        Ok(PeerClient {
            socket,
            address: address.to_string(),
        })
    }

    /// Get the address of this peer coordinator.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Submit a sub-flow for execution on the peer coordinator.
    /// Returns the boundary outputs produced by the sub-flow.
    ///
    /// `wasm_base_url` is the base URL of the parent's WASM HTTP server,
    /// passed to the peer for potential further delegation.
    ///
    /// # Errors
    ///
    /// Returns an error if the submission cannot be sent or the response
    /// cannot be received/parsed.
    pub fn submit_subflow(
        &self,
        manifest: FlowManifest,
        inputs: Vec<(usize, usize, Value)>,
        wasm_base_url: Option<String>,
    ) -> Result<Vec<BoundaryOutput>> {
        let request = PeerRequest::Submit(Box::new(SubflowSubmission {
            manifest,
            inputs,
            wasm_base_url,
        }));
        let json = serde_json::to_string(&request)
            .map_err(|e| format!("Could not serialize peer request: {e}"))?;

        self.socket
            .send(json.as_bytes(), 0)
            .map_err(|e| format!("Could not send to peer: {e}"))?;

        // The Completed response arrives only after the whole sub-flow has
        // executed, so disable the receive timeout for this call. The
        // 30-second timeout set in connect() is kept for signal_done().
        self.socket
            .set_rcvtimeo(-1)
            .map_err(|e| format!("Could not clear receive timeout: {e}"))?;

        // Receive the single Completed response with all boundary outputs
        let msg = self
            .socket
            .recv_msg(0)
            .map_err(|e| format!("Could not receive from peer: {e}"))?;

        // Restore the 30-second timeout for subsequent operations
        self.socket
            .set_rcvtimeo(30_000)
            .map_err(|e| format!("Could not restore receive timeout: {e}"))?;

        let msg_str = msg
            .as_str()
            .ok_or("Could not convert peer response to string")?;
        let response: PeerResponse = serde_json::from_str(msg_str)
            .map_err(|e| format!("Could not deserialize peer response: {e}"))?;

        match response {
            PeerResponse::Completed(batch) => {
                info!(
                    "Peer sub-flow completed with {} boundary outputs",
                    batch.len()
                );
                Ok(batch
                    .into_iter()
                    .map(|entry| BoundaryOutput {
                        connection: entry.connection,
                        value: entry.value,
                    })
                    .collect())
            }
            PeerResponse::Error(msg) => Err(format!("Peer coordinator error: {msg}").into()),
            PeerResponse::DoneAck => {
                Err("Peer returned DoneAck in response to Submit; protocol out of step".into())
            }
        }
    }

    /// Submit a sub-flow for execution and invoke a callback for each
    /// boundary output. The peer executes the sub-flow and returns all
    /// boundary outputs in a single batched `Completed` message. The
    /// callback is invoked once per output from the batch.
    ///
    /// `wasm_base_url` is the base URL of the parent's WASM HTTP server,
    /// passed to the peer for potential further delegation.
    ///
    /// # Errors
    ///
    /// Returns an error if the submission fails, a response cannot be
    /// parsed, or the callback returns an error.
    pub fn submit_subflow_for_each<F>(
        &self,
        manifest: FlowManifest,
        inputs: Vec<(usize, usize, Value)>,
        wasm_base_url: Option<String>,
        mut on_boundary_output: F,
    ) -> Result<()>
    where
        F: FnMut(BoundaryOutput) -> Result<()>,
    {
        let request = PeerRequest::Submit(Box::new(SubflowSubmission {
            manifest,
            inputs,
            wasm_base_url,
        }));
        let json = serde_json::to_string(&request)
            .map_err(|e| format!("Could not serialize peer request: {e}"))?;

        self.socket
            .send(json.as_bytes(), 0)
            .map_err(|e| format!("Could not send to peer: {e}"))?;

        // The Completed response arrives only after the whole sub-flow has
        // executed, so disable the receive timeout for this call.
        self.socket
            .set_rcvtimeo(-1)
            .map_err(|e| format!("Could not clear receive timeout: {e}"))?;

        let msg = self
            .socket
            .recv_msg(0)
            .map_err(|e| format!("Could not receive from peer: {e}"))?;

        // Restore the 30-second timeout for subsequent operations
        self.socket
            .set_rcvtimeo(30_000)
            .map_err(|e| format!("Could not restore receive timeout: {e}"))?;

        let msg_str = msg
            .as_str()
            .ok_or("Could not convert peer response to string")?;
        let response: PeerResponse = serde_json::from_str(msg_str)
            .map_err(|e| format!("Could not deserialize peer response: {e}"))?;

        match response {
            PeerResponse::Completed(batch) => {
                for entry in batch {
                    on_boundary_output(BoundaryOutput {
                        connection: entry.connection,
                        value: entry.value,
                    })?;
                }
                Ok(())
            }
            PeerResponse::Error(msg) => Err(format!("Peer coordinator error: {msg}").into()),
            PeerResponse::DoneAck => {
                Err("Peer returned DoneAck in response to Submit; protocol out of step".into())
            }
        }
    }

    /// Signal the peer coordinator that the parent flow is done.
    ///
    /// # Errors
    ///
    /// Returns an error if the done signal cannot be sent.
    pub fn signal_done(&self) -> Result<()> {
        let request = PeerRequest::Done;
        let json = serde_json::to_string(&request)
            .map_err(|e| format!("Could not serialize done request: {e}"))?;
        self.socket
            .send(json.as_bytes(), 0)
            .map_err(|e| format!("Could not send done to peer: {e}"))?;

        // Wait for ack
        let _msg = self
            .socket
            .recv_msg(0)
            .map_err(|e| format!("Could not receive done ack: {e}"))?;

        Ok(())
    }
}
