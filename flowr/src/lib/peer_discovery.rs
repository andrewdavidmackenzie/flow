//! Discovery of peer coordinators on the network for sub-flow delegation.

use std::time::Duration;

use flowcore::discovery::discover_services_by_prefix;
use flowcore::errors::Result;
use flowcore::services::PEER_COORDINATOR_SERVICE_NAME;
use log::info;

/// Discover peer coordinators on the network, excluding the local one.
///
/// `own_instance` is the mDNS instance name of this coordinator's own peer
/// service (if any). Peers whose address was discovered under this instance
/// name are filtered out.
///
/// # Errors
///
/// Returns an error if mDNS discovery fails.
pub fn discover_peer_coordinators(
    timeout: Duration,
    own_instance: Option<&str>,
) -> Result<Vec<String>> {
    let services = discover_services_by_prefix(PEER_COORDINATOR_SERVICE_NAME, timeout)?;

    // discover_services_by_prefix returns (address, port) pairs.
    // We need the instance name to filter. Re-discover with names.
    // For now, filter by the port embedded in our own instance name.
    let own_port: Option<u16> = own_instance.and_then(|name| {
        name.rsplit_once('-')
            .and_then(|(_, p)| p.parse::<u16>().ok())
    });

    let peers: Vec<String> = services
        .into_iter()
        .filter(|(addr, _)| {
            // Filter out our own peer coordinator by port number extracted
            // from our instance name (format: "peer-coordinator-PID-PORT")
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
