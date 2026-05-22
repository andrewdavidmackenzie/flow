use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use iced::futures::SinkExt;
use iced::Subscription;
use log::{error, info, trace};
use portpicker::pick_unused_port;
use tokio::sync::mpsc::Receiver;
use url::Url;

use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;
use flowrlib::coordinator::Coordinator;
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME};

use crate::errors::{Result, ResultExt};
use crate::gui::client_connection::{discover_service, ClientConnection};
use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_connection::{
    enable_service_discovery, COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
};
use crate::gui::coordinator_message::CoordinatorMessage;
use crate::gui::debug_handler::CliDebugHandler;
use crate::gui::submission_handler::CLISubmissionHandler;
use crate::{context, CoordinatorSettings, ServerSettings};

/// States in which the Connection to the Coordinator can find itself
pub enum CoordinatorState {
    Init(ServerSettings),
    Discovery,
    Discovered(String),
    Connected(Receiver<ClientMessage>, Arc<Mutex<ClientConnection>>),
}

/// Global storage for coordinator settings, initialized once and read by the subscription
static COORDINATOR_SETTINGS: OnceLock<CoordinatorSettings> = OnceLock::new();

/// Global flag for requesting flow stop from the GUI
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Request the currently running flow to stop
pub fn request_stop() {
    STOP_REQUESTED.store(true, Ordering::Relaxed);
}

/// Check and clear the stop request flag
pub fn take_stop_request() -> bool {
    STOP_REQUESTED.swap(false, Ordering::Relaxed)
}

// Creates an asynchronous worker that sends messages back and forth between the App and
// the Coordinator
pub fn subscribe(coordinator_settings: CoordinatorSettings) -> Subscription<CoordinatorMessage> {
    COORDINATOR_SETTINGS.get_or_init(|| coordinator_settings);
    Subscription::run(coordinator_stream)
}

fn coordinator_stream() -> impl iced::futures::Stream<Item = CoordinatorMessage> {
    let Some(settings) = COORDINATOR_SETTINGS.get().cloned() else {
        eprintln!("Error: Coordinator settings were not initialized before subscribing. This is a programming error.");
        std::process::exit(1);
    };
    iced::stream::channel(
        100,
        move |mut app_sender: iced::futures::channel::mpsc::Sender<CoordinatorMessage>| async move {
            let mut state = match settings {
                CoordinatorSettings::Server(sett) => CoordinatorState::Init(sett.clone()),
                CoordinatorSettings::ClientOnly(_) => CoordinatorState::Discovery,
            };

            let mut running = false;
            loop {
                match state {
                    CoordinatorState::Init(settings) => match start_server(settings) {
                        Ok(()) => state = CoordinatorState::Discovery,
                        Err(e) => {
                            let _ = app_sender
                                .send(CoordinatorMessage::Disconnected(format!(
                                    "Could not start coordinator server: {e}"
                                )))
                                .await;
                            break;
                        }
                    },

                    CoordinatorState::Discovery => {
                        match discover_service(COORDINATOR_SERVICE_NAME) {
                            Ok(address) => state = CoordinatorState::Discovered(address),
                            Err(e) => {
                                let _ = app_sender
                                    .send(CoordinatorMessage::Disconnected(format!(
                                        "Could not discover coordinator: {e}"
                                    )))
                                    .await;
                                break;
                            }
                        }
                    }

                    CoordinatorState::Discovered(address) => {
                        match ClientConnection::new(&address) {
                            Ok(connection) => {
                                let (app_side_sender, app_receiver) =
                                    tokio::sync::mpsc::channel(100);

                                if app_sender
                                    .send(CoordinatorMessage::Connected(app_side_sender))
                                    .await
                                    .is_err()
                                {
                                    error!("Could not send Connected message to app");
                                    break;
                                }

                                state = CoordinatorState::Connected(
                                    app_receiver,
                                    Arc::new(Mutex::new(connection)),
                                );
                            }
                            Err(e) => {
                                let _ = app_sender
                                    .send(CoordinatorMessage::Disconnected(format!(
                                        "Could not connect to coordinator: {e}"
                                    )))
                                    .await;
                                break;
                            }
                        }
                    }

                    CoordinatorState::Connected(ref mut app_receiver, ref connection) => {
                        let result = if running {
                            handle_running(connection, &mut app_sender, app_receiver, &mut running)
                                .await
                        } else {
                            handle_idle(connection, &mut app_sender, app_receiver, &mut running)
                                .await
                        };
                        if result.is_err() {
                            break;
                        }
                    }
                }
            }
        },
    )
}

fn lock_and_receive(
    connection: &Arc<Mutex<ClientConnection>>,
) -> std::result::Result<CoordinatorMessage, String> {
    connection
        .lock()
        .map_err(|e| format!("Lock error: {e}"))
        .and_then(|conn| conn.receive().map_err(|e| format!("{e}")))
}

fn lock_and_send(
    connection: &Arc<Mutex<ClientConnection>>,
    msg: ClientMessage,
) -> std::result::Result<(), String> {
    connection
        .lock()
        .map_err(|e| format!("Lock error: {e}"))
        .and_then(|conn| conn.send(msg).map_err(|e| format!("{e}")))
}

