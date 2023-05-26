use std::path::PathBuf;
use std::sync::{Arc, mpsc, Mutex};
use std::thread;

use iced::{Subscription, subscription};
use log::{info, trace};
use portpicker::pick_unused_port;
use url::Url;

use flowcore::meta_provider::MetaProvider;
use flowcore::provider::Provider;
use flowrlib::coordinator::Coordinator;
use flowrlib::dispatcher::Dispatcher;
use flowrlib::executor::Executor;
use flowrlib::services::{CONTROL_SERVICE_NAME, JOB_QUEUES_DISCOVERY_PORT, JOB_SERVICE_NAME, RESULTS_JOB_SERVICE_NAME};

use crate::{CoordinatorSettings, gui, Message};
use crate::errors::*;
//use crate::gui::client_connection::ClientConnection;
use crate::gui::client_connection::{ClientConnection, discover_service};
use crate::gui::client_message::ClientMessage;
use crate::gui::coordinator_connection::{COORDINATOR_SERVICE_NAME, DEBUG_SERVICE_NAME,
                                         enable_service_discovery};
use crate::gui::coordinator_connection::CoordinatorConnection;
use crate::gui::coordinator_message::CoordinatorMessage;
//use crate::gui::debug_client::DebugClient;
use crate::gui::debug_handler::CliDebugHandler;
use crate::gui::submission_handler::CLISubmissionHandler;
use crate::Message::CoordinatorSent;

#[derive(Debug, Clone)]
pub(crate) enum CoordinatorState {
    Unconnected,
    Connected(mpsc::SyncSender<ClientMessage>),
}

// Creates an asynchronous worker that sends messages back and forth between the App and
// the Coordinator
pub fn connect(coordinator_settings: CoordinatorSettings) -> Subscription<Message> {
    struct Connect;

    // TODO maybe try discovering one and start if not...
    let coordinator_info = start(coordinator_settings);
    let coordinator_address = Arc::new(coordinator_info.0);
    println!("Coordinator started: {}", &coordinator_address);

    subscription::channel(
        std::any::TypeId::of::<Connect>(),
        100,
        move |mut app_sender| {
            let address = coordinator_address.clone();
            async move {
                let coordinator = ClientConnection::new(&address)
                    .unwrap(); // TODO

                // Create channel to get messages from the app
                let (app_side_sender, app_receiver) =
                    mpsc::sync_channel(100);

                // Send the Sender for that channel to the App in a Message,
                // for App to use to send us messages
                let _ = app_sender.try_send(Message::CoordinatorConnected(app_side_sender));
                println!("Sent CoordinatorReady to App");

                // If I don't do this - the app doesn't receive the message before panic below
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                // read the Submit message from the app to send to the coordinator
                match app_receiver.recv() {
                    Ok(client_message) => {
                        println!("App sent: {client_message}");
                        coordinator.send(client_message).unwrap(); // TODO
                    },
                    Err(e) => eprintln!("Error: '{e}'"),
                }

                loop {
                    // read the message back from the Coordinator
                    let coordinator_message: CoordinatorMessage = coordinator.receive().unwrap(); // TODO
                    println!("Coordinator sent: {coordinator_message}");
                    // Forward the message to the App
                    let _ = app_sender.try_send(CoordinatorSent(coordinator_message));

                    // read the message from the app to send to the coordinator
                    match app_receiver.recv() {
                        Ok(client_message) => {
                            println!("App sent: {client_message}");
                            coordinator.send(client_message).unwrap(); // TODO
                        },
                        Err(e) => eprintln!("Error: '{e}'"),
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