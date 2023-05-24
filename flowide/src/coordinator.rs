use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use log::{info, trace};
use portpicker::pick_unused_port;
use simpath::Simpath;
use url::Url;

use flowcore::meta_provider::MetaProvider;
use flowcore::model::flow_manifest::FlowManifest;
use flowcore::model::submission::Submission;
use flowcore::provider::Provider;
use flowrlib::coordinator::Coordinator;
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME};

use crate::errors::*;
use crate::gui;
use crate::gui::client::Client;
use crate::gui::client_connection::ClientConnection;
use crate::gui::client_connection::discover_service;
use crate::gui::coordinator_connection::{COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
                                         enable_service_discovery};
use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_message::ClientMessage;
use crate::gui::debug_client::DebugClient;
use crate::gui::debug_handler::CliDebugHandler;
use crate::gui::submission_handler::CLISubmissionHandler;

#[derive(Debug, Clone)]
pub(crate) enum GuiCoordinator {
    Unknown,
    Found((String, u16)), // coordinator_address, discovery_port
}

impl GuiCoordinator {
    pub(crate) fn debug_client(
        override_args: Arc<Mutex<Vec<String>>>,
        discovery_port: u16,
    ) -> Result<()> {
        trace!("Creating Debug Client");
        let debug_server_address = discover_service(discovery_port,
                                                    DEBUG_SERVICE_NAME)?;
        let debug_client_connection = ClientConnection::new(&debug_server_address)?;
        let debug_client = DebugClient::new(debug_client_connection,
                                            override_args);
        let _ = thread::spawn(move || {
            debug_client.debug_client_loop();
        });

        Ok(())
    }

    pub(crate) async fn submit(client: &mut Client,
                         url: Url,
                         parallel_jobs_limit: Option<usize>,
                         debug_this_flow: bool) -> Result<()> {
        let provider = &MetaProvider::new(Simpath::new(""),
                                          PathBuf::default()) as &dyn Provider;

        let (flow_manifest, _) = FlowManifest::load(provider, &url)?;
        let submission = Submission::new(
            flow_manifest,
            parallel_jobs_limit,
            None, // No timeout waiting for job results
            debug_this_flow,
        );

        info!("Client sending submission to coordinator");
        client.send(ClientMessage::ClientSubmission(submission)).await?;

        Ok(())
    }

    pub(crate) fn event_loop(mut client: Client) -> Result<()> {
        trace!("Entering client event loop");
        Ok(client.event_loop()?)
    }
}

pub(crate) fn start(
    num_threads: usize,
    lib_search_path: Simpath,
    native_flowstdlib: bool,
) -> (String, u16) {
    let runtime_port = pick_unused_port().chain_err(|| "No ports free").unwrap(); // TODO
    let coordinator_connection = CoordinatorConnection::new(COORDINATOR_SERVICE_NAME,
                                                            runtime_port).unwrap(); // TODO

    let discovery_port = pick_unused_port().chain_err(|| "No ports free").unwrap(); //TODO
    enable_service_discovery(discovery_port, COORDINATOR_SERVICE_NAME, runtime_port)
        .unwrap(); // TODO

    let debug_port = pick_unused_port().chain_err(|| "No ports free").unwrap(); // TODO
    let debug_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME,
                                                      debug_port).unwrap(); // TODO
    enable_service_discovery(discovery_port, DEBUG_SERVICE_NAME, debug_port)
        .unwrap(); // TODO

    info!("Starting coordinator in background thread");
    thread::spawn(move || {
        let _ = coordinator_main(
            num_threads,
            lib_search_path,
            native_flowstdlib,
            coordinator_connection,
            debug_connection,
            false,
        );
    });

    let coordinator_address = discover_service(discovery_port, COORDINATOR_SERVICE_NAME)
        .unwrap(); // TODO;

    (coordinator_address, discovery_port)
}

fn coordinator_main(
    num_threads: usize,
    lib_search_path: Simpath,
    native_flowstdlib: bool,
    coordinator_connection: CoordinatorConnection,
    debug_connection: CoordinatorConnection,
    loop_forever: bool,
) -> Result<()> {
    let connection = Arc::new(Mutex::new(coordinator_connection));

    let mut debug_server = CliDebugHandler { debug_server_connection: debug_connection };

    let provider = Arc::new(MetaProvider::new(lib_search_path,
                                              PathBuf::from("/"))) as Arc<dyn Provider>;

    let ports = get_four_ports()?;
    trace!("Announcing three job queues and a control socket on ports: {ports:?}");
    let job_queues = get_bind_addresses(ports);
    let dispatcher = Dispatcher::new(job_queues)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, ports.0)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, RESULTS_JOB_SERVICE_NAME, ports.2)?;
    enable_service_discovery(JOB_QUEUES_DISCOVERY_PORT, CONTROL_SERVICE_NAME, ports.3)?;

    let (job_source_name, context_job_source_name, results_sink, control_socket) =
        get_connect_addresses(ports);

    let mut executor = Executor::new()?;
    // if the command line options request loading native implementation of available native libs
    // if not, the native implementation is not loaded and later when a flow is loaded it's library
    // references will be resolved and those libraries (WASM implementations) will be loaded at runtime
    if native_flowstdlib {
        executor.add_lib(
            flowstdlib::manifest::get_manifest()
                .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
            Url::parse("memory://")? // Statically linked library has no resolved Url
        )?;
    }
    executor.start(provider.clone(), num_threads,
                   &job_source_name,
                   &results_sink,
                   &control_socket,
    );

    let mut context_executor = Executor::new()?;
    context_executor.add_lib(
        gui::get_manifest(connection.clone())?,
        Url::parse("memory://")? // Statically linked library has no resolved Url
    )?;
    context_executor.start(provider, 1,
                           &context_job_source_name,
                           &results_sink,
                           &control_socket,
    );

    let mut submitter = CLISubmissionHandler::new(connection);

    let mut coordinator = Coordinator::new(
        dispatcher,
        &mut submitter,
        &mut debug_server
    )?;

    Ok(coordinator.submission_loop(loop_forever)?)
}


// Return addresses and ports to be used for each of the three queues
// - (general) job source
// - context job source
// - results sink
// - control messages
fn get_connect_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://127.0.0.1:{}", ports.0),
        format!("tcp://127.0.0.1:{}", ports.1),
        format!("tcp://127.0.0.1:{}", ports.2),
        format!("tcp://127.0.0.1:{}", ports.3),
    )
}

// Return addresses to bind to for
// - (general) job source
// - context job source
// - results sink
// - control messages
fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://*:{}", ports.0),
        format!("tcp://*:{}", ports.1),
        format!("tcp://*:{}", ports.2),
        format!("tcp://*:{}", ports.3),
    )
}

// Return four free ports to use for client-coordinator message queues
fn get_four_ports() -> Result<(u16, u16, u16, u16)> {
    Ok((pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
    ))
}