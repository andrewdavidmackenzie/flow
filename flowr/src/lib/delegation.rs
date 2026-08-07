//! Sub-flow delegation logic for the parent coordinator.
//!
//! Provides the ability to discover peer coordinators, extract a sub-flow,
//! delegate it to a peer, and integrate the results back.

use std::time::Duration;

use flowcore::errors::Result;
use flowcore::model::flow_manifest::FlowManifest;
use log::info;
use serde_json::Value;

use crate::peer_client::PeerClient;
use crate::peer_discovery::discover_peer_coordinators;
use crate::run_state::BoundaryOutput;

/// Result of attempting to delegate a sub-flow to a peer coordinator.
pub struct DelegationResult {
    /// The extracted sub-flow manifest (for reference)
    pub subflow_manifest: FlowManifest,
    /// Boundary outputs received from the peer
    pub boundary_outputs: Vec<BoundaryOutput>,
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
    own_port: Option<u16>,
    inputs: Vec<(usize, usize, Value)>,
) -> Result<Option<DelegationResult>> {
    // Discover peer coordinators
    let peers = discover_peer_coordinators(Duration::from_secs(3), own_port)?;

    if peers.is_empty() {
        info!("No peer coordinators available for delegation");
        return Ok(None);
    }

    let peer_address = peers.first().ok_or("No peers available")?;
    info!("Delegating sub-flow #{flow_id} to peer at {peer_address}");

    // Extract the sub-flow
    let extracted = manifest.extract_subflow(flow_id)?;

    // Connect to the peer and submit
    let zmq_context = zmq::Context::new();
    let client = PeerClient::connect(&zmq_context, peer_address)?;

    let boundary_outputs = client.submit_subflow(extracted.clone(), inputs)?;

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
