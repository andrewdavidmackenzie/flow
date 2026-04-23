//! Optional mDNS-SD service discovery helpers.
//!
//! These functions provide a convenient way to advertise and discover flow services
//! using mDNS-SD. They are not required — other binaries using `flowrlib` can
//! implement their own discovery mechanism.

use std::time::{Duration, Instant};

use log::info;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use flowcore::errors::{bail, Result};

use crate::services::FLOW_SERVICE_TYPE;

const DEFAULT_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Register a service for mDNS-SD discovery.
///
/// The returned `ServiceDaemon` must be kept alive by the caller — the service is
/// unregistered when the daemon is dropped.
pub fn enable_service_discovery(name: &str, service_port: u16) -> Result<ServiceDaemon> {
    let mdns = ServiceDaemon::new().map_err(|e| format!("Could not create mDNS daemon: {e}"))?;

    let service_hostname = format!("{name}.local.");

    let service_info = ServiceInfo::new(
        FLOW_SERVICE_TYPE,
        name,
        &service_hostname,
        "",
        service_port,
        None,
    )
    .map_err(|e| format!("Could not create mDNS ServiceInfo: {e}"))?
    .enable_addr_auto();

    mdns.register(service_info)
        .map_err(|e| format!("Could not register mDNS service: {e}"))?;

    info!("mDNS service registered: '{name}' on port {service_port}");

    Ok(mdns)
}

/// Discover a service by name using mDNS-SD. Blocks until the service is found
/// or the default timeout (30 seconds) expires.
///
/// Returns the service address as `"{ip}:{port}"`.
///
/// # Errors
/// - Cannot create `ServiceDaemon`
/// - Cannot get receiver for discovery messages
pub fn discover_service(name: &str) -> Result<String> {
    let mdns = ServiceDaemon::new().map_err(|e| format!("Could not create mDNS daemon: {e}"))?;

    let receiver = mdns
        .browse(FLOW_SERVICE_TYPE)
        .map_err(|e| format!("Could not browse for mDNS services: {e}"))?;

    let full_name_suffix = format!(".{FLOW_SERVICE_TYPE}");
    let start = Instant::now();

    loop {
        if start.elapsed() > DEFAULT_DISCOVERY_TIMEOUT {
            mdns.shutdown().ok();
            bail!(format!(
                "mDNS discovery timed out after {}s for '{name}'",
                DEFAULT_DISCOVERY_TIMEOUT.as_secs()
            ));
        }

        if let Ok(ServiceEvent::ServiceResolved(info)) = receiver.recv_timeout(Duration::from_millis(500)) {
            let instance = info
                .get_fullname()
                .strip_suffix(&full_name_suffix)
                .unwrap_or(info.get_fullname());

            if instance == name {
                let port = info.get_port();
                if let Some(addr) = info.get_addresses_v4().into_iter().next() {
                    let address = format!("{addr}:{port}");
                    info!("Discovered mDNS service '{name}' at {address}");
                    mdns.shutdown().ok();
                    return Ok(address);
                }
            }
        }
    }
}
