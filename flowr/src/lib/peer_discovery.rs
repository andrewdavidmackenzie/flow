//! Discovery of peer coordinators on the network for sub-flow delegation.

use std::time::Duration;

use flowcore::discovery::discover_services;
use flowcore::errors::Result;
use flowcore::services::PEER_COORDINATOR_SERVICE_NAME;
use log::info;

/// Discover peer coordinators on the network, excluding the local one.
///
/// `own_address` is the full `address:port` of this coordinator's own peer
/// service (if any). Peers matching this exact address are filtered out.
///
/// # Errors
///
/// Returns an error if mDNS discovery fails.
pub fn discover_peer_coordinators(
    timeout: Duration,
    own_address: Option<&str>,
) -> Result<Vec<String>> {
    let services = discover_services(PEER_COORDINATOR_SERVICE_NAME, timeout)?;

    let peers: Vec<String> = services
        .into_iter()
        .filter(|(addr, _)| {
            // Filter out our own peer coordinator by exact address match
            own_address.is_none_or(|own| *addr != own)
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
