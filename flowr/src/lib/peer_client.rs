//! Client for communicating with a peer coordinator.
//!
//! Used by a parent coordinator to delegate sub-flows to discovered peer
//! coordinators on the network.

use flowcore::errors::Result;
use flowcore::model::flow_manifest::FlowManifest;
use log::{debug, info};
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
    /// # Errors
    ///
    /// Returns an error if the submission cannot be sent or the response
    /// cannot be received/parsed.
    pub fn submit_subflow(
        &self,
        manifest: FlowManifest,
        inputs: Vec<(usize, usize, Value)>,
    ) -> Result<Vec<BoundaryOutput>> {
        let request = PeerRequest::Submit(Box::new(SubflowSubmission { manifest, inputs }));
        let json = serde_json::to_string(&request)
            .map_err(|e| format!("Could not serialize peer request: {e}"))?;

        self.socket
            .send(json.as_bytes(), 0)
            .map_err(|e| format!("Could not send to peer: {e}"))?;

        // Receive responses until Idle or Error
        let mut boundary_outputs = Vec::new();
        loop {
            let msg = self
                .socket
                .recv_msg(0)
                .map_err(|e| format!("Could not receive from peer: {e}"))?;
            let msg_str = msg
                .as_str()
                .ok_or("Could not convert peer response to string")?;
            let response: PeerResponse = serde_json::from_str(msg_str)
                .map_err(|e| format!("Could not deserialize peer response: {e}"))?;

            match response {
                PeerResponse::BoundaryOutput { connection, value } => {
                    debug!(
                        "Peer boundary output: -> #{}:{}",
                        connection.destination_id, connection.destination_io_number
                    );
                    boundary_outputs.push(BoundaryOutput { connection, value });
                    // Acknowledge so peer can send next output (REQ/REP pattern)
                    self.socket
                        .send("ack".as_bytes(), 0)
                        .map_err(|e| format!("Could not send ack: {e}"))?;
                }
                PeerResponse::Idle => {
                    info!(
                        "Peer sub-flow idle ({} boundary outputs)",
                        boundary_outputs.len()
                    );
                    break;
                }
                PeerResponse::Error(msg) => {
                    return Err(format!("Peer coordinator error: {msg}").into());
                }
                PeerResponse::DoneAck => {
                    break;
                }
            }
        }

        Ok(boundary_outputs)
    }

    /// Submit a sub-flow and invoke a callback for each boundary output
    /// as it arrives, rather than collecting them all.
    ///
    /// # Errors
    ///
    /// Returns an error if the submission fails, a response cannot be
    /// parsed, or the callback returns an error.
    pub fn submit_subflow_streaming<F>(
        &self,
        manifest: FlowManifest,
        inputs: Vec<(usize, usize, Value)>,
        mut on_boundary_output: F,
    ) -> Result<()>
    where
        F: FnMut(BoundaryOutput) -> Result<()>,
    {
        let request = PeerRequest::Submit(Box::new(SubflowSubmission { manifest, inputs }));
        let json = serde_json::to_string(&request)
            .map_err(|e| format!("Could not serialize peer request: {e}"))?;

        self.socket
            .send(json.as_bytes(), 0)
            .map_err(|e| format!("Could not send to peer: {e}"))?;

        loop {
            let msg = self
                .socket
                .recv_msg(0)
                .map_err(|e| format!("Could not receive from peer: {e}"))?;
            let msg_str = msg
                .as_str()
                .ok_or("Could not convert peer response to string")?;
            let response: PeerResponse = serde_json::from_str(msg_str)
                .map_err(|e| format!("Could not deserialize peer response: {e}"))?;

            match response {
                PeerResponse::BoundaryOutput { connection, value } => {
                    on_boundary_output(BoundaryOutput { connection, value })?;
                    self.socket
                        .send("ack".as_bytes(), 0)
                        .map_err(|e| format!("Could not send ack: {e}"))?;
                }
                PeerResponse::Idle | PeerResponse::DoneAck => break,
                PeerResponse::Error(msg) => {
                    return Err(format!("Peer coordinator error: {msg}").into());
                }
            }
        }

        Ok(())
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
