//! Coordinator module for running flows in-process.
//!
//! Adapted from flowrgui's connection manager. Spawns a coordinator in a
//! background thread and communicates via ZMQ sockets through an iced Subscription.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use iced::futures::SinkExt;
use iced::Subscription;
use log::{error, info, trace};
use portpicker::pick_unused_port;
use tokio::sync::mpsc::Receiver;
use url::Url;

use flowcore::errors::{Result, ResultExt};
use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;
use flowrlib::coordinator::Coordinator;
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME};

pub(crate) mod client_connection;
pub(crate) mod client_message;
pub(crate) mod coordinator_connection;
pub(crate) mod coordinator_message;
mod debug_handler;
mod submission_handler;

pub(crate) use coordinator_connection::CoordinatorConnection;

use client_connection::{discover_service, ClientConnection};
use client_message::ClientMessage;
use coordinator_connection::{
    enable_service_discovery, COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
};
use coordinator_message::CoordinatorMessage;
use debug_handler::NoOpDebugHandler;
use submission_handler::CLISubmissionHandler;

/// Settings for starting the coordinator server
#[derive(Clone)]
pub(crate) struct ServerSettings {
    /// Use natively linked flowstdlib
    pub native_flowstdlib: bool,
    /// Number of executor threads
    pub num_threads: usize,
    /// Library search path
    pub lib_search_path: simpath::Simpath,
}

/// States of the coordinator connection
enum CoordinatorState {
    Init(ServerSettings),
    Discovery,
    Discovered(String),
    Connected(Receiver<ClientMessage>, Arc<Mutex<ClientConnection>>),
}

/// Global storage for server settings
static SERVER_SETTINGS: OnceLock<ServerSettings> = OnceLock::new();

/// Create a subscription that manages the coordinator connection
pub(crate) fn subscribe(settings: ServerSettings) -> Subscription<CoordinatorMessage> {
    SERVER_SETTINGS.get_or_init(|| settings);
    Subscription::run(coordinator_stream)
}

// Matches flowrgui's connection_manager::coordinator_stream exactly
#[allow(clippy::unwrap_used)]
fn coordinator_stream() -> impl iced::futures::Stream<Item = CoordinatorMessage> {
    let settings = SERVER_SETTINGS.get().unwrap().clone();
    iced::stream::channel(
        100,
        move |mut app_sender: iced::futures::channel::mpsc::Sender<CoordinatorMessage>| async move {
            let mut state = CoordinatorState::Init(settings);
            let mut running = false;

            loop {
                match state {
                    CoordinatorState::Init(ref settings) => {
                        start_server(settings.clone()).unwrap(); // TODO
                        state = CoordinatorState::Discovery;
                    }

                    CoordinatorState::Discovery => {
                        let address = discover_service(COORDINATOR_SERVICE_NAME).unwrap(); // TODO
                        state = CoordinatorState::Discovered(address);
                    }

                    CoordinatorState::Discovered(address) => {
                        let connection = ClientConnection::new(&address).unwrap(); // TODO

                        // Create channel to get messages from the app
                        let (app_side_sender, app_receiver) = tokio::sync::mpsc::channel(100);

                        // Send the Sender to the App in a Message, for App to use to send us messages
                        let _ = app_sender
                            .send(CoordinatorMessage::Connected(app_side_sender))
                            .await;

                        state = CoordinatorState::Connected(
                            app_receiver,
                            Arc::new(Mutex::new(connection)),
                        );
                    }

                    CoordinatorState::Connected(ref mut app_receiver, ref connection) => {
                        if running {
                            // read the message back from the Coordinator
                            let coordinator_message: CoordinatorMessage =
                                connection.lock().unwrap().receive().unwrap(); // TODO

                            // Forward the message to the app
                            let _ = app_sender.send(coordinator_message.clone()).await;

                            // If that was end of flow, there will be no response from app
                            if matches!(&coordinator_message, &CoordinatorMessage::FlowEnd(_)) {
                                running = false;
                            } else {
                                // read the message back from the app and send it to the Coordinator
                                #[allow(clippy::single_match_else)]
                                match app_receiver.recv().await {
                                    Some(client_message) => {
                                        connection.lock().unwrap().send(client_message).unwrap();
                                    }
                                    None => error!("Error receiving from app"), // TODO
                                }
                            }

                            // TODO handle coordinator exit, disconnection or error
                        } else {
                            // read the Submit message from the app and send it to the coordinator
                            if let Some(client_message) = app_receiver.recv().await {
                                connection.lock().unwrap().send(client_message).unwrap();
                                running = true;
                            }
                        }
                    }
                }
            }
        },
    )
}

