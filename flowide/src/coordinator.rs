use std::path::PathBuf;
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread;

use iced::{Subscription, subscription};
use log::{debug, error, info, trace};
use portpicker::pick_unused_port;
use tokio::task;
use url::Url;

use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;
use flowrlib::coordinator::Coordinator;
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME};

use crate::{CoordinatorSettings, gui};
use crate::errors::*;
use crate::gui::client_connection::{ClientConnection, discover_service};
use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_connection::{COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
                                         enable_service_discovery};
use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_message::CoordinatorMessage;
use crate::gui::debug_handler::CliDebugHandler;
use crate::gui::submission_handler::CLISubmissionHandler;

enum CoordinatorState {
    None,
    Disconnected(String),
    Connected(Receiver<ClientMessage>, ClientConnection),
    FlowSubmitted(Receiver<ClientMessage>, ClientConnection),
}

// Creates an asynchronous worker that sends messages back and forth between the App and
// the Coordinator
// TODO maybe try discovering one and start if not...
pub fn subscribe(coordinator_settings: CoordinatorSettings) -> Subscription<CoordinatorMessage> {
    struct Connect;

    subscription::channel(
        std::any::TypeId::of::<Connect>(),
        100,
        move |mut app_sender| {
            let settings = coordinator_settings.clone();
            async move {
                let mut state = CoordinatorState::None;

                loop {
                    match state {
                        CoordinatorState::None => {
                            state = task::spawn_blocking( || {
                                let (address, _) = start(settings.clone());
                                println!("Coordinator started (but Disconnected). Address '{}'", address);
                                CoordinatorState::Disconnected(address)
                            }).await.unwrap();
                        },

                        CoordinatorState::Disconnected(address) => {
                            let coordinator = ClientConnection::new(&address)
                                .unwrap(); // TODO

                            // Create channel to get messages from the app
                            let (app_side_sender, app_receiver) =
                                mpsc::sync_channel(100);

                            // Send the Sender to the App in a Message, for App to use to send us messages
                            let _ = app_sender.try_send(CoordinatorMessage::Connected(app_side_sender));

                            println!("Connected to Coordinator at address: {}", address);
                            state = CoordinatorState::Connected(app_receiver,
                                                                coordinator);
                        },

                        CoordinatorState::Connected(app_receiver,
                                                    coordinator) => {
                            state = task::spawn_blocking(move || {
                                // read the Submit message from the app
                                match app_receiver.recv() {
                                    Ok(client_message) => {
                                        // TODO check it's the correct message?
                                        // to send to the coordinator
                                        coordinator.send(client_message).unwrap();

                                        println!("Flow submitted to Coordinator");
                                        CoordinatorState::FlowSubmitted(app_receiver,
                                                                        coordinator)
                                    },
                                    Err(e) => {
                                        error!("Error: '{e}'");
                                        CoordinatorState::Connected(app_receiver,
                                                                    coordinator)
                                    },
                                }
                            }).await.unwrap();
                        },

                        CoordinatorState::FlowSubmitted(ref app_receiver,
                                                        ref coordinator) => {
                            // TODO Maybe handle Coordinator exiting message here and update state???
                            // TODO maybe add a state for ended and process flow ended state also?

                            println!("Waiting for message from coordinator");

                            let coordinator_message: CoordinatorMessage = task::spawn_blocking(move || {
                                // read the message back from the Coordinator
                                coordinator.receive().unwrap() // TODO
                            }).await.unwrap();

                            println!("Got message {}", coordinator_message);

                            // Forward the message to the App - TODO check it's the flow started message
                            app_sender.try_send(coordinator_message).unwrap(); // TODO

                            println!("Waiting for message from app");

                            let _ = task::spawn_blocking( move || {
                                // read the message from the app to send to the coordinator
                                match app_receiver.recv() {
                                    Ok(client_message) => {
                                        println!("FlowStarted state: message from app: {}", client_message);
                                        coordinator.send(client_message).unwrap();
                                    }
                                    Err(e) => error!("Error: '{e}'"),
                                }
                            }).await.unwrap();

                            println!("Flow started state ended");
                        }
                    }
                }
            }
        }
    )
}

fn start(coordinator_settings: CoordinatorSettings) -> (String, u16) {
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
        let _ = coordinator(
            coordinator_settings,
            coordinator_connection,
            debug_connection,
            false,
        );
    });

    let coordinator_address = discover_service(discovery_port, COORDINATOR_SERVICE_NAME)
        .unwrap(); // TODO;

    (coordinator_address, discovery_port)
}

fn coordinator(
    coordinator_settings: CoordinatorSettings,
    coordinator_connection: CoordinatorConnection,
    debug_connection: CoordinatorConnection,
    loop_forever: bool,
) -> Result<()> {
    let connection = Arc::new(Mutex::new(coordinator_connection));

    let mut debug_server = CliDebugHandler { debug_server_connection: debug_connection };

    let provider = Arc::new(MetaProvider::new(coordinator_settings.lib_search_path,
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
    if coordinator_settings.native_flowstdlib {
        executor.add_lib(
            flowstdlib::manifest::get_manifest()
                .chain_err(|| "Could not get 'native' flowstdlib manifest")?,
            Url::parse("memory://")? // Statically linked library has no resolved Url
        )?;
    }
    executor.start(provider.clone(), coordinator_settings.num_threads,
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