async fn handle_running(
    connection: &Arc<Mutex<ClientConnection>>,
    app_sender: &mut iced::futures::channel::mpsc::Sender<CoordinatorMessage>,
    app_receiver: &mut Receiver<ClientMessage>,
    running: &mut bool,
) -> std::result::Result<(), ()> {
    let coordinator_message = lock_and_receive(connection).map_err(|e| {
        let _ = app_sender.try_send(CoordinatorMessage::Disconnected(format!(
            "Lost connection to coordinator: {e}"
        )));
    })?;

    let is_flow_end = matches!(&coordinator_message, &CoordinatorMessage::FlowEnd(_));

    if app_sender.send(coordinator_message).await.is_err() {
        error!("Could not forward coordinator message to app");
        return Err(());
    }

    if is_flow_end {
        *running = false;
        return Ok(());
    }

    if let Some(client_message) = app_receiver.recv().await {
        lock_and_send(connection, client_message).map_err(|e| {
            let _ = app_sender.try_send(CoordinatorMessage::Disconnected(format!(
                "Lost connection to coordinator: {e}"
            )));
        })
    } else {
        error!("App channel closed while running");
        Err(())
    }
}

async fn handle_idle(
    connection: &Arc<Mutex<ClientConnection>>,
    app_sender: &mut iced::futures::channel::mpsc::Sender<CoordinatorMessage>,
    app_receiver: &mut Receiver<ClientMessage>,
    running: &mut bool,
) -> std::result::Result<(), ()> {
    if let Some(client_message) = app_receiver.recv().await {
        lock_and_send(connection, client_message).map_err(|e| {
            let _ = app_sender.try_send(CoordinatorMessage::Disconnected(format!(
                "Could not submit flow: {e}"
            )));
        })?;
        *running = true;
    }
    Ok(())
}

// Start a coordinator server in a background thread, then discover it and return the address
fn start_server(coordinator_settings: ServerSettings) -> Result<()> {
    let runtime_port = pick_unused_port().chain_err(|| "No ports free")?;
    let coordinator_connection =
        CoordinatorConnection::new(COORDINATOR_SERVICE_NAME, runtime_port)?;

    let _mdns_coordinator = enable_service_discovery(COORDINATOR_SERVICE_NAME, runtime_port)?;

    let debug_port = pick_unused_port().chain_err(|| "No ports free")?;
    let debug_connection = CoordinatorConnection::new(DEBUG_SERVICE_NAME, debug_port)?;
    let _mdns_debug = enable_service_discovery(DEBUG_SERVICE_NAME, debug_port)?;

    info!("Starting coordinator in background thread");
    thread::spawn(move || {
        if let Err(e) = coordinator(
            coordinator_settings,
            coordinator_connection,
            debug_connection,
            true,
        ) {
            error!("Coordinator thread exited with error: {e}");
        }
    });

    Ok(())
}

fn coordinator(
    coordinator_settings: ServerSettings,
    coordinator_connection: CoordinatorConnection,
    debug_connection: CoordinatorConnection,
    loop_forever: bool,
) -> Result<()> {
    let connection = Arc::new(Mutex::new(coordinator_connection));

    let mut debug_server = CliDebugHandler {
        debug_server_connection: debug_connection,
    };

    let provider = Arc::new(MetaProvider::new(
        coordinator_settings.lib_search_path,
        PathBuf::from("/"),
    )) as Arc<dyn Provider>;

    let ports = get_four_ports()?;
    trace!("Announcing three job queues and a control socket on ports: {ports:?}");
    let job_queues = get_bind_addresses(ports);
    let dispatcher = Dispatcher::new(&job_queues)?;
    let _mdns_jobs = enable_service_discovery(JOB_SERVICE_NAME, ports.0)?;
    let _mdns_results = enable_service_discovery(RESULTS_JOB_SERVICE_NAME, ports.2)?;
    let _mdns_control = enable_service_discovery(CONTROL_SERVICE_NAME, ports.3)?;

    let (job_source_name, context_job_source_name, results_sink, control_socket) =
        get_connect_addresses(ports);

    let mut executor = Executor::new();
    // if the command line options request loading native implementation of available native libs
    // if not, the native implementation is not loaded and later when a flow is loaded it's library
    // references will be resolved and those libraries (WASM implementations) will be loaded at runtime
    if coordinator_settings.native_flowstdlib {
        executor.add_lib(
            flowstdlib::manifest::get()
                .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
            Url::parse("memory://")?, // Statically linked library has no resolved Url
        )?;
    }
    executor.start(
        &provider,
        coordinator_settings.num_threads,
        &job_source_name,
        &results_sink,
        &control_socket,
    );

    let mut context_executor = Executor::new();
    context_executor.add_lib(
        context::get_manifest(connection.clone())?,
        Url::parse("memory://")?, // Statically linked library has no resolved Url
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
    Ok((
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
        pick_unused_port().chain_err(|| "No ports free")?,
    ))
}
