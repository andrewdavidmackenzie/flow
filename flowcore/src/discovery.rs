//! Optional mDNS-SD service discovery helpers.
//!
//! These functions provide a convenient way to advertise and discover flow services
//! using mDNS-SD. They are not required — other binaries using `flowrlib` can
//! implement their own discovery mechanism.

use std::time::{Duration, Instant};

use log::{debug, info};
pub use mdns_sd::ServiceDaemon;
use mdns_sd::{ServiceEvent, ServiceInfo};

use crate::errors::{bail, Result};

use crate::services::FLOW_SERVICE_TYPE;

const DEFAULT_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Create a new mDNS service daemon.
///
/// Prefer creating a single daemon and registering multiple services on it via
/// [`register_service`] instead of creating one daemon per service — this avoids
/// resource contention on Windows (see [`mdns-sd`] issue #...).
///
/// The returned `ServiceDaemon` must be kept alive for the lifetime of the
/// registered services. All services are unregistered when the daemon is dropped.
///
/// # Errors
///
/// Returns an error if the mDNS daemon cannot be initialized.
pub fn create_service_daemon() -> Result<ServiceDaemon> {
    let mdns = ServiceDaemon::new().map_err(|e| format!("Could not create mDNS daemon: {e}"))?;
    Ok(mdns)
}

/// Register a service for mDNS-SD discovery on an existing daemon.
///
/// Use together with [`create_service_daemon`] when registering multiple services
/// so that only one daemon thread and one multicast socket are created.
///
/// Returns the service's mDNS "fullname" (e.g. `"runtime._flow._tcp.local."`), which
/// must be kept and passed to [`unregister_service`] to properly deregister the
/// service (send a "goodbye" packet) before the daemon is shut down. `mdns-sd`'s
/// `ServiceDaemon` has no `Drop` implementation of its own: simply letting it go out
/// of scope does **not** unregister services or notify other hosts — see
/// [`shutdown_service_daemon`].
///
/// # Errors
///
/// Returns an error if the service registration fails.
pub fn register_service(mdns: &ServiceDaemon, name: &str, service_port: u16) -> Result<String> {
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

    let fullname = service_info.get_fullname().to_string();

    mdns.register(service_info)
        .map_err(|e| format!("Could not register mDNS service: {e}"))?;

    info!("mDNS service registered: '{name}' on port {service_port}");

    Ok(fullname)
}

/// Unregister a single service (identified by the "fullname" returned from
/// [`register_service`]), waiting briefly for confirmation that the "goodbye"
/// packet was sent.
///
/// This is best-effort: failures are logged (at debug level) rather than returned,
/// since callers typically call this while already tearing down/exiting.
pub fn unregister_service(mdns: &ServiceDaemon, fullname: &str) {
    match mdns.unregister(fullname) {
        Ok(rx) => {
            if let Err(e) = rx.recv_timeout(Duration::from_secs(1)) {
                debug!("No confirmation of mDNS unregister for '{fullname}': {e}");
            }
        }
        Err(e) => debug!("Could not unregister mDNS service '{fullname}': {e}"),
    }
}

/// Unregister all `fullnames` (as returned by [`register_service`]) then shut the
/// daemon down.
///
/// Unregistering before shutdown gives other hosts a "goodbye" packet so they don't
/// keep treating the service as available after this process exits - simply
/// dropping the `ServiceDaemon` does neither of these things (see [`register_service`]).
///
/// # Errors
///
/// Returns an error if the daemon itself could not be shut down. Failures to
/// unregister individual services are logged but do not cause an error, since
/// the daemon is being shut down regardless.
pub fn shutdown_service_daemon(mdns: &ServiceDaemon, fullnames: &[String]) -> Result<()> {
    for fullname in fullnames {
        unregister_service(mdns, fullname);
    }

    mdns.shutdown()
        .map_err(|e| format!("Could not shut down mDNS daemon: {e}"))?;

    Ok(())
}

/// Register a service for mDNS-SD discovery, creating a new daemon.
///
/// The returned `ServiceDaemon` must be explicitly torn down with
/// [`shutdown_service_daemon`] (passing back the service's fullname) once the
/// service is no longer needed — dropping the `ServiceDaemon` alone does not
/// unregister the service or shut down its background thread.
///
/// Prefer [`create_service_daemon`] + [`register_service`] when registering
/// multiple services, to share a single daemon.
///
/// # Errors
///
/// Returns an error if the mDNS daemon or service registration fails.
pub fn enable_service_discovery(name: &str, service_port: u16) -> Result<ServiceDaemon> {
    let mdns = create_service_daemon()?;
    register_service(&mdns, name, service_port)?;
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

        if let Ok(ServiceEvent::ServiceResolved(info)) =
            receiver.recv_timeout(Duration::from_millis(500))
        {
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

/// Discover all instances of a service by name using mDNS-SD.
///
/// Scans for the given timeout and returns all matching services found as
/// `(address, port)` pairs.
///
/// # Errors
/// - Cannot create `ServiceDaemon`
/// - Cannot get receiver for discovery messages
pub fn discover_services(name: &str, timeout: Duration) -> Result<Vec<(String, u16)>> {
    let mdns = ServiceDaemon::new().map_err(|e| format!("Could not create mDNS daemon: {e}"))?;

    let receiver = mdns
        .browse(FLOW_SERVICE_TYPE)
        .map_err(|e| format!("Could not browse for mDNS services: {e}"))?;

    let full_name_suffix = format!(".{FLOW_SERVICE_TYPE}");
    let start = Instant::now();
    let mut results = Vec::new();

    loop {
        if start.elapsed() > timeout {
            break;
        }

        if let Ok(ServiceEvent::ServiceResolved(info)) =
            receiver.recv_timeout(Duration::from_millis(500))
        {
            let instance = info
                .get_fullname()
                .strip_suffix(&full_name_suffix)
                .unwrap_or(info.get_fullname());

            if instance == name {
                let port = info.get_port();
                if let Some(addr) = info.get_addresses_v4().into_iter().next() {
                    let address = format!("{addr}:{port}");
                    if !results.iter().any(|(a, _): &(String, u16)| *a == address) {
                        info!("Discovered mDNS service '{name}' at {address}");
                        results.push((address, port));
                    }
                }
            }
        }
    }

    mdns.shutdown().ok();
    Ok(results)
}
