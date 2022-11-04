use std::time::Duration;
use log::info;
use simpdiscoverylib::{BeaconListener, BeaconSender};
use flowcore::errors::*;

/// `RUNTIME_SERVICE_NAME` can be used to discover the runtime service by name
pub const RUNTIME_SERVICE_NAME: &str = "runtime._flowr._tcp.local";

/// `DEBUG_SERVICE_NAME` can be used to discover the debug service by name
#[cfg(feature = "debugger")]
pub const DEBUG_SERVICE_NAME: &str = "debug._flowr._tcp.local";

/// `JOB_SERVICE_NAME` can be used to discover the queue serving jobs for execution
pub const JOB_SERVICE_NAME: &str = "jobs._flowr._tcp.local";

/// `CONTEXT_JOB_SERVICE_NAME` can be used to discover the queue serving context jobs for execution
pub const CONTEXT_JOB_SERVICE_NAME: &str = "context_jobs._flowr._tcp.local";

/// `RESULTS_JOB_SERVICE_NAME` can be used to discover the queue where to send job results
pub const RESULTS_JOB_SERVICE_NAME: &str = "results._flowr._tcp.local";

/// WAIT for a message to arrive when performing a receive()
pub const WAIT:i32 = 0;
/// Do NOT WAIT for a message to arrive when performing a receive()
pub static DONT_WAIT:i32 = zmq::DONTWAIT;

/// This is a "well known" port for announcing and discovering the client/server services
pub const CLIENT_SERVER_DISCOVERY_PORT:u16 = 9002;

/// This is a "well known" port for announcing and discovering the job queues
pub const JOB_QUEUES_DISCOVERY_PORT:u16 = 9003;

/// Try to discover a server offering a particular service by name
pub fn discover_service(discovery_port: u16, name: &str) -> Result<String> {
    let listener = BeaconListener::new(name.as_bytes(), discovery_port)?;
    let beacon = listener.wait(None)?;
    let server_address = format!("{}:{}", beacon.service_ip, beacon.service_port);
    Ok(server_address)
}

/// Start a background thread that sends out beacons for service discovery by a client every second
pub fn enable_service_discovery(discovery_port: u16, name: &str, service_port: u16) -> Result<()> {
    match BeaconSender::new(service_port, name.as_bytes(), discovery_port) {
        Ok(beacon) => {
            info!(
                    "Discovery beacon announcing service named '{}', on port: {}",
                    name, service_port
                );
            std::thread::spawn(move || {
                let _ = beacon.send_loop(Duration::from_secs(1));
            });
        }
        Err(e) => bail!("Error starting discovery beacon: {}", e.to_string()),
    }

    Ok(())
}