fn start_server(settings: ServerSettings) -> Result<()> {
    let runtime_port = pick_unused_port().chain_err(|| "No ports free")?;
    let coordinator_connection =
        CoordinatorConnection::new(COORDINATOR_SERVICE_NAME, runtime_port)?;
    let _mdns_coordinator = enable_service_discovery(COORDINATOR_SERVICE_NAME, runtime_port)?;

    let debug_port = pick_unused_port().chain_err(|| "No ports free")?;
    let debug_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME, debug_port)?;
    let _mdns_debug = enable_service_discovery(DEBUG_SERVICE_NAME, debug_port)?;

    info!("Starting coordinator in background thread");
    thread::spawn(move || {
        let _ = run_coordinator(settings, coordinator_connection, debug_connection);
    });

    Ok(())
}

fn run_coordinator(
    settings: ServerSettings,
    coordinator_connection: CoordinatorConnection,
    debug_connection: CoordinatorConnection,
) -> Result<()> {
    let connection = Arc::new(Mutex::new(coordinator_connection));

    let mut debug_server = NoOpDebugHandler {
        connection: debug_connection,
    };

    let provider = Arc::new(MetaProvider::new(
        settings.lib_search_path,
        PathBuf::from("/"),
    )) as Arc<dyn Provider>;

    let ports = get_four_ports()?;
    trace!("Announcing job queues on ports: {ports:?}");
    let job_queues = get_bind_addresses(ports);
    let dispatcher = Dispatcher::new(&job_queues)?;
    let _mdns_jobs = enable_service_discovery(JOB_SERVICE_NAME, ports.0)?;
    let _mdns_results = enable_service_discovery(RESULTS_JOB_SERVICE_NAME, ports.2)?;
    let _mdns_control = enable_service_discovery(CONTROL_SERVICE_NAME, ports.3)?;

    let (job_source_name, context_job_source_name, results_sink, control_socket) =
        get_connect_addresses(ports);

    let mut executor = Executor::new();
    if settings.native_flowstdlib {
        executor.add_lib(
            flowstdlib::manifest::get().chain_err(|| "Could not get native flowstdlib manifest")?,
            Url::parse("memory://")?,
        )?;
    }
    executor.start(
        &provider,
        settings.num_threads,
        &job_source_name,
        &results_sink,
        &control_socket,
    );

    let mut context_executor = Executor::new();
    context_executor.add_lib(
        crate::context::get_manifest(connection.clone())?,
        Url::parse("memory://")?,
    )?;
    context_executor.start(
        &provider,
        1,
        &context_job_source_name,
        &results_sink,
        &control_socket,
    );

    let mut submitter = CLISubmissionHandler::new(connection);
    let mut coordinator = Coordinator::new(dispatcher, &mut submitter, &mut debug_server);
    Ok(coordinator.submission_loop(true)?)
}

fn get_connect_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://127.0.0.1:{}", ports.0),
        format!("tcp://127.0.0.1:{}", ports.1),
        format!("tcp://127.0.0.1:{}", ports.2),
        format!("tcp://127.0.0.1:{}", ports.3),
    )
}

fn get_bind_addresses(ports: (u16, u16, u16, u16)) -> (String, String, String, String) {
    (
        format!("tcp://*:{}", ports.0),
        format!("tcp://*:{}", ports.1),
        format!("tcp://*:{}", ports.2),
        format!("tcp://*:{}", ports.3),
    )
}

fn get_four_ports() -> Result<(u16, u16, u16, u16)> {
    Ok((
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
    ))
}
