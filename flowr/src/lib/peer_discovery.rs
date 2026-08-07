//! Discovery of peer coordinators on the network for sub-flow delegation.

use std::time::Duration;

use flowcore::discovery::discover_services_by_prefix;
use flowcore::errors::Result;
use flowcore::services::PEER_COORDINATOR_SERVICE_NAME;
use log::info;

/// Discover peer coordinators on the network, excluding the local one.
///
/// `own_port` is the port of this coordinator's own peer service (if any).
/// Peers whose port matches are filtered out (since we may appear under
/// multiple IP addresses via mDNS).
///
/// Peer coordinators register with instance names prefixed with
/// `PEER_COORDINATOR_SERVICE_NAME` (e.g. `peer-coordinator-12345-20000`).
/// This function matches any instance starting with that prefix.
///
/// # Errors
///
/// Returns an error if mDNS discovery fails.
pub fn discover_peer_coordinators(timeout: Duration, own_port: Option<u16>) -> Result<Vec<String>> {
    let services = discover_services_by_prefix(PEER_COORDINATOR_SERVICE_NAME, timeout)?;

    let peers: Vec<String> = services
        .into_iter()
        .filter(|(addr, _)| {
            // Filter out our own peer coordinator by port number
            own_port.is_none_or(|port| {
                addr.rsplit_once(':')
                    .and_then(|(_, p)| p.parse::<u16>().ok())
                    .is_none_or(|peer_port| peer_port != port)
            })
        })
        .map(|(addr, _)| addr)
        .collect();

    if peers.is_empty() {
        info!("No peer coordinators discovered");
    } else {
        info!("Discovered {} peer coordinator(s)", peers.len());
    }

    Ok(peers)
}
