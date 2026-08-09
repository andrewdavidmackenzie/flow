//! Peer coordinator for accepting delegated sub-flow submissions.
//!
//! This module provides `run_peer_coordinator()`, which sets up a complete
//! coordinator infrastructure (dispatcher, executor, WASM server) and listens
//! for sub-flow submissions from parent coordinators via a ZMQ REP socket
//! advertised over mDNS.
//!
//! Used by `flowrcli` (via `--server`) to act as a peer coordinator.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use log::{error, info};
use simpath::Simpath;
use url::Url;

use flowcore::discovery::{create_service_daemon, register_service, shutdown_service_daemon};
use flowcore::errors::{Result, ResultExt};
use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;

use crate::coordinator::Coordinator;
use crate::dispatcher::Dispatcher;
use crate::executor::Executor;
use crate::peer_submission_handler::PeerSubmissionHandler;
use crate::services::PEER_COORDINATOR_SERVICE_NAME;
use crate::wasm_server::WasmServer;
/// Start a peer coordinator in the background. Returns the mDNS instance
/// name so the caller can filter it from peer discovery.
///
/// # Errors
///
/// Returns an error if the port cannot be allocated.
pub fn start_peer_coordinator() -> Result<String> {
    let port = portpicker::pick_unused_port().ok_or("No ports free for peer coordinator")?;
    let instance_name = format!(
        "{PEER_COORDINATOR_SERVICE_NAME}-{}-{port}",
        std::process::id()
    );
    let name_copy = instance_name.clone();
    std::thread::spawn(move || {
        if let Err(e) = run_peer_coordinator(port, &name_copy) {
            log::error!("Peer coordinator error: {e}");
        }
    });
    Ok(instance_name)
}

/// Run a peer coordinator that accepts sub-flow submissions from parent
/// coordinators. Advertises itself via mDNS and listens on a ZMQ REP socket.
///
/// This function blocks forever, processing submissions in a loop. It sets up
/// its own dispatcher, executor pool, and WASM server independently.
///
/// # Errors
///
/// Returns an error if the coordinator cannot be set up or if the submission
/// loop encounters a fatal error.
pub fn run_peer_coordinator(peer_port: u16, instance_name: &str) -> Result<()> {
    let bind_address = format!("tcp://*:{peer_port}");

    let mdns = create_service_daemon()?;
    let fullname = register_service(&mdns, instance_name, peer_port)?;
    info!("Peer coordinator '{instance_name}' advertised on port {peer_port}");

    let zmq_context = zmq::Context::new();
    let mut peer_handler = PeerSubmissionHandler::new(&zmq_context, &bind_address)?;

    // Set up dispatcher and executor for running received sub-flows
    let ports = (
        portpicker::pick_unused_port().ok_or("No ports free")?,
        portpicker::pick_unused_port().ok_or("No ports free")?,
        portpicker::pick_unused_port().ok_or("No ports free")?,
        portpicker::pick_unused_port().ok_or("No ports free")?,
    );
    let bind_addrs = (
        format!("tcp://*:{}", ports.0),
        format!("tcp://*:{}", ports.1),
        format!("tcp://*:{}", ports.2),
        format!("tcp://*:{}", ports.3),
    );
    let connect_addrs = (
        format!("tcp://127.0.0.1:{}", ports.0),
        format!("tcp://127.0.0.1:{}", ports.1),
        format!("tcp://127.0.0.1:{}", ports.2),
        format!("tcp://127.0.0.1:{}", ports.3),
    );

    let dispatcher = Dispatcher::new(&bind_addrs)?;

    let provider =
        Arc::new(MetaProvider::new(Simpath::new(""), PathBuf::default())) as Arc<dyn Provider>;

    let mut executor = Executor::new();
    #[cfg(feature = "flowstdlib")]
    executor.add_lib(
        flowstdlib::manifest::get().chain_err(|| "Could not get 'native' flowstdlib manifest")?,
        Url::parse("memory://")?,
    )?;
    executor.start(
        &provider,
        thread::available_parallelism().map_or(1, std::num::NonZero::get),
        &connect_addrs.0,
        &connect_addrs.2,
        &connect_addrs.3,
    );

    // Start WASM server so this peer's executors can fetch WASM modules
    // over HTTP (e.g. when the delegated sub-flow contains file:// WASMs
    // that were rewritten to http:// by the parent coordinator)
    let wasm_server = match WasmServer::start(std::path::Path::new("/")) {
        Ok(server) => {
            info!("Peer WASM server at {}", server.base_url());
            Some(server)
        }
        Err(e) => {
            log::warn!("Could not start WASM server for peer coordinator: {e}");
            None
        }
    };

    #[cfg(feature = "debugger")]
    let mut debug_handler = crate::subflow::NoOpDebugHandler;

    let mut coordinator = Coordinator::new(
        dispatcher,
        #[cfg(feature = "submission")]
        &mut peer_handler,
        #[cfg(feature = "debugger")]
        &mut debug_handler,
    );

    // Set the WASM base URL so the peer coordinator can rewrite file:// URLs
    // for its own executors (and for further delegation if supported)
    if let Some(ref server) = wasm_server {
        if let Ok(url) = Url::parse(server.base_url()) {
            coordinator.set_wasm_base_url(url);
        }
    }

    info!("Peer coordinator entering submission loop");
    let result = coordinator.submission_loop(true);

    // Cleanup
    let _ = coordinator.send_done();
    executor.wait();
    if let Err(e) = shutdown_service_daemon(&mdns, &[fullname]) {
        error!("Could not shut down peer mDNS: {e}");
    }

    result
}
