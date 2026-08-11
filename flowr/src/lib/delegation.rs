//! Sub-flow delegation logic for the parent coordinator.
//!
//! Provides the ability to discover peer coordinators, extract a sub-flow,
//! delegate it to a peer, and integrate the results back.

use std::time::Duration;

use flowcore::errors::Result;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::input::InputInitializer;
use flowcore::model::submission::Submission;
use log::info;
use serde_json::Value;

use crate::client_protocol::{BoundaryOutputEntry, ClientMessage, CoordinatorMessage};
use crate::connections::ClientConnection;
use crate::peer_discovery::discover_peer_coordinators;

/// Result of attempting to delegate a sub-flow to a peer coordinator.
pub struct DelegationResult {
    /// The extracted sub-flow manifest (for reference)
    pub subflow_manifest: FlowManifest,
    /// Boundary outputs received from the peer
    pub boundary_outputs: Vec<BoundaryOutputEntry>,
    /// The flow ID that was delegated
    pub flow_id: usize,
    /// The address of the peer that executed it
    pub peer_address: String,
}

/// Attempt to delegate a sub-flow to a peer coordinator.
///
/// Discovers peer coordinators, selects the first available one, extracts
/// the specified sub-flow, sends it to the peer, and returns the results.
///
/// # Errors
///
/// Returns an error if discovery, connection, or execution fails.
pub fn delegate_subflow(
    manifest: &FlowManifest,
    flow_id: usize,
    own_instance: Option<&str>,
    inputs: &[(usize, usize, Value)],
) -> Result<Option<DelegationResult>> {
    // Discover peer coordinators
    let peers = discover_peer_coordinators(Duration::from_secs(3), own_instance)?;

    if peers.is_empty() {
        info!("No peer coordinators available for delegation");
        return Ok(None);
    }

    let peer_address = peers.first().ok_or("No peers available")?;
    info!("Delegating sub-flow #{flow_id} to peer at {peer_address}");

    // Extract the sub-flow
    let mut extracted = manifest.extract_subflow(flow_id)?;

    // Inject inputs as Once initializers on boundary functions
    for (dest_func_id, dest_io_number, value) in inputs {
        let func = extracted
            .get_functions()
            .get_mut(dest_func_id)
            .ok_or_else(|| {
                format!(
                    "Input destination function #{dest_func_id} not found in extracted sub-flow"
                )
            })?;
        func.set_flow_initializer(*dest_io_number, InputInitializer::Once(value.clone()));
    }

    // Build a sub-flow submission
    let mut submission = Submission::new(
        extracted.clone(),
        None,
        None,
        #[cfg(feature = "debugger")]
        false,
        #[cfg(feature = "trace")]
        None,
    );
    submission.is_subflow = true;

    // Connect to the peer and submit using the unified protocol
    let connection = ClientConnection::new(peer_address)?;
    // Sub-flow execution may take much longer than the default 30s.
    // Use 5 minutes — long enough for real workloads, finite enough to
    // avoid hanging forever if the peer exits unexpectedly.
    connection.set_receive_timeout(300_000)?;

    connection.send(ClientMessage::ClientSubmission(Box::new(submission)))?;

    let response: CoordinatorMessage = connection.receive()?;

    #[cfg(feature = "metrics")]
    let boundary_outputs = match response {
        CoordinatorMessage::FlowEnd(outputs, _) => outputs,
        CoordinatorMessage::CoordinatorExiting(result) => {
            return Err(format!("Peer coordinator exited: {result:?}").into());
        }
        other => return Err(format!("Unexpected response from peer: {other}").into()),
    };

    #[cfg(not(feature = "metrics"))]
    let boundary_outputs = match response {
        CoordinatorMessage::FlowEnd(outputs) => outputs,
        CoordinatorMessage::CoordinatorExiting(result) => {
            return Err(format!("Peer coordinator exited: {result:?}").into());
        }
        other => return Err(format!("Unexpected response from peer: {other}").into()),
    };

    info!(
        "Peer returned {} boundary outputs for sub-flow #{flow_id}",
        boundary_outputs.len()
    );

    Ok(Some(DelegationResult {
        subflow_manifest: extracted,
        boundary_outputs,
        flow_id,
        peer_address: peer_address.clone(),
    }))
}
